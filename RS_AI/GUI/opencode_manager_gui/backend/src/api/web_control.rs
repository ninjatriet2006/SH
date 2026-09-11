//! OpenCode terminal and loopback-only web process lifecycle.
//!
//! The web child is owned here, never shared with the visible terminal process,
//! and every process argument is passed directly (there is no shell expansion).

use serde::Serialize;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const HOST: &str = "127.0.0.1";
const READINESS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WebState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WebStatus {
    pub state: WebState,
    pub url: Option<String>,
    pub error: Option<String>,
    pub has_owned_child: bool,
    pub generation: u64,
    pub revision: u64,
}

impl Default for WebStatus {
    fn default() -> Self {
        Self {
            state: WebState::Stopped,
            url: None,
            error: None,
            has_owned_child: false,
            generation: 0,
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WebControlError {
    pub code: &'static str,
    pub message: String,
}

impl WebControlError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

trait ManagedChild: Send {
    fn id(&self) -> u32;
    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>>;
    fn terminate(&mut self) -> std::io::Result<()>;
    fn kill(&mut self) -> std::io::Result<()>;
}

struct OsChild(Child);

impl ManagedChild for OsChild {
    fn id(&self) -> u32 {
        self.0.id()
    }

    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        self.0.try_wait()
    }

    fn terminate(&mut self) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            // SAFETY: `kill` does not dereference pointers; PID comes from the
            // still-owned Child handle and only SIGTERM is sent to that PID.
            let result = unsafe { libc::kill(self.0.id() as libc::pid_t, libc::SIGTERM) };
            if result == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
        #[cfg(not(unix))]
        {
            self.0.kill()
        }
    }

    fn kill(&mut self) -> std::io::Result<()> {
        self.0.kill()
    }
}

trait Platform: Send + Sync {
    fn probe_web(&self) -> Result<(), WebControlError>;
    fn probe_terminal(&self) -> Result<(), WebControlError>;
    fn spawn_web(&self, port: u16) -> Result<Box<dyn ManagedChild>, WebControlError>;
    fn http_ready(&self, url: &str) -> bool;
    fn launch_terminal(&self) -> Result<(), WebControlError>;
    fn open_url(&self, url: &str) -> Result<(), WebControlError>;
}

struct OsPlatform;

fn neutral_cwd(command: &mut Command) {
    if let Some(home) = opencode_manager::config::get_home_dir() {
        command.current_dir(home);
    }
}

fn command_output(program: &str, args: &[&str]) -> Result<std::process::Output, WebControlError> {
    let mut command = Command::new(program);
    command.args(args);
    neutral_cwd(&mut command);
    command.output().map_err(|error| {
        WebControlError::new(
            "opencode_not_found",
            format!("Không tìm thấy executable '{program}': {error}"),
        )
    })
}

fn web_help_supported(output: &std::process::Output) -> bool {
    if !output.status.success() {
        return false;
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    text.contains("opencode web") && text.contains("--hostname") && text.contains("--port")
}

fn hidden_command(program: &str) -> Command {
    let command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    }
    #[cfg(not(target_os = "windows"))]
    command
}

impl Platform for OsPlatform {
    fn probe_web(&self) -> Result<(), WebControlError> {
        let version = command_output("opencode", &["--version"])?;
        if !version.status.success() {
            return Err(WebControlError::new(
                "opencode_unavailable",
                "OpenCode không trả về version thành công.",
            ));
        }
        let help = command_output("opencode", &["web", "--help"])?;
        if !web_help_supported(&help) {
            return Err(WebControlError::new(
                "web_subcommand_unsupported",
                "Bản OpenCode hiện tại không hỗ trợ 'web --hostname --port'.",
            ));
        }
        Ok(())
    }

