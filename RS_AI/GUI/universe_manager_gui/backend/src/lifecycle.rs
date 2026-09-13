use crate::{BackendError, CancellationToken, ConfigStore, ErrorKind, ManagerConfig, Progress, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
#[cfg(target_os = "linux")]
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub app_id: String,
    pub pid: u32,
    pub start_time_ticks: u64,
    pub executable: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationResult {
    pub app_id: String,
    pub operation: String,
    pub completed: bool,
}

#[derive(Debug)]
struct RunningProcess {
    identity: ProcessIdentity,
    #[cfg(target_os = "linux")]
    executable_identity: FileIdentity,
    child: Child,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct OpenedExecutable {
    file: File,
    resolved_path: PathBuf,
    identity: FileIdentity,
}

#[derive(Debug)]
pub struct LifecycleManager {
    config: ConfigStore,
    running: Mutex<HashMap<String, RunningProcess>>,
}

impl LifecycleManager {
    pub fn new(config: ConfigStore) -> Self {
        Self {
            config,
            running: Mutex::new(HashMap::new()),
        }
    }

    pub fn start(
        &self,
        config: &ManagerConfig,
        app_id: &str,
        confirmed: bool,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(Progress),
    ) -> Result<OperationResult> {
        platform_preflight()?;
        if !confirmed {
            return Err(BackendError::new(
                ErrorKind::Forbidden,
                "start requires explicit confirmation",
            ));
        }
        cancellation.check()?;
        let app = self.config.resolve_app(config, app_id)?;
        if app.is_custom.unwrap_or(false) || app.start_cmd.as_ref().is_some_and(|command| !command.trim().is_empty()) {
            return Err(BackendError::new(
                ErrorKind::Forbidden,
                "custom or shell command launchers are not executable by this backend",
            ));
        }
        let executable = self.config.resolve_executable(config, app)?;
        let opened = open_executable(&executable, &self.config.managed_directory(config)?)?;
        let mut running = self.lock_running()?;
        if let Some(existing) = running.get_mut(app_id) {
            if existing
                .child
                .try_wait()
                .map_err(|error| BackendError::from_io("cannot inspect child process", &executable, error))?
                .is_none()
            {
                return Err(BackendError::new(ErrorKind::Conflict, "application is already running"));
            }
            running.remove(app_id);
        }
        progress(Progress {
            completed: 0,
            total: Some(1),
            message: "Starting application".to_string(),
        });
        let mut child = spawn_opened(&opened)?;
        if let Err(error) = cancellation.check() {
            terminate_spawned(&mut child);
            return Err(error);
        }
        let identity = match read_process_identity(app_id, child.id(), &opened) {
            Ok(identity) => identity,
            Err(error) => {
                terminate_spawned(&mut child);
                return Err(error);
            }
        };
        running.insert(
            app_id.to_string(),
            RunningProcess {
                identity,
                #[cfg(target_os = "linux")]
                executable_identity: opened.identity,
                child,
            },
        );
        progress(Progress {
            completed: 1,
            total: Some(1),
            message: "Application started".to_string(),
        });
        Ok(OperationResult {
            app_id: app_id.to_string(),
            operation: "start".to_string(),
            completed: true,
        })
    }

    pub fn stop(
        &self,
        config: &ManagerConfig,
        app_id: &str,
        confirmed: bool,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(Progress),
    ) -> Result<OperationResult> {
        platform_preflight()?;
        if !confirmed {
            return Err(BackendError::new(
                ErrorKind::Forbidden,
                "stop requires explicit confirmation",
            ));
        }
        cancellation.check()?;
        let app = self.config.resolve_app(config, app_id)?;
        let expected_executable = self.config.resolve_executable(config, app)?;
        let mut running = self.lock_running()?;
        let process = running
            .get_mut(app_id)
            .ok_or_else(|| BackendError::new(ErrorKind::NotFound, "no tracked process exists for application"))?;
        verify_process_identity(
            &process.identity,
            #[cfg(target_os = "linux")]
            process.executable_identity,
            &expected_executable,
        )?;
        cancellation.check()?;
        progress(Progress {
            completed: 0,
            total: Some(1),
            message: "Stopping application".to_string(),
        });
        process
            .child
            .kill()
            .map_err(|error| BackendError::from_io("cannot stop application", &expected_executable, error))?;
        process
            .child
            .wait()
            .map_err(|error| BackendError::from_io("cannot reap application process", &expected_executable, error))?;
        running.remove(app_id);
        progress(Progress {
            completed: 1,
            total: Some(1),
            message: "Application stopped".to_string(),
        });
        Ok(OperationResult {
            app_id: app_id.to_string(),
            operation: "stop".to_string(),
            completed: true,
        })
    }

    pub fn identity(&self, app_id: &str) -> Result<Option<ProcessIdentity>> {
        Ok(self.lock_running()?.get(app_id).map(|process| process.identity.clone()))
    }

    fn lock_running(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, RunningProcess>>> {
        self.running
            .lock()
            .map_err(|_| BackendError::new(ErrorKind::Internal, "process registry lock is poisoned"))
    }
}

#[cfg(target_os = "linux")]
fn platform_preflight() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn platform_preflight() -> Result<()> {
    Err(BackendError::new(
        ErrorKind::Unavailable,
        "safe process lifecycle is currently available only on Linux",
    ))
}

#[cfg(target_os = "linux")]
fn open_executable(path: &Path, managed: &Path) -> Result<OpenedExecutable> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let path_bytes = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| BackendError::at_path(ErrorKind::InvalidArgument, "executable path contains NUL", path))?;
    // SAFETY: `path_bytes` is NUL-terminated and the returned owned descriptor is
    // immediately wrapped in `File`. O_NOFOLLOW rejects a swapped final symlink.
    let fd = unsafe { libc::open(path_bytes.as_ptr(), libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ELOOP) {
            return Err(BackendError::at_path(
                ErrorKind::Forbidden,
                "application executable must not be a symbolic link",
                path,
            ));
        }
        return Err(BackendError::from_io(
            "cannot securely open application executable",
            path,
            error,
        ));
    }
    // SAFETY: `fd` was returned by open above and ownership is transferred once.
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(|error| BackendError::from_io("cannot inspect opened executable", path, error))?;
    if !metadata.is_file() {
        return Err(BackendError::at_path(
            ErrorKind::Forbidden,
            "application executable is not a regular file",
            path,
        ));
    }
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(BackendError::at_path(
            ErrorKind::Forbidden,
            "application file is not executable",
            path,
        ));
    }
    let proc_fd = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    let resolved_path = fs::canonicalize(&proc_fd)
        .map_err(|error| BackendError::from_io("cannot resolve opened executable", path, error))?;
    if !resolved_path.starts_with(managed) {
        return Err(BackendError::at_path(
            ErrorKind::Forbidden,
            "opened executable resolves outside managed_dir",
            resolved_path,
        ));
    }
    let identity = FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    };
    Ok(OpenedExecutable {
        file,
        resolved_path,
        identity,
    })
}

