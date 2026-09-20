//! Cờ toàn cục cho engine transfer (S2: đã wire vào transfer/plans).
//!
//! Hành vi config cũ (rclone.conf INI) giữ nguyên trong [`super::config_manager`];
//! module này chỉ quản lý `GlobalFlags` lưu JSON trong thư mục config của app.

use crate::core::task::blocking;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Cờ toàn cục áp cho mọi transfer (S2 đã wire: xem `logic::transfer`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalFlags {
    pub transfers: u32,
    pub checkers: u32,
    pub fast_list: bool,
    pub server_side_across: bool,
    pub dry_run: bool,
    pub backup_dir: Option<String>,
}

impl Default for GlobalFlags {
    fn default() -> Self {
        Self {
            transfers: 4,
            checkers: 8,
            fast_list: false,
            server_side_across: false,
            dry_run: false,
            backup_dir: None,
        }
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

/// Đường dẫn tệp JSON lưu cờ (`<config_dir>/engine_flags.json`).
pub fn config_file_path() -> PathBuf {
    app_config_dir().join("engine_flags.json")
}

fn validate(flags: &GlobalFlags) -> Result<(), String> {
    if !(1..=32).contains(&flags.transfers) {
        return Err("transfers must be 1..=32".to_string());
    }
    if !(1..=64).contains(&flags.checkers) {
        return Err("checkers must be 1..=64".to_string());
    }
    if let Some(dir) = flags.backup_dir.as_deref() {
        if dir.trim().is_empty() {
            return Err("backup_dir must be non-empty or null".to_string());
        }
    }
    Ok(())
}

/// Đọc cờ từ đĩa; thiếu tệp thì trả default (giữ hành vi cũ: không lỗi).
pub fn load_engine_flags() -> Result<GlobalFlags, String> {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|e| format!("Lỗi đọc cấu hình engine {}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(GlobalFlags::default()),
        Err(e) => Err(format!("Lỗi đọc cấu hình engine {}: {}", path.display(), e)),
    }
}

/// Ghi cờ xuống đĩa (tạo thư mục cha nếu thiếu).
pub fn save_engine_flags(flags: &GlobalFlags) -> Result<(), String> {
    validate(flags)?;
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Lỗi tạo thư mục {}: {}", parent.display(), e))?;
    }
    let content =
        serde_json::to_string_pretty(flags).map_err(|e| format!("Lỗi mã hoá cấu hình engine: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Lỗi ghi {}: {}", path.display(), e))
}

/// IPC get: trả cờ hiện tại (thiếu tệp → default).
pub async fn get_engine_flags() -> Result<GlobalFlags, String> {
    blocking(load_engine_flags).await
}

/// IPC set: kiểm tra, lưu rồi trả cờ đã lưu.
pub async fn set_engine_flags(flags: GlobalFlags) -> Result<GlobalFlags, String> {
    blocking(move || {
        save_engine_flags(&flags)?;
        Ok(flags)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_rclone_baseline() {
        let flags = GlobalFlags::default();
        assert_eq!(flags.transfers, 4);
        assert_eq!(flags.checkers, 8);
        assert!(!flags.fast_list);
        assert!(!flags.server_side_across);
        assert!(!flags.dry_run);
        assert_eq!(flags.backup_dir, None);
    }

    #[test]
    fn serde_roundtrip_preserves_all_fields() {
        let flags = GlobalFlags {
            transfers: 8,
            checkers: 16,
            fast_list: true,
            server_side_across: true,
            dry_run: true,
            backup_dir: Some("/tmp/backup".to_string()),
        };
        let json = serde_json::to_string(&flags).expect("serialize");
        let back: GlobalFlags = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(flags, back);
    }

    #[test]
    fn rejects_out_of_range_and_empty_backup_dir() {
        let mut flags = GlobalFlags::default();
        flags.transfers = 0;
        assert!(validate(&flags).is_err());
        flags = GlobalFlags::default();
        flags.checkers = 65;
        assert!(validate(&flags).is_err());
        flags = GlobalFlags::default();
        flags.backup_dir = Some("   ".to_string());
        assert!(validate(&flags).is_err());
    }
}