    fn spawn_web(&self, port: u16) -> Result<Box<dyn ManagedChild>, WebControlError> {
        let mut command = hidden_command("opencode");
        command
            .args(["web", "--hostname", HOST, "--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        neutral_cwd(&mut command);
        command
            .spawn()
            .map(|child| Box::new(OsChild(child)) as Box<dyn ManagedChild>)
            .map_err(|error| {
                WebControlError::new(
                    "web_spawn_failed",
                    format!("Không khởi động được OpenCode Web: {error}"),
                )
            })
    }

    fn probe_terminal(&self) -> Result<(), WebControlError> {
        let output = command_output("opencode", &["--version"])?;
        if output.status.success() {
            Ok(())
        } else {
            Err(WebControlError::new(
                "opencode_unavailable",
                "OpenCode không trả về version thành công.",
            ))
        }
    }

    fn http_ready(&self, url: &str) -> bool {
        let Some(port) = localhost_port(url) else {
            return false;
        };
        let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(250)) else {
            return false;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
        if stream
            .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .is_err()
        {
            return false;
        }
        let mut response = [0_u8; 64];
        let Ok(read) = stream.read(&mut response) else {
            return false;
        };
        let first = String::from_utf8_lossy(&response[..read]);
        first
            .split_whitespace()
            .nth(1)
            .and_then(|status| status.parse::<u16>().ok())
            .is_some_and(|status| (200..400).contains(&status))
    }

    fn launch_terminal(&self) -> Result<(), WebControlError> {
        launch_terminal_os("opencode")
    }

    fn open_url(&self, url: &str) -> Result<(), WebControlError> {
        #[cfg(target_os = "windows")]
        let result = Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
        #[cfg(target_os = "macos")]
        let result = Command::new("open").arg(url).spawn();
        #[cfg(all(unix, not(target_os = "macos")))]
        let result = Command::new("xdg-open").arg(url).spawn();
        result
            .map(|_| ())
            .map_err(|error| WebControlError::new("browser_unavailable", format!("Không mở được trình duyệt: {error}")))
    }
}

#[cfg(target_os = "linux")]
fn launch_terminal_os(binary: &str) -> Result<(), WebControlError> {
    let candidates: [(&str, &[&str]); 5] = [
        ("gnome-terminal", &["--", binary]),
        ("konsole", &["-e", binary]),
        ("kitty", &[binary]),
        ("alacritty", &["-e", binary]),
        ("x-terminal-emulator", &["-e", binary]),
    ];
    launch_first_terminal(&candidates)
}

#[cfg(target_os = "macos")]
fn launch_terminal_os(_binary: &str) -> Result<(), WebControlError> {
    launch_first_terminal(&[(
        "osascript",
        &[
            "-e",
            "tell application \"Terminal\" to do script \"opencode\"",
            "-e",
            "tell application \"Terminal\" to activate",
        ],
    )])
}

#[cfg(target_os = "windows")]
fn launch_terminal_os(binary: &str) -> Result<(), WebControlError> {
    launch_first_terminal(&[("wt.exe", &[binary])])
}

fn launch_first_terminal(candidates: &[(&str, &[&str])]) -> Result<(), WebControlError> {
    for (program, args) in candidates {
        let mut command = Command::new(program);
        command
            .args(*args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        neutral_cwd(&mut command);
        match command.spawn() {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(WebControlError::new(
                    "terminal_launch_failed",
                    format!("Không mở được terminal '{program}': {error}"),
                ));
            }
        }
    }
    Err(WebControlError::new(
        "terminal_emulator_not_found",
        "Không tìm thấy terminal emulator được hỗ trợ.",
    ))
}

struct OwnedProcess {
    generation: u64,
    child: Box<dyn ManagedChild>,
}

#[derive(Default)]
struct Inner {
    status: WebStatus,
    process: Option<OwnedProcess>,
}

pub struct WebService {
    inner: Arc<Mutex<Inner>>,
    lifecycle: Arc<Mutex<()>>,
    platform: Arc<dyn Platform>,
    readiness_timeout: Duration,
    poll_interval: Duration,
    #[cfg(test)]
    monitor_events: Option<std::sync::mpsc::Sender<u64>>,
}

impl Default for WebService {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            lifecycle: Arc::new(Mutex::new(())),
            platform: Arc::new(OsPlatform),
            readiness_timeout: READINESS_TIMEOUT,
            poll_interval: POLL_INTERVAL,
            #[cfg(test)]
            monitor_events: None,
        }
    }
}

impl Drop for WebService {
    fn drop(&mut self) {
        let lifecycle = Arc::clone(&self.lifecycle);
        if let Ok(_guard) = lifecycle.lock() {
            let _ = stop_inner(&self.inner, STOP_TIMEOUT);
        };
    }
}

impl WebService {
    fn lock(&self) -> Result<MutexGuard<'_, Inner>, WebControlError> {
        self.inner
            .lock()
            .map_err(|_| WebControlError::new("state_poisoned", "Trạng thái OpenCode Web không còn khả dụng."))
    }

