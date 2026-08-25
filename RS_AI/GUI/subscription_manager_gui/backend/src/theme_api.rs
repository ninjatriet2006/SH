/*
[INTEGRITY NOTES]
- Mục đích: API quản lý Theme (Tùy biến giao diện).
- Trách nhiệm: Đọc các file JSON từ thư mục `themes/` và trả về danh sách Theme cho frontend.
- Tương tác: Gọi bởi Frontend qua Tauri Invoke.
*/

use std::fs;
use std::path::PathBuf;
use crate::models::Theme;

// Lấy đường dẫn tới thư mục lưu trữ themes
fn get_themes_path() -> PathBuf {
    let mut base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base_dir.push("themes");
    if !base_dir.exists() {
        let _ = fs::create_dir_all(&base_dir);
    }
    
    // Tự động tạo theme mặc định nếu chưa tồn tại
    let default_theme_path = base_dir.join("default.json");
    if !default_theme_path.exists() {
        let default_theme_content = r##"{
    "id": "default",
    "name": "Mặc định (Deep Space)",
    "type": "dark",
    "colors": {
      "bg_dark": "#0f172a",
      "bg_panel": "rgba(30, 41, 59, 0.7)",
      "text_primary": "#f8fafc",
      "text_secondary": "#94a3b8",
      "primary": "#6366f1",
      "primary_hover": "#818cf8",
      "success": "#10b981",
      "danger": "#ef4444",
      "warning": "#f59e0b",
      "border": "rgba(255, 255, 255, 0.1)"
    }
}"##;
        let _ = fs::write(default_theme_path, default_theme_content);
    }

    base_dir
}

#[tauri::command]
pub fn get_available_themes() -> Result<Vec<Theme>, String> {
    let path = get_themes_path();
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_path = entry.path();
                    if file_path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(&file_path) {
                            if let Ok(theme) = serde_json::from_str::<Theme>(&content) {
                                themes.push(theme);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(themes)
}
