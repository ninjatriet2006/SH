use crate::CancellationToken;
use crate::path_security::{CanonicalRoots, outside_roots, require_absolute};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeInstallRequest {
    pub artifact: PathBuf,
    pub target_dir: PathBuf,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUninstallRequest {
    pub installation_id: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOperation {
    Install,
    Uninstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOperationResult {
    pub operation: NativeOperation,
    pub installation_id: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProgress {
    pub completed: u64,
    pub total: u64,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallationRecord {
    pub installation_id: String,
    pub installed_path: PathBuf,
    operation_token: String,
}

#[derive(Debug, Clone)]
pub struct UserInstallRoots {
    roots: CanonicalRoots,
}

impl UserInstallRoots {
    #[cfg(target_os = "linux")]
    pub fn from_home(home: impl AsRef<Path>) -> io::Result<Self> {
        let home = canonical_directory(home.as_ref())?;
        Self::create_exact(vec![home.join(".local/bin"), home.join(".local/share/applications")])
    }

    #[cfg(target_os = "macos")]
    pub fn from_home(_home: impl AsRef<Path>) -> io::Result<Self> {
        Err(native_platform_unavailable())
    }

    #[cfg(target_os = "windows")]
    pub fn from_local_app_data(_local_app_data: impl AsRef<Path>) -> io::Result<Self> {
        Err(native_platform_unavailable())
    }

    #[cfg(target_os = "linux")]
    fn create_exact(paths: Vec<PathBuf>) -> io::Result<Self> {
        for path in &paths {
            fs::create_dir_all(path)?;
        }
        Ok(Self {
            roots: CanonicalRoots::new(paths, "user install")?,
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn resolve_target_directory(&self, path: &Path) -> io::Result<PathBuf> {
        let resolved = self.roots.resolve_existing(path)?;
        if resolved.is_dir() && self.roots.contains_exact(&resolved) {
            Ok(resolved)
        } else {
            Err(outside_roots())
        }
    }
}

pub struct NativeManager {
    artifact_roots: CanonicalRoots,
    user_roots: UserInstallRoots,
    records: HashMap<String, InstallationRecord>,
}

impl NativeManager {
    pub fn new(artifact_roots: Vec<PathBuf>, user_roots: UserInstallRoots) -> io::Result<Self> {
        Ok(Self {
            artifact_roots: CanonicalRoots::new(artifact_roots, "artifact")?,
            user_roots,
            records: HashMap::new(),
        })
    }

    pub fn with_records(
        artifact_roots: Vec<PathBuf>,
        user_roots: UserInstallRoots,
        records: Vec<InstallationRecord>,
    ) -> io::Result<Self> {
        let mut manager = Self::new(artifact_roots, user_roots)?;
        for record in records {
            validate_installation_id(&record.installation_id)?;
            validate_operation_token(&record.operation_token)?;
            #[cfg(target_os = "linux")]
            linux::validate_recorded_path(
                &record.installed_path,
                &record.operation_token,
                &manager.user_roots.roots,
            )?;
            #[cfg(not(target_os = "linux"))]
            let path = manager.user_roots.roots.resolve_existing(&record.installed_path)?;
            #[cfg(not(target_os = "linux"))]
            if path != record.installed_path {
                return Err(outside_roots());
            }
            if manager.records.insert(record.installation_id.clone(), record).is_some() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "duplicate installation id"));
            }
        }
        Ok(manager)
    }

    pub fn with_records_for_uninstall(
        user_roots: UserInstallRoots,
        records: Vec<InstallationRecord>,
    ) -> io::Result<Self> {
        let artifact_roots = user_roots.roots.clone();
        let mut manager = Self {
            artifact_roots,
            user_roots,
            records: HashMap::new(),
        };
        for record in records {
            validate_installation_id(&record.installation_id)?;
            validate_operation_token(&record.operation_token)?;
            #[cfg(target_os = "linux")]
            linux::validate_recorded_path(
                &record.installed_path,
                &record.operation_token,
                &manager.user_roots.roots,
            )?;
            #[cfg(not(target_os = "linux"))]
            manager.user_roots.roots.resolve_existing(&record.installed_path)?;
            if manager.records.insert(record.installation_id.clone(), record).is_some() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "duplicate installation id"));
            }
        }
        Ok(manager)
    }

    pub fn records(&self) -> Vec<InstallationRecord> {
        let mut records: Vec<_> = self.records.values().cloned().collect();
        records.sort_by(|left, right| left.installation_id.cmp(&right.installation_id));
        records
    }

    pub fn merge_records(&mut self, records: Vec<InstallationRecord>) -> io::Result<()> {
        for record in records {
            validate_installation_id(&record.installation_id)?;
            validate_operation_token(&record.operation_token)?;
            #[cfg(target_os = "linux")]
            linux::validate_recorded_path(&record.installed_path, &record.operation_token, &self.user_roots.roots)?;
            #[cfg(not(target_os = "linux"))]
            let path = self.user_roots.roots.resolve_existing(&record.installed_path)?;
            #[cfg(not(target_os = "linux"))]
            if path != record.installed_path {
                return Err(outside_roots());
            }
            match self.records.get(&record.installation_id) {
                Some(existing) if existing == &record => {}
                Some(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "installation id has conflicting records",
                    ));
                }
                None => {
                    self.records.insert(record.installation_id.clone(), record);
                }
            }
        }
        Ok(())
    }

    pub fn installation_record(&self, installation_id: &str) -> Option<InstallationRecord> {
        self.records.get(installation_id).cloned()
    }

    pub fn user_install_roots(&self) -> UserInstallRoots {
        self.user_roots.clone()
    }

    pub fn reconcile_pending_uninstall(
        user_roots: UserInstallRoots,
        record: &InstallationRecord,
        cancellation: &CancellationToken,
    ) -> io::Result<bool> {
        validate_installation_id(&record.installation_id)?;
        validate_operation_token(&record.operation_token)?;
        #[cfg(target_os = "linux")]
        {
            linux::reconcile_pending_uninstall(
                &record.installed_path,
                &record.operation_token,
                &user_roots.roots,
                cancellation,
            )
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (user_roots, cancellation);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "native uninstall reconciliation is unavailable on this platform",
            ))
        }
    }

    pub fn abandon_pending_install(&self, record: &InstallationRecord) -> io::Result<()> {
        #[cfg(target_os = "linux")]
        {
            linux::abandon_pending_install(&record.installed_path, &record.operation_token, &self.user_roots.roots)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = record;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "native install reconciliation is unavailable on this platform",
            ))
        }
    }

    pub fn install(
        &mut self,
        request: &NativeInstallRequest,
        cancellation: &CancellationToken,
        progress: impl FnMut(NativeProgress),
    ) -> io::Result<NativeOperationResult> {
        self.install_with_commit(request, cancellation, progress, |_| Ok(()))
    }

    pub fn install_with_commit(
        &mut self,
        request: &NativeInstallRequest,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(NativeProgress),
        mut commit_record: impl FnMut(&InstallationRecord) -> io::Result<()>,
    ) -> io::Result<NativeOperationResult> {
        let (artifact, destination) = self.preflight_install(request, cancellation)?;
        #[cfg(target_os = "linux")]
        let total = linux::count_artifact(&artifact, &self.artifact_roots, cancellation)?;
        #[cfg(not(target_os = "linux"))]
        let total = count_files(&artifact, &self.artifact_roots)?;
        progress(NativeProgress {
            completed: 0,
            total,
            path: Some(artifact.clone()),
        });
        let installation_id = format!("native-{}", random_token()?);
        let operation_token = random_token()?;
        let record = InstallationRecord {
            installation_id: installation_id.clone(),
            installed_path: destination.clone(),
            operation_token: operation_token.clone(),
        };
        let installed_path = perform_install(
            &artifact,
            &destination,
            &operation_token,
            &self.artifact_roots,
            &self.user_roots.roots,
            cancellation,
            total,
            &mut progress,
            || commit_record(&record),
        )?;
        self.records.insert(
            installation_id.clone(),
            InstallationRecord {
                installation_id: installation_id.clone(),
                installed_path,
                operation_token,
            },
        );
        Ok(NativeOperationResult {
            operation: NativeOperation::Install,
            installation_id,
            completed: true,
        })
    }

    /// Checks confirmation, paths, containment and destination conflicts without
    /// copying an artifact or changing installation records.
    pub fn preflight_install(
        &self,
        request: &NativeInstallRequest,
        cancellation: &CancellationToken,
    ) -> io::Result<(PathBuf, PathBuf)> {
        if !cfg!(target_os = "linux") {
            return Err(native_platform_unavailable());
        }

        require_confirmation(request.confirmed)?;
        require_absolute(&request.artifact)?;
        require_absolute(&request.target_dir)?;
        cancellation.check()?;
        #[cfg(target_os = "linux")]
        let artifact = request.artifact.clone();
        #[cfg(not(target_os = "linux"))]
        let artifact = self.artifact_roots.resolve_existing(&request.artifact)?;
        #[cfg(target_os = "linux")]
        let target_dir = linux::validate_target_directory(&request.target_dir, &self.user_roots.roots)?;
        #[cfg(not(target_os = "linux"))]
        let target_dir = self.user_roots.resolve_target_directory(&request.target_dir)?;
        let name = artifact
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "artifact must have a file name"))?;
        let destination = target_dir.join(name);
        #[cfg(target_os = "linux")]
        linux::preflight(
            &artifact,
            &destination,
            &self.artifact_roots,
            &self.user_roots.roots,
            cancellation,
        )?;
        #[cfg(not(target_os = "linux"))]
        {
            reject_symlink(&request.artifact)?;
            if fs::symlink_metadata(&destination).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "installation target already exists",
                ));
            }
            count_files(&artifact, &self.artifact_roots)?;
        }
        Ok((artifact, destination))
    }

    pub fn uninstall(
        &mut self,
        request: &NativeUninstallRequest,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(NativeProgress),
    ) -> io::Result<NativeOperationResult> {
        if !cfg!(target_os = "linux") {
            return Err(native_platform_unavailable());
        }

        require_confirmation(request.confirmed)?;
        validate_installation_id(&request.installation_id)?;
        cancellation.check()?;
        let record = self
            .records
            .get(&request.installation_id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "installation is not recorded"))?
            .clone();
        #[cfg(target_os = "linux")]
        let installed_path =
            linux::validate_recorded_path(&record.installed_path, &record.operation_token, &self.user_roots.roots)?;
        #[cfg(not(target_os = "linux"))]
        let installed_path = self.user_roots.roots.resolve_existing(&record.installed_path)?;
        #[cfg(not(target_os = "linux"))]
        if installed_path != record.installed_path {
            return Err(outside_roots());
        }
        progress(NativeProgress {
            completed: 0,
            total: 1,
            path: Some(installed_path.clone()),
        });
        cancellation.check()?;
        perform_uninstall(
            &installed_path,
            &record.operation_token,
            &self.user_roots.roots,
            cancellation,
        )?;
        #[cfg(test)]
        linux::record_durability_event_for_test("record-remove");
        self.records.remove(&request.installation_id);
        progress(NativeProgress {
            completed: 1,
            total: 1,
            path: Some(installed_path),
        });
        Ok(NativeOperationResult {
            operation: NativeOperation::Uninstall,
            installation_id: request.installation_id.clone(),
            completed: true,
        })
    }
}