    pub fn status(&self) -> Result<WebStatus, WebControlError> {
        let mut inner = self.lock()?;
        refresh_exit(&mut inner);
        Ok(status_snapshot(&inner))
    }

    pub fn start(&self) -> Result<WebStatus, WebControlError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| WebControlError::new("state_poisoned", "Trạng thái OpenCode Web không còn khả dụng."))?;
        {
            let mut inner = self.lock()?;
            refresh_exit(&mut inner);
            if matches!(inner.status.state, WebState::Starting | WebState::Running) {
                return Ok(status_snapshot(&inner));
            }
            if inner.status.state == WebState::Stopping {
                return Err(WebControlError::new(
                    "transition_in_progress",
                    "OpenCode Web đang dừng.",
                ));
            }
            if inner.process.is_some() {
                return Err(WebControlError::new(
                    "process_cleanup_required",
                    "Tiến trình OpenCode Web cũ chưa được dọn dẹp; hãy thử dừng lại.",
                ));
            }
        }

        self.platform.probe_web()?;
        let listener = TcpListener::bind((HOST, 0)).map_err(|error| {
            WebControlError::new(
                "loopback_bind_failed",
                format!("Không chọn được cổng loopback: {error}"),
            )
        })?;
        let port = listener
            .local_addr()
            .map_err(|error| WebControlError::new("loopback_bind_failed", error.to_string()))?
            .port();

        let (generation, url) = {
            let mut inner = self.lock()?;
            refresh_exit(&mut inner);
            inner.status.generation = inner.status.generation.saturating_add(1);
            inner.status.revision = inner.status.revision.saturating_add(1);
            inner.status.state = WebState::Starting;
            inner.status.url = None;
            inner.status.error = None;
            let generation = inner.status.generation;
            let url = format!("http://{HOST}:{port}/");
            drop(listener);
            match self.platform.spawn_web(port) {
                Ok(child) => inner.process = Some(OwnedProcess { generation, child }),
                Err(error) => {
                    transition_error(&mut inner, error.message.clone());
                    return Err(error);
                }
            }
            (generation, url)
        };

        let inner = Arc::clone(&self.inner);
        let platform = Arc::clone(&self.platform);
        let lifecycle = Arc::clone(&self.lifecycle);
        let timeout = self.readiness_timeout;
        let interval = self.poll_interval;
        #[cfg(test)]
        let monitor_events = self.monitor_events.clone();
        thread::spawn(move || {
            monitor_generation(inner, lifecycle, platform, generation, url, timeout, interval);
            #[cfg(test)]
            if let Some(events) = monitor_events {
                let _ = events.send(generation);
            }
        });
        self.status()
    }

    pub fn stop(&self) -> Result<WebStatus, WebControlError> {
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| WebControlError::new("state_poisoned", "Trạng thái OpenCode Web không còn khả dụng."))?;
        stop_inner(&self.inner, STOP_TIMEOUT)?;
        self.status()
    }

    pub fn launch_terminal(&self) -> Result<(), WebControlError> {
        self.platform.probe_terminal()?;
        self.platform.launch_terminal()
    }

    pub fn open_url(&self, url: &str) -> Result<(), WebControlError> {
        if localhost_port(url).is_none() {
            return Err(WebControlError::new(
                "invalid_localhost_url",
                "Chỉ được mở URL HTTP loopback 127.0.0.1/localhost có cổng hợp lệ.",
            ));
        }
        let status = self.status()?;
        if status.state != WebState::Running || status.url.as_deref() != Some(url) {
            return Err(WebControlError::new(
                "web_not_running",
                "OpenCode Web chưa chạy tại URL này.",
            ));
        }
        self.platform.open_url(url)
    }
}

fn status_snapshot(inner: &Inner) -> WebStatus {
    let mut status = inner.status.clone();
    status.has_owned_child = inner.process.is_some();
    status
}

fn refresh_exit(inner: &mut Inner) {
    let Some(process) = inner.process.as_mut() else {
        return;
    };
    if process.generation != inner.status.generation {
        return;
    }
    match process.child.try_wait() {
        Ok(Some(status)) if matches!(inner.status.state, WebState::Starting | WebState::Running) => {
            let code = status
                .code()
                .map_or_else(|| "signal".to_string(), |code| code.to_string());
            inner.process = None;
            transition_error(inner, format!("OpenCode Web thoát ngoài dự kiến ({code})."));
        }
        Err(error) if matches!(inner.status.state, WebState::Starting | WebState::Running) => {
            transition_error(inner, format!("Không đọc được trạng thái OpenCode Web: {error}"));
        }
        _ => {}
    }
}

