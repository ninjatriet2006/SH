/*
[INTEGRITY NOTES]
- Mục đích: API quản lý Font (Tùy biến typography).
- Trách nhiệm: Đọc cấu hình Web Fonts từ `fonts/web_fonts.json` và quét Local Fonts từ `fonts/local/`.
- Tương tác: Gọi bởi Frontend qua Tauri Invoke.
*/

use std::fs;
use std::path::PathBuf;
use crate::models::FontInfo;


// Lấy đường dẫn tới thư mục fonts
fn get_fonts_path() -> PathBuf {
    let mut base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base_dir.push("fonts");
    if !base_dir.exists() {
        let _ = fs::create_dir_all(&base_dir);
    }
    base_dir
}



#[tauri::command]
pub fn get_available_fonts() -> Result<Vec<FontInfo>, String> {
    let mut fonts = Vec::new();
    let fonts_dir = get_fonts_path();
    
    // Luôn luôn có một font Hệ thống mặc định đứng đầu danh sách
    fonts.push(FontInfo {
        id: "default".to_string(),
        name: "Mặc định (Hệ thống)".to_string(),
        provider: "system".to_string(),
        family: "system-ui, -apple-system, sans-serif".to_string(),
        is_local: false,
        src_url: None,
    });
    
    // 1. Loại bỏ Web Fonts theo yêu cầu của user, chỉ dùng Local Fonts.
    
    // 2. Đọc Local Fonts trực tiếp từ thư mục `fonts`
    if let Ok(entries) = fs::read_dir(&fonts_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_path = entry.path();
                    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                        let ext = ext.to_lowercase();
                        if ext == "ttf" || ext == "otf" || ext == "woff" || ext == "woff2" {
                            let file_name = file_path.file_stem().and_then(|n| n.to_str()).unwrap_or("Unknown");
                            let font_id = format!("local_{}", file_name.replace(" ", "_").to_lowercase());
                            
                            // Trả về URI nội bộ để Frontend dùng @font-face
                            // Dùng path tuyệt đối (sẽ do Frontend quyết định ConvertFileSrc)
                            let src_path = file_path.to_string_lossy().to_string();
                            
                            fonts.push(FontInfo {
                                id: font_id,
                                name: format!("{} (Local)", file_name),
                                provider: "local".to_string(),
                                family: file_name.to_string(), // Tên family tạm
                                is_local: true,
                                src_url: Some(src_path),
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(fonts)
}
