//! Cấu hình engine transfer (S2: đã wire vào transfer/plans).
//!
//! Hành vi config cũ (rclone.conf INI) giữ nguyên trong [`crate::logic::config_manager`];
//! module này chỉ quản lý `EngineSettings` lưu JSON trong thư mục config của app.
//! UNIVERSAL: module thuần SYNC — ipc gọi thì bọc [`crate::logic::fastlane::fastlane`] ở ngoài.
//!
//! UNIVERSAL: tách theo BẢN CHẤT — `EngineSwitches` là các công tắc bật/tắt,
//! `EngineTuning` là các con số + đường dẫn chỉnh tay. Cả hai đều thuộc lĩnh vực
//! transfer nên gộp trong `EngineSettings` và được đóng dấu nguyên khối vào vé
//! (`TransferTicket`/`QueueItem`). Ngưỡng dọn log KHÔNG nằm đây (không phải việc
//! transfer) — xem [`crate::settings::diagnostics`].

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// UNIVERSAL: nhóm CÔNG TẮC bật/tắt của engine (mặc định tất cả tắt).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EngineSwitches {
    #[serde(default)]
    pub fast_list: bool,
    #[serde(default)]
    pub server_side_across: bool,
    #[serde(default)]
    pub dry_run: bool,
    // UNIVERSAL: false (mặc định) = bóc thư mục thành từng món qua queue con;
    // true = chạy nguyên khối một lệnh src→dst duy nhất qua queue con.
    #[serde(default)]
    pub bulk_transfer: bool,
}

fn default_queue_concurrency() -> u32 {
    4
}

/// UNIVERSAL: nhóm CON SỐ + đường dẫn chỉnh tay của engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineTuning {
    pub transfers: u32,
    pub checkers: u32,
    #[serde(default = "default_queue_concurrency")]
    pub queue_concurrency: u32,
    #[serde(default)]
    pub backup_dir: Option<String>,
}

impl Default for EngineTuning {
    fn default() -> Self {
        Self {
            transfers: 4,
            checkers: 8,
            queue_concurrency: 4,
            backup_dir: None,
        }
    }
}

/// Cấu hình toàn cục áp cho mọi transfer (S2 đã wire: xem `logic::tracker`).
///
/// UNIVERSAL: hai nhóm con `flatten` để JSON `engine_flags.json` vẫn PHẲNG
/// (khoá cùng cấp như bản cũ) → file cũ đọc được, vé vẫn mang 1 field.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EngineSettings {
    #[serde(flatten)]
    pub switches: EngineSwitches,
    #[serde(flatten)]
    pub tuning: EngineTuning,
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

/// Đường dẫn tệp JSON lưu cấu hình (`<config_dir>/engine_flags.json`).
pub fn config_file_path() -> PathBuf {
    app_config_dir().join("engine_flags.json")
}

fn validate(settings: &EngineSettings) -> Result<(), String> {
    if !(1..=32).contains(&settings.tuning.transfers) {
        return Err("transfers must be 1..=32".to_string());
    }
    if !(1..=64).contains(&settings.tuning.checkers) {
        return Err("checkers must be 1..=64".to_string());
    }
    if !(1..=32).contains(&settings.tuning.queue_concurrency) {
        return Err("queue_concurrency must be 1..=32".to_string());
    }
    if let Some(dir) = settings.tuning.backup_dir.as_deref() {
        if dir.trim().is_empty() {
            return Err("backup_dir must be non-empty or null".to_string());
        }
    }
    Ok(())
}

/// Đọc cấu hình từ đĩa; thiếu tệp thì trả default (giữ hành vi cũ: không lỗi).
pub fn load_engine_flags() -> Result<EngineSettings, String> {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|e| format!("Lỗi đọc cấu hình engine {}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(EngineSettings::default()),
        Err(e) => Err(format!("Lỗi đọc cấu hình engine {}: {}", path.display(), e)),
    }
}

/// Ghi cấu hình xuống đĩa (tạo thư mục cha nếu thiếu).
pub fn save_engine_flags(settings: &EngineSettings) -> Result<(), String> {
    validate(settings)?;
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Lỗi tạo thư mục {}: {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Lỗi mã hoá cấu hình engine: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Lỗi ghi {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_rclone_baseline() {
        let s = EngineSettings::default();
        assert_eq!(s.tuning.transfers, 4);
        assert_eq!(s.tuning.checkers, 8);
        assert_eq!(s.tuning.queue_concurrency, 4);
        assert!(!s.switches.fast_list);
        assert!(!s.switches.server_side_across);
        assert!(!s.switches.dry_run);
        assert_eq!(s.tuning.backup_dir, None);
        assert!(!s.switches.bulk_transfer);
    }

    #[test]
    fn bulk_defaults_off_and_old_json_without_field_still_loads() {
        // UNIVERSAL: file cũ thiếu `bulk_transfer` và `queue_concurrency` vẫn đọc được → default.
        let old = r#"{"transfers":4,"checkers":8,"fast_list":false,"server_side_across":false,"dry_run":false,"backup_dir":null}"#;
        let s: EngineSettings = serde_json::from_str(old).expect("old json loads");
        assert!(!s.switches.bulk_transfer);
        assert_eq!(s.tuning.queue_concurrency, 4);
        assert_eq!(s, EngineSettings::default());
    }

    #[test]
    fn json_stays_flat_after_split() {
        // UNIVERSAL: dù tách 2 nhóm, JSON vẫn PHẲNG (khoá cùng cấp) nhờ flatten.
        let json = serde_json::to_string(&EngineSettings::default()).expect("serialize");
        assert!(json.contains("\"transfers\""));
        assert!(json.contains("\"queue_concurrency\""));
        assert!(json.contains("\"fast_list\""));
        assert!(!json.contains("\"switches\""), "không được lộ tên nhóm ra JSON");
        assert!(!json.contains("\"tuning\""), "không được lộ tên nhóm ra JSON");
    }

    #[test]
    fn serde_roundtrip_preserves_all_fields() {
        let s = EngineSettings {
            switches: EngineSwitches {
                fast_list: true,
                server_side_across: true,
                dry_run: true,
                bulk_transfer: true,
            },
            tuning: EngineTuning {
                transfers: 8,
                checkers: 16,
                queue_concurrency: 6,
                backup_dir: Some("/tmp/backup".to_string()),
            },
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: EngineSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }

    #[test]
    fn rejects_out_of_range_and_empty_backup_dir() {
        let mut s = EngineSettings::default();
        s.tuning.transfers = 0;
        assert!(validate(&s).is_err());
        s = EngineSettings::default();
        s.tuning.checkers = 65;
        assert!(validate(&s).is_err());
        s = EngineSettings::default();
        s.tuning.queue_concurrency = 0;
        assert!(validate(&s).is_err());
        s = EngineSettings::default();
        s.tuning.queue_concurrency = 33;
        assert!(validate(&s).is_err());
        s = EngineSettings::default();
        s.tuning.backup_dir = Some("   ".to_string());
        assert!(validate(&s).is_err());
    }
}