fn transition_error(inner: &mut Inner, message: String) {
    inner.status.state = WebState::Error;
    inner.status.url = None;
    inner.status.error = Some(message);
    inner.status.revision = inner.status.revision.saturating_add(1);
}

fn monitor_generation(
    inner: Arc<Mutex<Inner>>,
    lifecycle: Arc<Mutex<()>>,
    platform: Arc<dyn Platform>,
    generation: u64,
    url: String,
    timeout: Duration,
    interval: Duration,
) {
    let started = Instant::now();
    loop {
        let state = {
            let Ok(mut guard) = inner.lock() else { return };
            if guard.status.generation != generation {
                return;
            }
            refresh_exit(&mut guard);
            guard.status.state
        };
        match state {
            WebState::Starting => {
                if platform.http_ready(&url) {
                    let Ok(mut guard) = inner.lock() else { return };
                    refresh_exit(&mut guard);
                    let child_is_alive = guard
                        .process
                        .as_ref()
                        .is_some_and(|process| process.generation == generation);
                    if guard.status.generation == generation
                        && guard.status.state == WebState::Starting
                        && child_is_alive
                    {
                        guard.status.state = WebState::Running;
                        guard.status.url = Some(url.clone());
                        guard.status.error = None;
                        guard.status.revision = guard.status.revision.saturating_add(1);
                    }
                } else if started.elapsed() >= timeout {
                    let Ok(_lifecycle) = lifecycle.lock() else { return };
                    cleanup_timed_out_generation(&inner, generation, timeout);
                    return;
                }
            }
            WebState::Running => {}
            _ => return,
        }
        thread::sleep(interval);
    }
}

fn cleanup_process(process: &mut OwnedProcess, timeout: Duration) -> Result<(), WebControlError> {
    let mut last_status_error = None;
    match process.child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) => last_status_error = Some(error),
    }

    process.child.terminate().map_err(|error| {
        WebControlError::new(
            "terminate_failed",
            format!(
                "Không thể yêu cầu OpenCode Web (PID {}) thoát: {error}",
                process.child.id()
            ),
        )
    })?;

    let deadline = Instant::now() + timeout;
    loop {
        match process.child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => break,
            Err(error) if Instant::now() < deadline => {
                last_status_error = Some(error);
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => {
                last_status_error = Some(error);
                break;
            }
        }
    }

    process.child.kill().map_err(|error| {
        WebControlError::new(
            "kill_failed",
            format!(
                "Không thể buộc OpenCode Web (PID {}) thoát: {error}",
                process.child.id()
            ),
        )
    })?;

    let deadline = Instant::now() + timeout;
    loop {
        match process.child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                if let Some(error) = last_status_error {
                    return Err(WebControlError::new(
                        "child_status_failed",
                        format!(
                            "Không đọc được trạng thái OpenCode Web (PID {}): {error}",
                            process.child.id()
                        ),
                    ));
                }
                return Err(WebControlError::new(
                    "stop_timeout",
                    format!(
                        "OpenCode Web (PID {}) chưa thoát sau khi bị buộc dừng.",
                        process.child.id()
                    ),
                ));
            }
            Err(error) if Instant::now() < deadline => {
                last_status_error = Some(error);
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => {
                return Err(WebControlError::new(
                    "child_status_failed",
                    format!(
                        "Không đọc được trạng thái OpenCode Web (PID {}): {error}",
                        process.child.id()
                    ),
                ));
            }
        }
    }
}

fn restore_failed_cleanup(inner: &Arc<Mutex<Inner>>, mut process: OwnedProcess, error: &WebControlError) {
    if let Ok(mut guard) = inner.lock() {
        if guard.status.generation == process.generation && guard.process.is_none() {
            guard.process = Some(process);
            transition_error(&mut guard, error.message.clone());
            return;
        }
    }

    // Ownership could not be restored (for example after mutex poisoning). Make
    // one final bounded cleanup attempt rather than silently orphaning the child.
    let _ = cleanup_process(&mut process, Duration::from_millis(100));
}