#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
fn perform_install(
    artifact: &Path,
    destination: &Path,
    operation_token: &str,
    artifact_roots: &CanonicalRoots,
    user_roots: &CanonicalRoots,
    cancellation: &CancellationToken,
    total: u64,
    progress: &mut impl FnMut(NativeProgress),
    before_publish: impl FnOnce() -> io::Result<()>,
) -> io::Result<PathBuf> {
    linux::install_artifact(
        artifact,
        destination,
        operation_token,
        artifact_roots,
        user_roots,
        cancellation,
        total,
        progress,
        before_publish,
    )?;
    Ok(destination.to_path_buf())
}

#[cfg(not(target_os = "linux"))]
#[allow(clippy::too_many_arguments)]
fn perform_install(
    _artifact: &Path,
    _destination: &Path,
    _operation_token: &str,
    _artifact_roots: &CanonicalRoots,
    _user_roots: &CanonicalRoots,
    _cancellation: &CancellationToken,
    _total: u64,
    _progress: &mut impl FnMut(NativeProgress),
    _before_publish: impl FnOnce() -> io::Result<()>,
) -> io::Result<PathBuf> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native filesystem mutation is unavailable on this platform",
    ))
}

#[cfg(target_os = "linux")]
fn perform_uninstall(
    installed_path: &Path,
    operation_token: &str,
    roots: &CanonicalRoots,
    cancellation: &CancellationToken,
) -> io::Result<()> {
    linux::uninstall_artifact(installed_path, operation_token, roots, cancellation)
}

