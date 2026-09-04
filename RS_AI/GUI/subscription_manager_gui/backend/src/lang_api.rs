use std::fs;
use std::path::PathBuf;

// Hàm lấy đường dẫn thư mục `langs`: dùng chung rule với storage
// (CWD trước, rồi tới cạnh binary) thay vì tự chế như trước.
fn get_langs_dir() -> PathBuf {
    crate::storage::resource_dir("langs")
}

// Lấy danh sách các ngôn ngữ có sẵn (dựa vào file JSON trong thư mục langs).
// Danh sách được SẮP XẾP: thứ tự `read_dir` phụ thuộc filesystem nên nếu không
// sắp thì "ngôn ngữ đầu tiên" mỗi máy một khác, khó tái hiện lỗi.
// `rename_all = "snake_case"` là BẮT BUỘC: Tauri v2 mặc định đổi tên tham số
// sang camelCase khi expose ra JS (`lang_code` -> `langCode`), trong khi bridge
// gửi snake_case → IPC báo "missing required key" và command không chạy.
#[tauri::command(rename_all = "snake_case")]
pub fn get_available_langs() -> Result<Vec<String>, String> {
    Ok(scan_lang_codes())
}

/// Quét mã ngôn ngữ từ tên file `*.json` trong `langs/`, đã sắp xếp.
pub fn scan_lang_codes() -> Vec<String> {
    let langs_dir = get_langs_dir();
    let mut langs = Vec::new();

    if let Ok(entries) = fs::read_dir(&langs_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    langs.push(stem.to_string());
                }
            }
        }
    }

    langs.sort();
    langs
}

/// Ngôn ngữ dùng khi chưa có lựa chọn hợp lệ: file `.json` đầu tiên trong
/// `langs/`. KHÔNG hardcode "vi" — app phải chạy được với bộ ngôn ngữ bất kỳ,
/// và khi thư mục rỗng thì trả `None` để UI hiện ID thay vì im lặng giả vờ ổn.
pub fn first_available_lang() -> Option<String> {
    scan_lang_codes().into_iter().next()
}

// Đọc nội dung file JSON ngôn ngữ
// `rename_all = "snake_case"` là BẮT BUỘC: Tauri v2 mặc định đổi tên tham số
// sang camelCase khi expose ra JS (`lang_code` -> `langCode`), trong khi bridge
// gửi snake_case → IPC báo "missing required key" và command không chạy.
#[tauri::command(rename_all = "snake_case")]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Danh sách phải sắp xếp: `read_dir` không bảo đảm thứ tự nên nếu không
    /// sắp thì "ngôn ngữ đầu tiên" (dùng làm fallback) mỗi máy một khác.
    #[test]
    fn danh_sach_ngon_ngu_duoc_sap_xep() {
        let langs = scan_lang_codes();
        let mut sorted = langs.clone();
        sorted.sort();
        assert_eq!(langs, sorted, "scan_lang_codes phải trả về danh sách đã sắp");
    }

    /// Fallback không được hardcode: phải là một mã có file thật trong `langs/`.
    #[test]
    fn fallback_lay_tu_file_co_that() {
        let langs = scan_lang_codes();
        match first_available_lang() {
            Some(code) => {
                assert!(
                    langs.contains(&code),
                    "fallback '{code}' không có file tương ứng trong langs/"
                );
                assert!(
                    get_lang_content(code.clone()).is_ok(),
                    "fallback '{code}' phải đọc được"
                );
            }
            None => assert!(
                langs.is_empty(),
                "có file ngôn ngữ nhưng first_available_lang() trả None"
            ),
        }
    }

    /// Chặn path traversal: mã ngôn ngữ chỉ được là ký tự an toàn.
    #[test]
    fn chan_ma_ngon_ngu_doc_hai() {
        for bad in ["../../etc/passwd", "..", "vi/../en", "vi.json", ""] {
            assert!(
                get_lang_content(bad.to_string()).is_err(),
                "mã '{bad}' phải bị từ chối"
            );
        }
    }
}