#[cfg(not(target_os = "linux"))]
fn open_executable(_path: &Path, _managed: &Path) -> Result<()> {
    platform_preflight()
}

#[cfg(target_os = "linux")]
fn spawn_opened(executable: &OpenedExecutable) -> Result<Child> {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;

    let fd = executable.file.as_raw_fd();
    // The descriptor must survive Command's exec so /proc/self/fd/N remains a
    // stable reference to the already validated file, including for scripts.
    // SAFETY: fcntl only updates flags on our valid, owned descriptor.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } == -1 {
        return Err(BackendError::from_io(
            "cannot prepare opened executable",
            &executable.resolved_path,
            std::io::Error::last_os_error(),
        ));
    }
    let mut command = Command::new(format!("/proc/self/fd/{fd}"));
    command.arg0(
        executable
            .resolved_path
            .file_name()
            .unwrap_or(executable.resolved_path.as_os_str()),
    );
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| BackendError::from_io("cannot start application", &executable.resolved_path, error))
}

#[cfg(not(target_os = "linux"))]
fn spawn_opened(_executable: &()) -> Result<Child> {
    platform_preflight()?;
    Err(BackendError::new(ErrorKind::Unavailable, "process spawn unavailable"))
}

#[cfg(target_os = "linux")]
fn read_process_identity(app_id: &str, pid: u32, expected: &OpenedExecutable) -> Result<ProcessIdentity> {
    use std::os::unix::fs::MetadataExt;

    let proc_root = PathBuf::from(format!("/proc/{pid}"));
    let proc_exe = proc_root.join("exe");
    let metadata = fs::metadata(&proc_exe).map_err(|error| {
        BackendError::from_io(
            "cannot identify spawned process executable",
            &expected.resolved_path,
            error,
        )
    })?;
    let actual = FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    };
    if actual != expected.identity {
        return Err(BackendError::new(
            ErrorKind::Forbidden,
            "spawned process executable identity does not match application",
        ));
    }
    Ok(ProcessIdentity {
        app_id: app_id.to_string(),
        pid,
        start_time_ticks: read_start_time(&proc_root.join("stat"))?,
        executable: expected.resolved_path.clone(),
    })
}

