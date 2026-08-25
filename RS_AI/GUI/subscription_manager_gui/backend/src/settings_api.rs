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

// Lấy đường dẫn tới thư mục lưu trữ settings
fn get_settings_path() -> PathBuf {
    // Để an toàn, chúng ta lấy thư mục chứa app hiện tại
    let mut path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    // Loại bỏ tên file thực thi (app)
    path.pop();
    
    // Nếu chạy cargo run, path sẽ ở target/debug. Ta muốn data luôn ở ./storage
    // Dùng cách tìm lên thư mục gốc
    let mut base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Đảm bảo tạo mục storage
    base_dir.push("storage");
    if !base_dir.exists() {
        let _ = fs::create_dir_all(&base_dir);
    }
    
    base_dir.push("settings.json");
    base_dir
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
