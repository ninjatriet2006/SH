use std::fs;
use std::path::PathBuf;

// Hàm lấy đường dẫn thư mục `langs` ngang hàng với backend
fn get_langs_dir() -> PathBuf {
    let mut base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    
    // Thường backend chạy ở thư mục src-tauri, hoặc thư mục dự án gốc.
    // Thư mục langs nằm ngang hàng backend: GUI/subscription_manager_gui/langs
    // Nếu chạy từ thư mục gốc của app, base_dir là "subscription_manager_gui"
    base_dir.push("langs");
    
    // Nếu không thấy (khi build release binary), thử tìm ngang hàng file thực thi
    if !base_dir.exists() {
        let mut exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe_path.pop(); // Thư mục chứa app
        exe_path.push("langs");
        return exe_path;
    }
    
    base_dir
}

// Lấy danh sách các ngôn ngữ có sẵn (dựa vào file JSON trong thư mục langs)
#[tauri::command]
pub fn get_available_langs() -> Result<Vec<String>, String> {
    let langs_dir = get_langs_dir();
    let mut langs = Vec::new();

    if langs_dir.exists() && langs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(langs_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        langs.push(stem.to_string());
                    }
                }
            }
        }
    }

    Ok(langs)
}

// Đọc nội dung file JSON ngôn ngữ
#[tauri::command]
pub fn get_lang_content(lang_code: String) -> Result<serde_json::Value, String> {
    let mut path = get_langs_dir();
    path.push(format!("{}.json", lang_code));

    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("Lỗi parse JSON ngôn ngữ {}: {}", lang_code, e)),
                }
            },
            Err(e) => Err(format!("Lỗi đọc file ngôn ngữ {}: {}", lang_code, e)),
        }
    } else {
        Err(format!("Không tìm thấy ngôn ngữ: {}", lang_code))
    }
}