#[cfg(not(target_os = "linux"))]
fn read_process_identity(_app_id: &str, _pid: u32, _expected_executable: &()) -> Result<ProcessIdentity> {
    platform_preflight()?;
    Err(BackendError::new(
        ErrorKind::Unavailable,
        "process identity unavailable",
    ))
}

#[cfg(target_os = "linux")]
fn verify_process_identity(
    identity: &ProcessIdentity,
    expected_identity: FileIdentity,
    expected_executable: &Path,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    let proc_root = PathBuf::from(format!("/proc/{}", identity.pid));
    let metadata = fs::metadata(proc_root.join("exe"))
        .map_err(|error| BackendError::from_io("tracked process no longer exists", expected_executable, error))?;
    let start_time = read_start_time(&proc_root.join("stat"))?;
    let actual = FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    };
    if actual != expected_identity
        || identity.executable != expected_executable
        || start_time != identity.start_time_ticks
    {
        return Err(BackendError::new(
            ErrorKind::Forbidden,
            "tracked PID was reused or its executable identity changed",
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn verify_process_identity(_identity: &ProcessIdentity, _expected_executable: &Path) -> Result<()> {
    platform_preflight()
}

#[cfg(target_os = "linux")]
fn read_start_time(path: &Path) -> Result<u64> {
    let stat =
        fs::read_to_string(path).map_err(|error| BackendError::from_io("cannot read process identity", path, error))?;
    let after_name = stat
        .rfind(") ")
        .and_then(|index| stat.get(index + 2..))
        .ok_or_else(|| BackendError::new(ErrorKind::Internal, "invalid Linux process stat record"))?;
    after_name
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| BackendError::new(ErrorKind::Internal, "Linux process stat lacks start time"))?
        .parse::<u64>()
        .map_err(|error| BackendError::with_source(ErrorKind::Internal, "invalid Linux process start time", error))
}

fn terminate_spawned(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn forged_or_reused_process_identity_is_denied() {
        let executable = fs::canonicalize("/proc/self/exe").expect("current executable");
        let metadata = fs::metadata("/proc/self/exe").expect("current executable metadata");
        let executable_identity = FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        let proc_root = PathBuf::from(format!("/proc/{}", std::process::id()));
        let start_time = read_start_time(&proc_root.join("stat")).expect("start time");

        let forged_start = ProcessIdentity {
            app_id: "fixture".into(),
            pid: std::process::id(),
            start_time_ticks: start_time.saturating_add(1),
            executable: executable.clone(),
        };
        assert_eq!(
            verify_process_identity(&forged_start, executable_identity, &executable)
                .expect_err("PID reuse marker must fail")
                .kind(),
            ErrorKind::Forbidden
        );

        let forged_executable = ProcessIdentity {
            start_time_ticks: start_time,
            executable: PathBuf::from("/definitely/not/the/current/executable"),
            ..forged_start
        };
        assert_eq!(
            verify_process_identity(&forged_executable, executable_identity, &executable)
                .expect_err("forged executable identity must fail")
                .kind(),
            ErrorKind::Forbidden
        );
    }

    #[test]
    fn opened_executable_survives_path_swap_before_spawn() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "universe-open-swap-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test directory");
        let executable = root.join("app");
        fs::copy("/usr/bin/yes", &executable).expect("fixture executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
        let opened = open_executable(&executable, &root).expect("secure open");
        fs::rename(&executable, root.join("original")).expect("swap original");
        fs::copy("/usr/bin/true", &executable).expect("replacement executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod replacement");

        let mut child = spawn_opened(&opened).expect("spawn opened executable");
        let identity = read_process_identity("fixture", child.id(), &opened).expect("stable identity");
        assert_eq!(identity.executable, opened.resolved_path);
        terminate_spawned(&mut child);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn opened_multicall_executable_preserves_approved_launch_name() {
        use std::os::unix::fs::PermissionsExt;

        let Some(busybox) = ["/usr/bin/busybox", "/bin/busybox"]
            .into_iter()
            .map(Path::new)
            .find(|path| path.is_file())
        else {
            return;
        };
        let root = std::env::temp_dir().join(format!("universe-multicall-{}", std::process::id()));
        fs::create_dir_all(&root).expect("test directory");
        let executable = root.join("yes");
        fs::copy(busybox, &executable).expect("multicall fixture executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");

        let opened = open_executable(&executable, &root).expect("secure open");
        let mut child = spawn_opened(&opened).expect("spawn multicall executable");
        read_process_identity("fixture", child.id(), &opened).expect("stable executable identity");
        terminate_spawned(&mut child);
        let _ = fs::remove_dir_all(root);
    }
}