#[cfg(not(target_os = "linux"))]
fn perform_uninstall(
    _installed_path: &Path,
    _operation_token: &str,
    _roots: &CanonicalRoots,
    _cancellation: &CancellationToken,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native filesystem mutation is unavailable on this platform",
    ))
}

#[cfg(target_os = "linux")]
fn canonical_directory(path: &Path) -> io::Result<PathBuf> {
    require_absolute(path)?;
    let path = fs::canonicalize(path)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "user-scope base must be a directory",
        ))
    }
}

fn require_confirmation(confirmed: bool) -> io::Result<()> {
    if confirmed {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "native operation requires explicit confirmation",
        ))
    }
}

fn native_platform_unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "native install and uninstall are available only on Linux",
    )
}

fn validate_installation_id(id: &str) -> io::Result<()> {
    if !id.is_empty() && id.len() <= 128 && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid installation id"))
    }
}

fn validate_operation_token(token: &str) -> io::Result<()> {
    if token.len() == 32 && token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid operation token"))
    }
}

fn random_token() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut token, "{byte:02x}").map_err(io::Error::other)?;
    }
    Ok(token)
}

#[cfg(not(target_os = "linux"))]
fn reject_symlink(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        Err(outside_roots())
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
fn count_files(path: &Path, roots: &CanonicalRoots) -> io::Result<u64> {
    reject_symlink(path)?;
    let resolved = fs::canonicalize(path)?;
    if !roots.contains(&resolved) {
        return Err(outside_roots());
    }
    if resolved.is_file() {
        return Ok(1);
    }
    let mut total = 0;
    for entry in fs::read_dir(resolved)? {
        total += count_files(&entry?.path(), roots)?;
    }
    Ok(total)
}

#[cfg(not(target_os = "linux"))]
fn copy_artifact(
    source: &Path,
    destination: &Path,
    roots: &CanonicalRoots,
    cancellation: &CancellationToken,
    total: u64,
    completed: &mut u64,
    progress: &mut impl FnMut(NativeProgress),
) -> io::Result<()> {
    cancellation.check()?;
    reject_symlink(source)?;
    let resolved = fs::canonicalize(source)?;
    if !roots.contains(&resolved) {
        return Err(outside_roots());
    }
    if resolved.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(&resolved)? {
            let entry = entry?;
            copy_artifact(
                &entry.path(),
                &destination.join(entry.file_name()),
                roots,
                cancellation,
                total,
                completed,
                progress,
            )?;
        }
    } else if resolved.is_file() {
        fs::copy(&resolved, destination)?;
        *completed += 1;
        progress(NativeProgress {
            completed: *completed,
            total,
            path: Some(resolved),
        });
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "artifact must contain only regular files and directories",
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn remove_installed_path(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(1);

    struct TestTree {
        root: PathBuf,
        home: PathBuf,
        artifacts: PathBuf,
    }

    impl TestTree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "universal-native-{name}-{}-{}",
                std::process::id(),
                TEST_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let home = root.join("home");
            let artifacts = root.join("artifacts");
            fs::create_dir_all(&home).expect("home");
            fs::create_dir_all(&artifacts).expect("artifacts");
            fs::write(artifacts.join("converter"), b"binary").expect("artifact");
            Self { root, home, artifacts }
        }

        fn manager(&self) -> NativeManager {
            NativeManager::new(
                vec![self.artifacts.clone()],
                UserInstallRoots::from_home(&self.home).expect("user roots"),
            )
            .expect("manager")
        }

        fn request(&self, confirmed: bool) -> NativeInstallRequest {
            NativeInstallRequest {
                artifact: self.artifacts.join("converter"),
                target_dir: self.home.join(".local/bin"),
                confirmed,
            }
        }
    }

    impl Drop for TestTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn install_and_uninstall_use_confirmed_recorded_user_scope() {
        let tree = TestTree::new("lifecycle");
        let mut manager = tree.manager();
        let mut progress = Vec::new();
        let installed = manager
            .install(&tree.request(true), &CancellationToken::new(), |event| {
                progress.push(event)
            })
            .expect("install");
        assert!(tree.home.join(".local/bin/converter").is_file());
        assert_eq!(manager.records().len(), 1);
        assert_eq!(progress.last().map(|event| event.completed), Some(1));

        manager
            .uninstall(
                &NativeUninstallRequest {
                    installation_id: installed.installation_id,
                    confirmed: true,
                },
                &CancellationToken::new(),
                |_| {},
            )
            .expect("uninstall");
        assert!(!tree.home.join(".local/bin/converter").exists());
        assert!(manager.records().is_empty());
    }

    #[test]
    fn native_operations_require_confirmation_and_reject_outside_target() {
        let tree = TestTree::new("policy");
        let mut manager = tree.manager();
        assert_eq!(
            manager
                .install(&tree.request(false), &CancellationToken::new(), |_| {})
                .expect_err("confirmation required")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        let mut outside = tree.request(true);
        outside.target_dir = tree.home.clone();
        assert_eq!(
            manager
                .install(&outside, &CancellationToken::new(), |_| {})
                .expect_err("outside target denied")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn failed_install_preserves_existing_records() {
        let tree = TestTree::new("preserve-records");
        let mut manager = tree.manager();
        let installed = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("initial install");
        let records = manager.records();

        let error = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect_err("existing destination conflicts");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(manager.records(), records);
        assert_eq!(manager.records()[0].installation_id, installed.installation_id);
    }

    #[test]
    fn mid_copy_cancellation_preserves_records_and_removes_partial_artifact() {
        let tree = TestTree::new("cancel-partial");
        let bundle = tree.artifacts.join("bundle");
        fs::create_dir(&bundle).expect("bundle");
        fs::write(bundle.join("first"), b"first").expect("first file");
        fs::write(bundle.join("second"), b"second").expect("second file");
        let mut manager = tree.manager();
        manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("prior install");
        let prior_records = manager.records();
        let cancellation = CancellationToken::new();
        let cancellation_from_progress = cancellation.clone();
        let request = NativeInstallRequest {
            artifact: bundle,
            target_dir: tree.home.join(".local/bin"),
            confirmed: true,
        };

        let error = manager
            .install(&request, &cancellation, |event| {
                if event.completed == 1 {
                    cancellation_from_progress.cancel();
                }
            })
            .expect_err("copy must stop after its first file");

        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(manager.records(), prior_records);
        assert!(tree.home.join(".local/bin/converter").is_file());
        assert!(!tree.home.join(".local/bin/bundle").exists());
    }

    #[cfg(unix)]
    #[test]
    fn native_install_rejects_artifact_symlink_escape() {
        use std::os::unix::fs::symlink;

        let tree = TestTree::new("symlink");
        let outside = tree.root.join("outside");
        fs::write(&outside, b"outside").expect("outside");
        let link = tree.artifacts.join("linked");
        symlink(outside, &link).expect("symlink");
        let mut manager = tree.manager();
        let request = NativeInstallRequest {
            artifact: link,
            target_dir: tree.home.join(".local/bin"),
            confirmed: true,
        };
        assert_eq!(
            manager
                .install(&request, &CancellationToken::new(), |_| {})
                .expect_err("symlink denied")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn native_install_rejects_nested_symlink_and_special_file() {
        use rustix::fs::{CWD, Mode, mkfifoat};
        use std::os::unix::fs::symlink;

        let tree = TestTree::new("nested-invalid");
        let bundle = tree.artifacts.join("bundle");
        fs::create_dir(&bundle).expect("bundle");
        let outside = tree.root.join("outside");
        fs::write(&outside, b"outside").expect("outside");
        symlink(&outside, bundle.join("link")).expect("nested symlink");
        let mut manager = tree.manager();
        let request = NativeInstallRequest {
            artifact: bundle.clone(),
            target_dir: tree.home.join(".local/bin"),
            confirmed: true,
        };
        assert!(manager.install(&request, &CancellationToken::new(), |_| {}).is_err());
        fs::remove_file(bundle.join("link")).expect("remove test link");
        mkfifoat(CWD, bundle.join("pipe"), Mode::RUSR | Mode::WUSR).expect("fifo");
        assert_eq!(
            manager
                .install(&request, &CancellationToken::new(), |_| {})
                .expect_err("special file denied")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!tree.home.join(".local/bin/bundle").exists());
    }

    #[test]
    fn publish_conflict_never_overwrites_existing_target() {
        let tree = TestTree::new("publish-conflict");
        let mut manager = tree.manager();
        let destination = tree.home.join(".local/bin/converter");
        fs::write(&destination, b"existing").expect("existing target");
        let error = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect_err("conflict");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(destination).expect("existing target retained"), b"existing");
        assert!(manager.records().is_empty());
    }

    #[test]
    fn uninstall_failure_retains_record() {
        let tree = TestTree::new("uninstall-failure");
        let mut manager = tree.manager();
        let installed = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("install");
        let record = manager.records()[0].clone();
        fs::write(
            tree.home
                .join(".local/bin")
                .join(format!(".universal-converter-uninstall-{}", record.operation_token)),
            b"conflict",
        )
        .expect("quarantine conflict");
        assert!(
            manager
                .uninstall(
                    &NativeUninstallRequest {
                        installation_id: installed.installation_id.clone(),
                        confirmed: true,
                    },
                    &CancellationToken::new(),
                    |_| {},
                )
                .is_err()
        );
        assert_eq!(manager.records()[0].installation_id, installed.installation_id);
        assert!(tree.home.join(".local/bin/converter").is_file());
    }

    #[test]
    fn cancellation_after_quarantine_restores_install() {
        let tree = TestTree::new("uninstall-quarantine-cancel");
        let mut manager = tree.manager();
        let _installed = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("install");
        let error = linux::quarantine_then_cancel_for_test(
            &manager.records()[0].installed_path,
            &manager.records()[0].operation_token,
            &manager.user_roots.roots,
        )
        .expect_err("cancel after quarantine");
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(tree.home.join(".local/bin/converter").is_file());
        assert!(
            !tree
                .home
                .join(".local/bin")
                .join(format!(
                    ".universal-converter-uninstall-{}",
                    manager.records()[0].operation_token
                ))
                .exists()
        );
    }

    #[test]
    fn cancellation_restore_collision_is_reported_and_quarantine_is_recoverable() {
        let tree = TestTree::new("uninstall-restore-collision");
        let mut manager = tree.manager();
        manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("install");
        let record = manager.records()[0].clone();
        let error = linux::quarantine_cancel_with_restore_collision_for_test(
            &record.installed_path,
            &record.operation_token,
            &manager.user_roots.roots,
        )
        .expect_err("restore collision must be propagated");
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(error.to_string().contains("quarantine retained for retry"));
        assert!(record.installed_path.is_file(), "colliding destination is preserved");
        assert!(
            record
                .installed_path
                .with_file_name(format!(".universal-converter-uninstall-{}", record.operation_token))
                .exists(),
            "owned quarantine remains recoverable"
        );
        assert_eq!(manager.records(), vec![record]);
    }

    #[test]
    fn source_replacement_after_preflight_is_rejected() {
        use std::os::unix::fs::symlink;

        let tree = TestTree::new("source-race");
        let mut manager = tree.manager();
        manager
            .preflight_install(&tree.request(true), &CancellationToken::new())
            .expect("preflight");
        fs::remove_file(tree.artifacts.join("converter")).expect("replace source");
        let outside = tree.root.join("outside-race");
        fs::write(&outside, b"outside").expect("outside");
        symlink(outside, tree.artifacts.join("converter")).expect("replacement symlink");
        assert!(
            manager
                .install(&tree.request(true), &CancellationToken::new(), |_| {})
                .is_err()
        );
        assert!(!tree.home.join(".local/bin/converter").exists());
    }

    #[test]
    fn random_operation_tokens_are_unique_and_not_process_counters() {
        let first = random_token().expect("first token");
        let second = random_token().expect("second token");
        assert_eq!(first.len(), 32);
        assert_ne!(first, second);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn committed_record_can_resume_publish_after_crash() {
        let tree = TestTree::new("resume-publish");
        let mut manager = tree.manager();
        let request = tree.request(true);
        let mut committed = None;
        let cancellation = CancellationToken::new();
        let collision = tree.home.join(".local/bin/converter");
        manager
            .install_with_commit(
                &request,
                &cancellation,
                |_| {},
                |record| {
                    committed = Some(record.clone());
                    fs::write(&collision, b"post-commit collision").expect("collision");
                    Ok(())
                },
            )
            .expect_err("simulated post-commit publish interruption");
        let record = committed.expect("record committed before publish");
        fs::remove_file(&collision).expect("remove collision before resume");

        let reopened = NativeManager::with_records_for_uninstall(
            UserInstallRoots::from_home(&tree.home).expect("roots"),
            vec![record.clone()],
        )
        .expect("reopen resumes pending publish");
        assert!(record.installed_path.is_file());
        assert_eq!(reopened.records(), vec![record]);
    }

    #[test]
    fn unowned_quarantine_name_is_never_accepted() {
        let tree = TestTree::new("quarantine-ownership");
        let mut manager = tree.manager();
        manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("install");
        let mut record = manager.records().remove(0);
        fs::rename(
            &record.installed_path,
            record.installed_path.with_file_name("attacker-quarantine"),
        )
        .expect("move fixture");
        record.operation_token = "00000000000000000000000000000000".to_owned();
        assert!(
            NativeManager::with_records_for_uninstall(
                UserInstallRoots::from_home(&tree.home).expect("roots"),
                vec![record],
            )
            .is_err()
        );
    }

    #[test]
    fn reused_quarantine_name_with_different_inode_is_never_deleted() {
        let tree = TestTree::new("quarantine-inode-reuse");
        let mut manager = tree.manager();
        let installed = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("install");
        let record = manager.records()[0].clone();
        let quarantine = record
            .installed_path
            .with_file_name(format!(".universal-converter-uninstall-{}", record.operation_token));
        fs::rename(&record.installed_path, &quarantine).expect("quarantine published inode");
        fs::remove_file(&quarantine).expect("simulate external removal");
        fs::write(&quarantine, b"attacker replacement").expect("reused quarantine name");

        let error = manager
            .uninstall(
                &NativeUninstallRequest {
                    installation_id: installed.installation_id,
                    confirmed: true,
                },
                &CancellationToken::new(),
                |_| {},
            )
            .expect_err("different inode must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(
            fs::read(&quarantine).expect("replacement retained"),
            b"attacker replacement"
        );
        assert_eq!(manager.records(), vec![record]);
    }

    #[test]
    fn uninstall_durability_precedes_record_removal() {
        let tree = TestTree::new("uninstall-durability-order");
        let mut manager = tree.manager();
        let installed = manager
            .install(&tree.request(true), &CancellationToken::new(), |_| {})
            .expect("install");
        let _ = linux::take_durability_events_for_test();

        manager
            .uninstall(
                &NativeUninstallRequest {
                    installation_id: installed.installation_id,
                    confirmed: true,
                },
                &CancellationToken::new(),
                |_| {},
            )
            .expect("uninstall");

        let events = linux::take_durability_events_for_test();
        let quarantine = events
            .iter()
            .position(|event| *event == "uninstall-quarantine-parent")
            .expect("quarantine parent sync");
        let deletion = events
            .iter()
            .position(|event| *event == "uninstall-delete-parent")
            .expect("delete parent sync");
        let record = events
            .iter()
            .position(|event| *event == "record-remove")
            .expect("record removal");
        assert!(quarantine < deletion && deletion < record, "events: {events:?}");
    }

    #[test]
    fn install_syncs_payload_stage_and_target_parent_before_record_commit() {
        let tree = TestTree::new("install-durability-order");
        let mut manager = tree.manager();
        let _ = linux::take_durability_events_for_test();

        manager
            .install_with_commit(
                &tree.request(true),
                &CancellationToken::new(),
                |_| {},
                |_| {
                    linux::record_durability_event_for_test("record-commit");
                    Ok(())
                },
            )
            .expect("install");

        let events = linux::take_durability_events_for_test();
        let file = events
            .iter()
            .position(|event| *event == "install-file")
            .expect("file sync");
        let stage = events
            .iter()
            .position(|event| *event == "install-stage")
            .expect("stage sync");
        let target = events
            .iter()
            .position(|event| *event == "install-target-before-record")
            .expect("target parent sync");
        let record = events
            .iter()
            .position(|event| *event == "record-commit")
            .expect("record commit");
        assert!(file < stage && stage < target && target < record, "events: {events:?}");
    }

    #[test]
    fn regular_delete_swap_never_unlinks_mutable_original_name() {
        let tree = TestTree::new("delete-swap");
        let path = tree.home.join(".local/bin/swap-target");
        let roots = UserInstallRoots::from_home(&tree.home).expect("roots");
        fs::write(&path, b"owned original").expect("original");

        let error =
            linux::swap_regular_before_tombstone_for_test(&path, &roots.roots).expect_err("identity swap must fail");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(!path.exists(), "mutable original name is never unlinked directly");
        let retained_replacement = fs::read_dir(tree.home.join(".local/bin"))
            .expect("target directory")
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".universal-converter-delete-")
            })
            .expect("replacement tombstone retained after failed identity check");
        assert_eq!(
            fs::read(retained_replacement.path()).expect("replacement retained"),
            b"attacker replacement"
        );
        assert_eq!(
            fs::read(tree.home.join(".local/bin/.universal-converter-test-original")).expect("original retained"),
            b"owned original"
        );
    }
}
