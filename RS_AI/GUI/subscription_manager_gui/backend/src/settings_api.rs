use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub language: String,
    pub timezone: String,
    #[serde(default = "default_theme_id")]
    pub theme_id: String,
    #[serde(default = "default_font_id")]
    pub font_id: String,
}

fn default_theme_id() -> String {
    "default".to_string()
}

fn default_font_id() -> String {
    "default".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "vi".to_string(),
            timezone: "Asia/Ho_Chi_Minh".to_string(),
            theme_id: "default".to_string(),
            font_id: "default".to_string(),
        }
    }
}

// Lấy đường dẫn tới file settings: dùng chung rule resolve với `storage`
// (CWD trước để tương thích cũ, rồi tới thư mục chứa binary).
fn get_settings_path() -> PathBuf {
    let path = crate::storage::storage_file_path("settings.json");

    // Đảm bảo tạo mục storage
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }

    path
}

// Đọc cài đặt
#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    let path = get_settings_path();
    
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<Settings>(&content) {
                    Ok(settings) => Ok(settings),
                    Err(_) => Ok(Settings::default()), // Lỗi parse thì lấy mặc định
                }
            },
            Err(_) => Ok(Settings::default()), // Lỗi đọc file thì lấy mặc định
        }
    } else {
        Ok(Settings::default()) // File chưa tồn tại
    }
}

// Lưu cài đặt
#[tauri::command]
pub fn save_settings(language: String, timezone: String, theme_id: String, font_id: String) -> Result<(), String> {
    // Từ chối giá trị rỗng — trước đây ghi đè settings.json bằng chuỗi trống
    // khiến theme/font rơi về trạng thái không tồn tại.
    if language.trim().is_empty() {
        return Err("Ngôn ngữ không được để trống".to_string());
    }
    if timezone.trim().is_empty() {
        return Err("Múi giờ không được để trống".to_string());
    }
    if theme_id.trim().is_empty() {
        return Err("Theme không được để trống".to_string());
    }
    if font_id.trim().is_empty() {
        return Err("Font không được để trống".to_string());
    }
    // Settings ghi file riêng nhưng vẫn serialize chung để tránh nghẽn IO dồn dập.
    let _store_guard = crate::storage::lock_store();
    let settings = Settings { language, timezone, theme_id, font_id };
    let path = get_settings_path();
    
    match serde_json::to_string_pretty(&settings) {
        Ok(json_str) => {
            match fs::write(path, json_str) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Lỗi ghi file settings: {}", e)),
            }
        },
        Err(e) => Err(format!("Lỗi chuyển đổi settings: {}", e)),
    }
}
