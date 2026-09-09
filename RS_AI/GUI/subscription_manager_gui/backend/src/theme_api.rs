/*
[INTEGRITY NOTES]
- Mục đích: API quản lý Theme (Tùy biến giao diện).
- Trách nhiệm: Đọc các file JSON từ thư mục `themes/` và trả về danh sách Theme cho frontend.
- Tương tác: Gọi bởi Frontend qua Tauri Invoke.
*/

use crate::models::Theme;
use std::fs;
use std::path::PathBuf;

// Lấy đường dẫn tới thư mục lưu trữ themes (chung rule resource_dir).
// KHÔNG tự ghi ra `default.json`: trước đây hàm này sinh file theme cứng, nên
// người dùng xoá/sửa theme mặc định thì lần chạy sau nó mọc lại, và app ngầm
// phụ thuộc vào đúng một cái tên. Giờ theme nào cũng như nhau, thiếu hết thì
// frontend giữ màu mặc định trong CSS.
fn get_themes_path() -> PathBuf {
    let base_dir = crate::storage::resource_dir("themes");
    if !base_dir.exists() {
        let _ = fs::create_dir_all(&base_dir);
    }
    base_dir
}

// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn get_available_themes() -> Result<Vec<Theme>, String> {
    let path = get_themes_path();
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_path = entry.path();
                    if file_path.extension().and_then(|e| e.to_str()) == Some("json") {
                        match fs::read_to_string(&file_path) {
                            Ok(content) => match serde_json::from_str::<Theme>(&content) {
                                Ok(theme) => themes.push(theme),
                                // Báo rõ file nào hỏng thay vì im lặng bỏ qua
                                // (trước đây theme lỗi biến mất khỏi list không dấu vết).
                                Err(e) => eprintln!("[theme] bỏ qua file hỏng {}: {e}", file_path.display()),
                            },
                            Err(e) => eprintln!("[theme] không đọc được {}: {e}", file_path.display()),
                        }
                    }
                }
            }
        }
    }

    // Sắp theo id để "theme đầu tiên" ổn định giữa các máy — `read_dir` không
    // bảo đảm thứ tự nên nếu không sắp thì fallback mỗi nơi một kết quả.
    themes.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(themes)
}
