/*
[INTEGRITY NOTES]
- Mục đích: Cài đặt của GUI (ngôn ngữ, theme) — lưu tại
  `~/.config/opencode/manager_gui.json`.
- Trách nhiệm: Đọc/ghi cài đặt, TỰ CHỮA khi mã ngôn ngữ đã lưu không còn file
  tương ứng. Không hardcode "vi" — chọn file đầu tiên thực có trong `langs/`.
- Tương tác: frontend `store/useSettingsStore.ts`, api::lang.

Vì sao đặt cạnh config của opencode: app này quản lý chính `~/.config/opencode/`
nên cài đặt GUI nằm cùng chỗ là hợp lý, và KHÔNG ghi vào thư mục cài đặt app
(có thể chỉ đọc, hoặc bị ghi đè khi cập nhật).
*/

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Mã ngôn ngữ. Rỗng = "chưa chọn / không còn hợp lệ" → frontend hiện ID.
    #[serde(default)]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme_id: String,
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            // Không hardcode "vi": lấy file ngôn ngữ đầu tiên thực có.
            language: crate::api::lang::first_available_lang().unwrap_or_default(),
            theme_id: default_theme(),
        }
    }
}

fn settings_path() -> PathBuf {
    let home = opencode_manager::config::get_home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".config").join("opencode").join("manager_gui.json")
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_gui_settings() -> Result<GuiSettings, String> {
    let path = settings_path();
    let mut settings = if path.is_file() {
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str::<GuiSettings>(&text).unwrap_or_else(|e| {
                eprintln!("[settings] manager_gui.json hỏng, dùng mặc định: {e}");
                GuiSettings::default()
            }),
            Err(e) => {
                eprintln!("[settings] không đọc được manager_gui.json: {e}");
                GuiSettings::default()
            }
        }
    } else {
        GuiSettings::default()
    };

    // Tự chữa mã ngôn ngữ không còn file tương ứng (đổi tên/xoá file, hoặc mã
    // rác). Lấy file đầu tiên có thật; hết file thì để rỗng để UI hiện ID.
    let available = crate::api::lang::scan_lang_codes();
    if settings.language.is_empty() || !available.contains(&settings.language) {
        if !settings.language.is_empty() {
            eprintln!(
                "[settings] ngôn ngữ '{}' không có trong langs/ {:?}, chuyển sang lựa chọn đầu tiên",
                settings.language, available
            );
        }
        settings.language = available.first().cloned().unwrap_or_default();
    }

    Ok(settings)
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_gui_settings(language: String, theme_id: String) -> Result<(), String> {
    if language.trim().is_empty() {
        return Err("Ngôn ngữ không được để trống".to_string());
    }
    if theme_id.trim().is_empty() {
        return Err("Theme không được để trống".to_string());
    }

    // Chỉ nhận ngôn ngữ thực sự có file — chặn ghi vào settings một mã không
    // tồn tại, nguyên nhân của lỗi "mọi lần mở sau đều hiện raw key".
    let available = crate::api::lang::scan_lang_codes();
    if !available.contains(&language) {
        return Err(format!(
            "Ngôn ngữ '{}' không có trong langs/. Có sẵn: {}",
            language,
            if available.is_empty() {
                "(không có file ngôn ngữ nào)".to_string()
            } else {
                available.join(", ")
            }
        ));
    }

    let settings = GuiSettings { language, theme_id };
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Không tạo được thư mục cài đặt: {e}"))?;
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|e| format!("Lỗi chuyển đổi cài đặt: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("Lỗi ghi file cài đặt: {e}"))?;
    Ok(())
}

/// Đường dẫn các file cấu hình — hiển thị ở trang Cài đặt để người dùng biết
/// app đang đọc/ghi vào đâu (khi họ sửa file tay hoặc chạy song song TUI).
#[derive(Debug, Clone, Serialize)]
pub struct ConfigPaths {
    pub opencode_json: String,
    pub auth_json: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_config_paths() -> Result<ConfigPaths, String> {
    Ok(ConfigPaths {
        opencode_json: opencode_manager::config::OpencodeConfig::file_path()
            .display()
            .to_string(),
        auth_json: opencode_manager::config::AuthEntry::file_path().display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_dinh_khong_hardcode_ngon_ngu() {
        let available = crate::api::lang::scan_lang_codes();
        let d = GuiSettings::default();
        if available.is_empty() {
            assert!(d.language.is_empty());
        } else {
            assert_eq!(d.language, available[0], "phải lấy file đầu tiên đã sắp");
        }
    }

    #[test]
    fn tu_choi_luu_ngon_ngu_khong_ton_tai() {
        let err = save_gui_settings("khong_ton_tai_9999".into(), "default".into())
            .expect_err("phải từ chối mã không có file");
        assert!(err.contains("không có trong langs/"), "lỗi: {err}");
    }
}