fn stop_inner(inner: &Arc<Mutex<Inner>>, timeout: Duration) -> Result<(), WebControlError> {
    let process = {
        let mut guard = inner
            .lock()
            .map_err(|_| WebControlError::new("state_poisoned", "Trạng thái OpenCode Web không còn khả dụng."))?;
        if guard.status.state == WebState::Stopped && guard.process.is_none() {
            return Ok(());
        }
        guard.status.state = WebState::Stopping;
        guard.status.url = None;
        guard.status.error = None;
        guard.status.revision = guard.status.revision.saturating_add(1);
        guard.process.take()
    };

    if let Some(mut process) = process {
        let generation = process.generation;
        if let Err(error) = cleanup_process(&mut process, timeout) {
            restore_failed_cleanup(inner, process, &error);
            return Err(error);
        }
        let mut guard = inner
            .lock()
            .map_err(|_| WebControlError::new("state_poisoned", "Trạng thái OpenCode Web không còn khả dụng."))?;
        if guard.status.generation == generation && guard.process.is_none() {
            guard.status.state = WebState::Stopped;
            guard.status.url = None;
            guard.status.error = None;
            guard.status.revision = guard.status.revision.saturating_add(1);
        }
        return Ok(());
    }

    let mut guard = inner
        .lock()
        .map_err(|_| WebControlError::new("state_poisoned", "Trạng thái OpenCode Web không còn khả dụng."))?;
    guard.status.state = WebState::Stopped;
    guard.status.url = None;
    guard.status.error = None;
    guard.status.revision = guard.status.revision.saturating_add(1);
    Ok(())
}

fn cleanup_timed_out_generation(inner: &Arc<Mutex<Inner>>, generation: u64, timeout: Duration) {
    let process = {
        let Ok(mut guard) = inner.lock() else { return };
        if guard.status.generation != generation || guard.status.state != WebState::Starting {
            return;
        }
        guard.status.state = WebState::Stopping;
        guard.status.revision = guard.status.revision.saturating_add(1);
        guard.process.take()
    };

    let Some(mut process) = process else { return };
    match cleanup_process(&mut process, timeout) {
        Ok(()) => {
            let Ok(mut guard) = inner.lock() else { return };
            if guard.status.generation == generation && guard.process.is_none() {
                transition_error(&mut guard, "OpenCode Web không sẵn sàng trước thời hạn.".to_string());
            }
        }
        Err(error) => restore_failed_cleanup(inner, process, &error),
    }
}

fn localhost_port(url: &str) -> Option<u16> {
    let authority = url.strip_prefix("http://")?.strip_suffix('/')?;
    if authority.contains(['/', '?', '#', '@']) {
        return None;
    }
    let (host, port) = authority.rsplit_once(':')?;
    if host != "127.0.0.1" && host != "localhost" {
        return None;
    }
    let port = port.parse::<u16>().ok()?;
    (port != 0).then_some(port)
}

#[tauri::command(rename_all = "snake_case")]
pub fn web_status(service: tauri::State<'_, WebService>) -> Result<WebStatus, WebControlError> {
    service.status()
}

#[tauri::command(rename_all = "snake_case")]
pub fn web_start(service: tauri::State<'_, WebService>) -> Result<WebStatus, WebControlError> {
    service.start()
}

#[tauri::command(rename_all = "snake_case")]
pub fn web_stop(service: tauri::State<'_, WebService>) -> Result<WebStatus, WebControlError> {
    service.stop()
}

#[tauri::command(rename_all = "snake_case")]
pub fn launch_terminal(service: tauri::State<'_, WebService>) -> Result<(), WebControlError> {
    service.launch_terminal()
}

