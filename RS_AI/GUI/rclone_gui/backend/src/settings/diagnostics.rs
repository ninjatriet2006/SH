//! Cấu hình chẩn đoán (debug) lưu JSON trong thư mục config của app.
//!
//! UNIVERSAL: tách khỏi `settings::engine` vì đây KHÔNG phải việc transfer —
//! không được đóng dấu vào vé (`TransferTicket`/`QueueItem`). Hiện chỉ giữ ngưỡng
//! dọn log; `core::debug::rotate_if_oversized` đọc giá trị này ở ranh giới job.
//! Module thuần SYNC — ipc gọi thì bọc [`crate::logic::fastlane::fastlane`] ở ngoài.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// UNIVERSAL: mặc định ngưỡng dọn log (MB) khi thiếu file/thiếu field.
pub const DEFAULT_LOG_ROTATE_MB: u32 = 5;

/// Cấu hình chẩn đoán do người dùng chỉnh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSettings {
    /// UNIVERSAL: `backend.log` vượt ngưỡng này (MB) thì xoay sang `.old` ở
    /// ranh giới job. Là CON SỐ "dọn bao nhiêu", không phải công tắc bật/tắt.
    #[serde(default = "default_log_rotate_mb")]
    pub log_rotate_mb: u32,
}

fn default_log_rotate_mb() -> u32 {
    DEFAULT_LOG_ROTATE_MB
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            log_rotate_mb: DEFAULT_LOG_ROTATE_MB,
        }
    }
}

impl DebugSettings {
    /// UNIVERSAL: đổi ngưỡng MB sang byte cho `rotate_if_oversized`.
    pub fn log_rotate_bytes(&self) -> u64 {
        self.log_rotate_mb as u64 * 1024 * 1024
    }
}

fn app_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir).join("rclone_gui");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("rclone_gui")
}

/// Đường dẫn tệp JSON lưu cấu hình chẩn đoán (`<config_dir>/diagnostics.json`).
pub fn config_file_path() -> PathBuf {
    app_config_dir().join("diagnostics.json")
}

fn validate(settings: &DebugSettings) -> Result<(), String> {
    if !(1..=500).contains(&settings.log_rotate_mb) {
        return Err("log_rotate_mb must be 1..=500".to_string());
    }
    Ok(())
}

/// Đọc cấu hình từ đĩa; thiếu tệp thì trả default (không lỗi).
pub fn load_debug_settings() -> Result<DebugSettings, String> {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|e| format!("Lỗi đọc cấu hình chẩn đoán {}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DebugSettings::default()),
        Err(e) => Err(format!("Lỗi đọc cấu hình chẩn đoán {}: {}", path.display(), e)),
    }
}

/// Ghi cấu hình xuống đĩa (tạo thư mục cha nếu thiếu).
pub fn save_debug_settings(settings: &DebugSettings) -> Result<(), String> {
    validate(settings)?;
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Lỗi tạo thư mục {}: {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Lỗi mã hoá cấu hình chẩn đoán: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Lỗi ghi {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_five_mb() {
        assert_eq!(DebugSettings::default().log_rotate_mb, 5);
        assert_eq!(DebugSettings::default().log_rotate_bytes(), 5 * 1024 * 1024);
    }

    #[test]
    fn old_or_empty_json_falls_back_to_default() {
        // UNIVERSAL: file thiếu field → default 5 (không lỗi).
        let s: DebugSettings = serde_json::from_str("{}").expect("empty json loads");
        assert_eq!(s.log_rotate_mb, 5);
    }

    #[test]
    fn rejects_zero_and_over_cap() {
        let mut s = DebugSettings::default();
        s.log_rotate_mb = 0;
        assert!(validate(&s).is_err());
        s.log_rotate_mb = 501;
        assert!(validate(&s).is_err());
        s.log_rotate_mb = 50;
        assert!(validate(&s).is_ok());
    }
}
