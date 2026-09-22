//! Lựa chọn giao diện đã lưu bền (ngôn ngữ / theme / font).
//!
//! UNIVERSAL: tách khỏi việc ĐỌC tài nguyên (`actions::appearance` lo quét
//! `langs/`/`themes/`/`fonts/`). Module này chỉ GIỮ LỰA CHỌN người dùng đã
//! chọn, lưu JSON song song `engine_flags.json` + `diagnostics.json` để mở lại
//! app giữ nguyên giao diện. Thuần SYNC — ipc bọc `fastlane` ở ngoài.
//!
//! An toàn: giao diện lưu bền là ĐÚNG (khác quyền vượt cấp — cái đó chỉ ở RAM).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Lựa chọn giao diện. Rỗng = chưa chọn → frontend tự quyết mặc định (danh sách
/// từ `actions::appearance`), backend không đoán hộ.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppearanceSettings {
    /// Mã ngôn ngữ đang chọn (vd `vi`, `en`); rỗng = chưa chọn.
    #[serde(default)]
    pub lang: String,
    /// Id theme đang chọn (vd `default`); rỗng = chưa chọn.
    #[serde(default)]
    pub theme: String,
    /// Id font đang chọn (vd `default`); rỗng = chưa chọn.
    #[serde(default)]
    pub font: String,
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

/// Đường dẫn tệp JSON lưu lựa chọn giao diện (`<config_dir>/appearance.json`).
pub fn config_file_path() -> PathBuf {
    app_config_dir().join("appearance.json")
}

/// UNIVERSAL: id giao diện chỉ nhận chữ-số + `-`/`_` (chống rác/traversal);
/// rỗng hợp lệ (nghĩa là "chưa chọn").
fn valid_id(value: &str) -> bool {
    value.is_empty()
        || value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn validate(settings: &AppearanceSettings) -> Result<(), String> {
    if !valid_id(&settings.lang) {
        return Err(format!("lang không hợp lệ: {}", settings.lang));
    }
    if !valid_id(&settings.theme) {
        return Err(format!("theme không hợp lệ: {}", settings.theme));
    }
    if !valid_id(&settings.font) {
        return Err(format!("font không hợp lệ: {}", settings.font));
    }
    Ok(())
}

/// Đọc lựa chọn từ đĩa; thiếu tệp thì trả default rỗng (không lỗi).
pub fn load_appearance() -> Result<AppearanceSettings, String> {
    let path = config_file_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .map_err(|e| format!("Lỗi đọc lựa chọn giao diện {}: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppearanceSettings::default()),
        Err(e) => Err(format!("Lỗi đọc lựa chọn giao diện {}: {}", path.display(), e)),
    }
}

/// Ghi lựa chọn xuống đĩa (tạo thư mục cha nếu thiếu).
pub fn save_appearance(settings: &AppearanceSettings) -> Result<(), String> {
    validate(settings)?;
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Lỗi tạo thư mục {}: {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Lỗi mã hoá lựa chọn giao diện: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Lỗi ghi {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_empty() {
        let s = AppearanceSettings::default();
        assert!(s.lang.is_empty() && s.theme.is_empty() && s.font.is_empty());
    }

    #[test]
    fn empty_or_partial_json_loads() {
        // UNIVERSAL: file thiếu field → rỗng (chưa chọn), không lỗi.
        let s: AppearanceSettings = serde_json::from_str(r#"{"theme":"neon"}"#).expect("loads");
        assert_eq!(s.theme, "neon");
        assert!(s.lang.is_empty() && s.font.is_empty());
    }

    #[test]
    fn rejects_junk_id() {
        let mut s = AppearanceSettings::default();
        s.lang = "../etc".to_string();
        assert!(validate(&s).is_err());
        s = AppearanceSettings::default();
        s.theme = "a/b".to_string();
        assert!(validate(&s).is_err());
        // Rỗng vẫn hợp lệ (chưa chọn).
        assert!(validate(&AppearanceSettings::default()).is_ok());
    }

    #[test]
    fn roundtrip_preserves_choice() {
        let s = AppearanceSettings {
            lang: "vi".to_string(),
            theme: "default".to_string(),
            font: "dejavusans".to_string(),
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: AppearanceSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }
}
