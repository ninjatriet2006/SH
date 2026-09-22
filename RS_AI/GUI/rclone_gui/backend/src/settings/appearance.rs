//! Cấu hình giao diện đã lưu bền: LỰA CHỌN (lang/theme/font) + NGUỒN (thư mục
//! tài nguyên) — lưu JSON song song `engine_flags.json` + `diagnostics.json`.
//!
//! UNIVERSAL: hai nhóm tách bạch trong một file "appearance". Nhóm LỰA CHỌN là
//! đang chọn cái nào (rỗng = "none" → frontend quyết mặc định). Nhóm NGUỒN là
//! thư mục quét `langs/`/`themes/`/`fonts/` (rỗng = vị trí đóng gói mặc định qua
//! `core::resources`; điền = quét đúng thư mục đó — cho người dùng trỏ thư mục
//! riêng, và cho test ẤN ĐỊNH path từ code).
//!
//! Việc ĐỌC file do `actions::appearance` lo; module này chỉ GIỮ lựa chọn/nguồn.
//! Thuần SYNC — ipc bọc `fastlane` ở ngoài.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Lựa chọn + nguồn giao diện. Mọi field rỗng = mặc định (chưa chọn / vị trí
/// đóng gói), nên file cũ thiếu field vẫn đọc được.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppearanceSettings {
    // ----- Nhóm LỰA CHỌN (rỗng = none = frontend quyết) -----
    /// Mã ngôn ngữ đang chọn (vd `vi`, `en`); rỗng = chưa chọn.
    #[serde(default)]
    pub lang: String,
    /// Id theme đang chọn (vd `neon`); rỗng = chưa chọn.
    #[serde(default)]
    pub theme: String,
    /// Id font đang chọn; rỗng = chưa chọn.
    #[serde(default)]
    pub font: String,

    // ----- Nhóm NGUỒN (rỗng = vị trí đóng gói mặc định) -----
    /// Thư mục chứa file ngôn ngữ; rỗng = `resource_dir("langs")`.
    #[serde(default)]
    pub langs_dir: String,
    /// Thư mục chứa file theme; rỗng = `resource_dir("themes")`.
    #[serde(default)]
    pub themes_dir: String,
    /// Thư mục chứa file font; rỗng = `resource_dir("fonts")`.
    #[serde(default)]
    pub fonts_dir: String,
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

/// UNIVERSAL: thư mục nguồn rỗng = mặc định (hợp lệ); điền thì phải là thư mục
/// CÓ THẬT (không trỏ vào chỗ trống/không tồn tại).
fn valid_dir(value: &str) -> bool {
    let v = value.trim();
    v.is_empty() || PathBuf::from(v).is_dir()
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
    if !valid_dir(&settings.langs_dir) {
        return Err(format!("langs_dir không phải thư mục tồn tại: {}", settings.langs_dir));
    }
    if !valid_dir(&settings.themes_dir) {
        return Err(format!("themes_dir không phải thư mục tồn tại: {}", settings.themes_dir));
    }
    if !valid_dir(&settings.fonts_dir) {
        return Err(format!("fonts_dir không phải thư mục tồn tại: {}", settings.fonts_dir));
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
        assert!(s.langs_dir.is_empty() && s.themes_dir.is_empty() && s.fonts_dir.is_empty());
    }

    #[test]
    fn empty_or_partial_json_loads() {
        // UNIVERSAL: file thiếu field → rỗng (chưa chọn / mặc định), không lỗi.
        let s: AppearanceSettings = serde_json::from_str(r#"{"theme":"neon"}"#).expect("loads");
        assert_eq!(s.theme, "neon");
        assert!(s.lang.is_empty() && s.font.is_empty() && s.themes_dir.is_empty());
    }

    #[test]
    fn rejects_junk_id() {
        let mut s = AppearanceSettings::default();
        s.lang = "../etc".to_string();
        assert!(validate(&s).is_err());
        s = AppearanceSettings::default();
        s.theme = "a/b".to_string();
        assert!(validate(&s).is_err());
        assert!(validate(&AppearanceSettings::default()).is_ok());
    }

    #[test]
    fn source_dir_empty_ok_but_nonexistent_rejected() {
        // UNIVERSAL: rỗng = mặc định (ok); điền path không tồn tại → từ chối.
        let mut s = AppearanceSettings::default();
        s.themes_dir = "/khong/ton/tai/chac_chan_123".to_string();
        assert!(validate(&s).is_err());
        // Thư mục thật (temp) thì hợp lệ.
        s = AppearanceSettings::default();
        s.themes_dir = std::env::temp_dir().to_string_lossy().into_owned();
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn roundtrip_preserves_choice_and_source() {
        let s = AppearanceSettings {
            lang: "vi".to_string(),
            theme: "neon".to_string(),
            font: "dejavusans".to_string(),
            langs_dir: "/opt/langs".to_string(),
            themes_dir: String::new(),
            fonts_dir: String::new(),
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: AppearanceSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }
}
