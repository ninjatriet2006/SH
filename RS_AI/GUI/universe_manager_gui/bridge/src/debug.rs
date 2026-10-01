//! Hệ thống ghi nhật ký & chẩn đoán nội bộ (module `debug`).
//!
//! Ghi đồng thời ra console (`eprintln!`) cho môi trường dev và nối (append)
//! vào file `universe_manager.log` tại thư mục dữ liệu ứng dụng.
//! Có cơ chế xoay vòng (log rotation) tự động để không làm tràn đĩa.

use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::Serialize;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LogLevel {
    Info,
    Start,
    Stop,
    Warn,
    Error,
    Debug,
    Success,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Start => "START",
            Self::Stop => "STOP",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Debug => "DEBUG",
            Self::Success => "SUCCESS",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn set_log_dir(dir: &Path) {
    match LOG_PATH.lock() {
        Ok(mut guard) => {
            *guard = Some(dir.join("universe_manager.log"));
        }
        Err(_) => {}
    }
}

fn current_log_path() -> PathBuf {
    match LOG_PATH.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(path) => path.clone(),
            None => fallback_log_path(),
        },
        Err(_) => fallback_log_path(),
    }
}

fn fallback_log_path() -> PathBuf {
    #[cfg(unix)]
    {
        match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home)
                .join(".local/share/universe_manager/universe_manager.log"),
            None => std::env::temp_dir().join("universe_manager.log"),
        }
    }
    #[cfg(windows)]
    {
        match std::env::var_os("APPDATA") {
            Some(appdata) => PathBuf::from(appdata)
                .join("universe_manager\\universe_manager.log"),
            None => std::env::temp_dir().join("universe_manager.log"),
        }
    }
}

/// Ghi một dòng log với mức độ và tag phân loại.
pub fn log(level: LogLevel, tag: &str, message: impl AsRef<str>) {
    let msg = message.as_ref();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{now}][{}][{}] {}", level, tag, msg);

    // Xuất console cho dev
    eprintln!("{line}");

    // Append file log an toàn (best-effort, không bao giờ panic/crash app)
    let path = current_log_path();
    rotate_if_oversized(&path, 2 * 1024 * 1024); // 2MB trần

    match path.parent() {
        Some(parent) => {
            let _ = std::fs::create_dir_all(parent);
        }
        None => {}
    }

    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut f) => {
            let _ = writeln!(f, "{line}");
        }
        Err(_) => {}
    }
}

/// Tự động xoay vòng file log sang .old nếu vượt quá ngưỡng bytes quy định.
fn rotate_if_oversized(path: &Path, max_bytes: u64) {
    match std::fs::metadata(path) {
        Ok(meta) => match meta.len() > max_bytes {
            true => {
                let old = path.with_extension("log.old");
                let _ = std::fs::rename(path, &old);
            }
            false => {}
        },
        Err(_) => {}
    }
}

pub fn info(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Info, tag, message);
}

pub fn start(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Start, tag, message);
}

pub fn stop(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Stop, tag, message);
}

pub fn warn(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Warn, tag, message);
}

pub fn error(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Error, tag, message);
}

pub fn debug(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Debug, tag, message);
}

pub fn success(tag: &str, message: impl AsRef<str>) {
    log(LogLevel::Success, tag, message);
}
