//! Sổ chẩn đoán backend (module `core::debug`, file `backend.log`, event `backend-log`).
//!
//! Khác với log chuyển file của tracker (dòng `--use-json-log` của rclone → % tiến
//! độ trong `logic::tracker`): module này chỉ ghi chẩn đoán nội bộ backend.
//!
//! Giữ `eprintln!` để log vẫn thấy khi chạy dev; file append nằm trong thư
//! mục config của app; event chỉ emit khi có `AppHandle` (best-effort).

use serde::Serialize;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// Mức log dùng chung cho backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    /// Chuỗi ngắn dùng cho dòng log text.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Payload của event `backend-log` gửi lên Frontend.
#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub level: Level,
    pub tag: String,
    pub message: String,
}

fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir).join("rclone_gui");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("rclone_gui")
}

/// Đường dẫn file log (`<config_dir>/backend.log`).
pub fn log_file_path() -> PathBuf {
    config_dir().join("backend.log")
}

/// Ghi một dòng log: file append (best-effort) + `eprintln!` + event
/// `backend-log` khi có handle (best-effort, thiếu thì bỏ qua).
pub fn log(handle: Option<&AppHandle>, level: Level, tag: &str, message: impl AsRef<str>) {
    let msg = message.as_ref();
    let line = format!("[{}][{}] {}", level, tag, msg);
    eprintln!("{line}");
    // File append: mọi lỗi đều bỏ qua để log không bao giờ làm crash app.
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
    }
    if let Some(app) = handle {
        let _ = app.emit(
            "backend-log",
            LogEvent {
                level,
                tag: tag.to_string(),
                message: msg.to_string(),
            },
        );
    }
}

/// Shortcut cho [`log`] với [`Level::Info`].
pub fn info(handle: Option<&AppHandle>, tag: &str, message: impl AsRef<str>) {
    log(handle, Level::Info, tag, message);
}

/// Shortcut cho [`log`] với [`Level::Warn`].
pub fn warn(handle: Option<&AppHandle>, tag: &str, message: impl AsRef<str>) {
    log(handle, Level::Warn, tag, message);
}

/// Shortcut cho [`log`] với [`Level::Error`].
pub fn error(handle: Option<&AppHandle>, tag: &str, message: impl AsRef<str>) {
    log(handle, Level::Error, tag, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_labels_match_wire_format() {
        assert_eq!(Level::Info.as_str(), "INFO");
        assert_eq!(Level::Warn.as_str(), "WARN");
        assert_eq!(Level::Error.as_str(), "ERROR");
        assert_eq!(Level::Warn.to_string(), "WARN");
    }

    #[test]
    fn log_file_lives_in_app_config_dir() {
        assert_eq!(log_file_path().file_name().and_then(|n| n.to_str()), Some("backend.log"));
    }

    #[test]
    fn log_without_handle_does_not_panic() {
        // Ghi file + eprintln, không emit event.
        log(None, Level::Info, "test", "hello");
    }
}