#[tauri::command(rename_all = "snake_case")]
pub fn open_web_url(url: String, service: tauri::State<'_, WebService>) -> Result<(), WebControlError> {
    service.open_url(url.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Error;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Barrier;

    #[derive(Default)]
    struct FakeChildState {
        exited: AtomicBool,
        exit_on_terminate: AtomicBool,
        terminated: AtomicUsize,
        killed: AtomicUsize,
        try_wait_error: AtomicBool,
        terminate_error: AtomicBool,
        kill_error: AtomicBool,
    }

    struct FakeChild(Arc<FakeChildState>);

    impl ManagedChild for FakeChild {
        fn id(&self) -> u32 {
            4242
        }
        fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
            if self.0.try_wait_error.load(Ordering::SeqCst) {
                return Err(Error::other("fake try_wait failure"));
            }
            if self.0.exited.load(Ordering::SeqCst) {
                Ok(Some(success_status()))
            } else {
                Ok(None)
            }
        }
        fn terminate(&mut self) -> std::io::Result<()> {
            self.0.terminated.fetch_add(1, Ordering::SeqCst);
            if self.0.terminate_error.load(Ordering::SeqCst) {
                return Err(Error::other("fake terminate failure"));
            }
            if self.0.exit_on_terminate.load(Ordering::SeqCst) {
                self.0.exited.store(true, Ordering::SeqCst);
            }
            Ok(())
        }
        fn kill(&mut self) -> std::io::Result<()> {
            self.0.killed.fetch_add(1, Ordering::SeqCst);
            if self.0.kill_error.load(Ordering::SeqCst) {
                return Err(Error::other("fake kill failure"));
            }
            self.0.exited.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[cfg(unix)]
    fn success_status() -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
    #[cfg(windows)]
    fn success_status() -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }

    struct FakePlatform {
        supported: bool,
        ready: AtomicBool,
        spawn_count: AtomicUsize,
        probe_count: AtomicUsize,
        terminal_count: AtomicUsize,
        spawn_barrier: Mutex<Option<Arc<Barrier>>>,
        readiness_barriers: Mutex<Option<(Arc<Barrier>, Arc<Barrier>)>>,
        children: Mutex<Vec<Arc<FakeChildState>>>,
    }

    impl FakePlatform {
        fn new(supported: bool) -> Self {
            Self {
                supported,
                ready: AtomicBool::new(false),
                spawn_count: AtomicUsize::new(0),
                probe_count: AtomicUsize::new(0),
                terminal_count: AtomicUsize::new(0),
                spawn_barrier: Mutex::new(None),
                readiness_barriers: Mutex::new(None),
                children: Mutex::new(Vec::new()),
            }
        }
    }

    impl Platform for FakePlatform {
        fn probe_web(&self) -> Result<(), WebControlError> {
            self.probe_count.fetch_add(1, Ordering::SeqCst);
            self.supported
                .then_some(())
                .ok_or_else(|| WebControlError::new("web_subcommand_unsupported", "unsupported"))
        }
        fn spawn_web(&self, _port: u16) -> Result<Box<dyn ManagedChild>, WebControlError> {
            self.spawn_count.fetch_add(1, Ordering::SeqCst);
            if let Some(barrier) = self.spawn_barrier.lock().expect("spawn barrier").clone() {
                barrier.wait();
            }
            let state = Arc::new(FakeChildState::default());
            state.exit_on_terminate.store(true, Ordering::SeqCst);
            self.children.lock().expect("children").push(Arc::clone(&state));
            Ok(Box::new(FakeChild(state)))
        }
        fn probe_terminal(&self) -> Result<(), WebControlError> {
            Ok(())
        }
        fn http_ready(&self, _url: &str) -> bool {
            let barriers = self.readiness_barriers.lock().expect("readiness barriers").take();
            if let Some((entered, release)) = barriers {
                entered.wait();
                release.wait();
                return true;
            }
            self.ready.load(Ordering::SeqCst)
        }
        fn launch_terminal(&self) -> Result<(), WebControlError> {
            self.terminal_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn open_url(&self, _url: &str) -> Result<(), WebControlError> {
            Ok(())
        }
    }

    fn service(platform: Arc<FakePlatform>) -> WebService {
        WebService {
            inner: Arc::new(Mutex::new(Inner::default())),
            lifecycle: Arc::new(Mutex::new(())),
            platform,
            readiness_timeout: Duration::from_millis(80),
            poll_interval: Duration::from_millis(5),
            monitor_events: None,
        }
    }

    fn wait_for_state(service: &WebService, expected: WebState) -> WebStatus {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let status = service.status().expect("status");
            if status.state == expected {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {expected:?}, got {status:?}"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn validates_only_loopback_http_urls() {
        assert_eq!(localhost_port("http://127.0.0.1:1234/"), Some(1234));
        assert_eq!(localhost_port("http://localhost:9/"), Some(9));
        for invalid in [
            "https://127.0.0.1:1/",
            "http://0.0.0.0:1/",
            "http://example.com:1/",
            "http://localhost:0/",
            "http://localhost:1/x",
            "http://user@localhost:1/",
        ] {
            assert_eq!(localhost_port(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn command_help_fallback_is_typed() {
        let platform = Arc::new(FakePlatform::new(false));
        let service = service(platform);
        let error = service.start().expect_err("unsupported web must fail");
        assert_eq!(error.code, "web_subcommand_unsupported");
        assert_eq!(service.status().expect("status").state, WebState::Stopped);
    }

    #[test]
    fn duplicate_start_is_idempotent_and_readiness_transitions() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        let first = service.start().expect("start");
        let second = service.start().expect("duplicate start");
        assert_eq!(first.generation, second.generation);
        assert_eq!(platform.spawn_count.load(Ordering::SeqCst), 1);
        assert_eq!(platform.probe_count.load(Ordering::SeqCst), 1);
        platform.ready.store(true, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(20));
        let running = service.status().expect("running");
        assert_eq!(running.state, WebState::Running);
        assert!(running.url.is_some());
    }

    #[test]
    fn timeout_enters_error_and_restart_gets_new_generation() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        let first = service.start().expect("start").generation;
        thread::sleep(Duration::from_millis(110));
        assert_eq!(service.status().expect("timeout").state, WebState::Error);
        platform.ready.store(true, Ordering::SeqCst);
        let second = service.start().expect("restart").generation;
        assert!(second > first);
    }

    #[test]
    fn stop_during_start_only_terminates_owned_child() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        let stopped = service.stop().expect("stop");
        assert_eq!(stopped.state, WebState::Stopped);
        let children = platform.children.lock().expect("children");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].terminated.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn stop_timeout_escalates_owned_child() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        let child = Arc::clone(&platform.children.lock().expect("children")[0]);
        child.exit_on_terminate.store(false, Ordering::SeqCst);
        stop_inner(&service.inner, Duration::from_millis(5)).expect("stop");
        assert_eq!(child.terminated.load(Ordering::SeqCst), 1);
        assert_eq!(child.killed.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn concurrent_duplicate_start_serializes_before_probe_and_spawn() {
        let platform = Arc::new(FakePlatform::new(true));
        let barrier = Arc::new(Barrier::new(2));
        *platform.spawn_barrier.lock().expect("spawn barrier") = Some(Arc::clone(&barrier));
        let service = Arc::new(service(Arc::clone(&platform)));
        let first_service = Arc::clone(&service);
        let first = thread::spawn(move || first_service.start().expect("first start"));
        barrier.wait();
        let second_service = Arc::clone(&service);
        let second = thread::spawn(move || second_service.start().expect("duplicate start"));
        let first = first.join().expect("first thread");
        let second = second.join().expect("second thread");
        assert_eq!(first.generation, second.generation);
        assert_eq!(platform.probe_count.load(Ordering::SeqCst), 1);
        assert_eq!(platform.spawn_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn concurrent_stop_waits_for_start_generation_and_cleans_it() {
        let platform = Arc::new(FakePlatform::new(true));
        let spawn_barrier = Arc::new(Barrier::new(2));
        *platform.spawn_barrier.lock().expect("spawn barrier") = Some(Arc::clone(&spawn_barrier));
        let service = Arc::new(service(Arc::clone(&platform)));

        let start_service = Arc::clone(&service);
        let start = thread::spawn(move || start_service.start().expect("start"));
        spawn_barrier.wait();

        let stop_entered = Arc::new(Barrier::new(2));
        let stop_service = Arc::clone(&service);
        let stop_thread_entered = Arc::clone(&stop_entered);
        let stop = thread::spawn(move || {
            stop_thread_entered.wait();
            stop_service.stop().expect("stop")
        });
        stop_entered.wait();

        let started = start.join().expect("start thread");
        let stopped = stop.join().expect("stop thread");
        assert_eq!(stopped.generation, started.generation);
        assert_eq!(stopped.state, WebState::Stopped);
        let children = platform.children.lock().expect("children");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].terminated.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn try_wait_error_retains_child_ownership() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        let child = Arc::clone(&platform.children.lock().expect("children")[0]);
        child.try_wait_error.store(true, Ordering::SeqCst);
        let status = service.status().expect("status");
        assert_eq!(status.state, WebState::Error);
        assert!(status.has_owned_child);
        assert!(service.inner.lock().expect("inner").process.is_some());
        child.try_wait_error.store(false, Ordering::SeqCst);
        let stopped = service.stop().expect("retained child can be cleaned");
        assert_eq!(stopped.state, WebState::Stopped);
        assert_eq!(child.terminated.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn try_wait_error_during_cleanup_is_bounded_and_retains_ownership() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        let child = Arc::clone(&platform.children.lock().expect("children")[0]);
        child.try_wait_error.store(true, Ordering::SeqCst);
        let started = Instant::now();
        let error = stop_inner(&service.inner, Duration::from_millis(1)).expect_err("status must fail");
        assert_eq!(error.code, "child_status_failed");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(child.terminated.load(Ordering::SeqCst), 1);
        assert_eq!(child.killed.load(Ordering::SeqCst), 1);
        assert!(service.inner.lock().expect("inner").process.is_some());
        child.try_wait_error.store(false, Ordering::SeqCst);
    }

    #[test]
    fn terminate_error_is_typed_and_retains_child_ownership() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        let child = Arc::clone(&platform.children.lock().expect("children")[0]);
        child.terminate_error.store(true, Ordering::SeqCst);
        let error = service.stop().expect_err("terminate must fail");
        assert_eq!(error.code, "terminate_failed");
        assert!(service.inner.lock().expect("inner").process.is_some());
        child.terminate_error.store(false, Ordering::SeqCst);
    }

    #[test]
    fn kill_error_is_typed_and_retains_child_ownership() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        let child = Arc::clone(&platform.children.lock().expect("children")[0]);
        child.exit_on_terminate.store(false, Ordering::SeqCst);
        child.kill_error.store(true, Ordering::SeqCst);
        let _lifecycle = service.lifecycle.lock().expect("lifecycle");
        let error = stop_inner(&service.inner, Duration::from_millis(1)).expect_err("kill must fail");
        assert_eq!(error.code, "kill_failed");
        assert!(service.inner.lock().expect("inner").process.is_some());
        child.kill_error.store(false, Ordering::SeqCst);
    }

    #[test]
    fn shutdown_during_start_cleans_owned_child() {
        let platform = Arc::new(FakePlatform::new(true));
        let child = {
            let service = service(Arc::clone(&platform));
            service.start().expect("start");
            Arc::clone(&platform.children.lock().expect("children")[0])
        };
        assert_eq!(child.terminated.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn visible_terminal_is_independent_from_web_child() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.launch_terminal().expect("terminal");
        assert_eq!(platform.terminal_count.load(Ordering::SeqCst), 1);
        assert_eq!(platform.spawn_count.load(Ordering::SeqCst), 0);
        assert_eq!(service.status().expect("status"), WebStatus::default());
    }

    #[test]
    fn early_exit_while_starting_enters_error() {
        let platform = Arc::new(FakePlatform::new(true));
        let service = service(Arc::clone(&platform));
        service.start().expect("start");
        platform.children.lock().expect("children")[0]
            .exited
            .store(true, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(15));
        assert_eq!(service.status().expect("exit").state, WebState::Error);
    }

    #[test]
    fn blocked_old_readiness_cannot_overwrite_restarted_generation() {
        let platform = Arc::new(FakePlatform::new(true));
        let probe_entered = Arc::new(Barrier::new(2));
        let release_probe = Arc::new(Barrier::new(2));
        *platform.readiness_barriers.lock().expect("readiness barriers") =
            Some((Arc::clone(&probe_entered), Arc::clone(&release_probe)));
        let (events, monitor_events) = std::sync::mpsc::channel();
        let mut service = service(Arc::clone(&platform));
        service.monitor_events = Some(events);

        let old_generation = service.start().expect("old start").generation;
        probe_entered.wait();
        service.stop().expect("stop old generation");

        platform.ready.store(true, Ordering::SeqCst);
        let new_generation = service.start().expect("restart").generation;
        assert!(new_generation > old_generation);
        let before_release = wait_for_state(&service, WebState::Running);

        release_probe.wait();
        assert_eq!(
            monitor_events
                .recv_timeout(Duration::from_secs(1))
                .expect("old monitor exited"),
            old_generation
        );
        assert_eq!(service.status().expect("status after old probe"), before_release);
    }

    #[test]
    fn terminal_candidates_have_typed_missing_fallback() {
        let error =
            launch_first_terminal(&[("definitely-not-a-terminal-f3", &["opencode"])]).expect_err("missing terminal");
        assert_eq!(error.code, "terminal_emulator_not_found");
    }
}
