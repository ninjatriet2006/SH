use std::fs;
use std::path::PathBuf;

// Hàm lấy đường dẫn thư mục `langs`: dùng chung rule với storage
// (CWD trước, rồi tới cạnh binary) thay vì tự chế như trước.
fn get_langs_dir() -> PathBuf {
    crate::storage::resource_dir("langs")
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
    // Chặn path traversal — trước đây `lang_code = "../../x"` đọc file ngoài
    // thư mục langs. Chỉ cho phép chữ, số, gạch nối và gạch dưới.
    if lang_code.is_empty()
        || !lang_code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Mã ngôn ngữ không hợp lệ: {}", lang_code));
    }
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
