/*
[INTEGRITY NOTES]
Mục đích: API đa ngôn ngữ — liệt kê và đọc file từ điển trong `langs/`.
Trách nhiệm: Tự nhận diện file `*.json` có thật thay vì gắn cứng một mã ngôn
  ngữ. Không tìm được thì trả lỗi để frontend hiện raw ID (lộ lỗi rõ ràng).
Các module tương tác: frontend/bridge/lang_api.ts, core::resources.
Lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::core::resources::resource_dir;
use std::fs;
use std::path::PathBuf;

fn langs_dir() -> PathBuf {
    resource_dir("langs")
}

/// Quét mã ngôn ngữ từ tên file `*.json`, ĐÃ SẮP XẾP.
/// `read_dir` không bảo đảm thứ tự nên không sắp thì "file đầu tiên" (dùng làm
/// fallback) sẽ khác nhau giữa các máy, lỗi rất khó tái hiện.
fn scan_lang_codes() -> Vec<String> {
    let mut codes = Vec::new();
    if let Ok(entries) = fs::read_dir(langs_dir()) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    codes.push(stem.to_string());
                }
            }
        }
    }
    codes.sort();
    codes
}

/// Mã ngôn ngữ dùng khi chưa có lựa chọn hợp lệ: file đầu tiên thực có.
/// KHÔNG hardcode "vi" — app phải chạy với bộ ngôn ngữ bất kỳ.
#[allow(dead_code)]
fn first_available_lang() -> Option<String> {
    scan_lang_codes().into_iter().next()
}

/// Đọc nội dung một file từ điển (logic dùng chung cho command + test).
fn read_lang_content(lang_code: &str) -> Result<serde_json::Value, String> {
    // Chặn path traversal: `lang_code` ghép trực tiếp vào đường dẫn nên phải
    // giới hạn ký tự, tránh `../../etc/passwd`.
    if lang_code.is_empty()
        || !lang_code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Mã ngôn ngữ không hợp lệ: {}", lang_code));
    }

    let path = langs_dir().join(format!("{}.json", lang_code));
    if !path.is_file() {
        return Err(format!("Không tìm thấy ngôn ngữ: {}", lang_code));
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("Lỗi đọc file ngôn ngữ {}: {}", lang_code, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON ngôn ngữ {}: {}", lang_code, e))
}

#[tauri::command]
pub fn get_available_langs() -> Result<Vec<String>, String> {
    Ok(scan_lang_codes())
}

#[tauri::command]
pub fn get_lang_content(lang_code: String) -> Result<serde_json::Value, String> {
    read_lang_content(&lang_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Danh sách phải sắp xếp để fallback ổn định giữa các máy.
    #[test]
    fn danh_sach_duoc_sap_xep() {
        let codes = scan_lang_codes();
        let mut sorted = codes.clone();
        sorted.sort();
        assert_eq!(codes, sorted);
    }

    /// Fallback không hardcode: phải là mã có file thật và đọc được.
    #[test]
    fn fallback_lay_tu_file_co_that() {
        let codes = scan_lang_codes();
        match first_available_lang() {
            Some(code) => {
                assert!(codes.contains(&code), "fallback '{code}' không có file");
                let dict = read_lang_content(&code).unwrap_or_else(|e| panic!("đọc '{code}' thất bại: {e}"));
                assert!(
                    dict.as_object().is_some_and(|m| !m.is_empty()),
                    "'{code}' phải là object không rỗng"
                );
            }
            None => assert!(codes.is_empty(), "có file nhưng fallback trả None"),
        }
    }

    /// Chặn path traversal và mã rác.
    #[test]
    fn chan_ma_ngon_ngu_doc_hai() {
        for bad in ["../../etc/passwd", "..", "vi/../en", "vi.json", ""] {
            assert!(read_lang_content(bad).is_err(), "mã '{bad}' phải bị từ chối");
        }
    }

    /// Mọi ID mà UI tham chiếu (`data-lang-id`) phải có trong MỌI file ngôn ngữ
    /// — thiếu là chỗ đó hiện raw ID trên giao diện.
    #[test]
    fn moi_file_ngon_ngu_du_id_ma_ui_dung() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app root");

        // Thu ID từ index.html và các component TS.
        let mut ids: Vec<String> = Vec::new();
        let mut collect = |text: &str| {
            for part in text.split("data-lang-id=").skip(1) {
                let part = part.trim_start();
                let Some(rest) = part.strip_prefix(['"', '\'']) else {
                    continue;
                };
                if let Some(end) = rest.find(['"', '\'']) {
                    let id = &rest[..end];
                    if !id.is_empty() && !id.contains("${") {
                        ids.push(id.to_string());
                    }
                }
            }
        };
        if let Ok(html) = fs::read_to_string(root.join("frontend/index.html")) {
            collect(&html);
        }
        let mut stack = vec![root.join("frontend/src")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.filter_map(Result::ok) {
                let p = entry.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if p.is_dir() {
                    stack.push(p);
                } else if name.ends_with(".ts") && !name.ends_with(".test.ts") {
                    // Bỏ file test: chúng cố ý chứa ID giả để kiểm tra nhánh
                    // "thiếu bản dịch", không phải ID thật của UI.
                    if let Ok(text) = fs::read_to_string(&p) {
                        collect(&text);
                    }
                }
            }
        }
        ids.sort();
        ids.dedup();
        assert!(!ids.is_empty(), "không thu được data-lang-id nào");

        for code in scan_lang_codes() {
            let dict = read_lang_content(&code).expect("đọc file ngôn ngữ");
            let obj = dict.as_object().expect("từ điển phải là object");
            let thieu: Vec<&String> = ids.iter().filter(|id| !obj.contains_key(*id)).collect();
            assert!(
                thieu.is_empty(),
                "langs/{code}.json thiếu {} ID mà UI đang dùng: {:?}",
                thieu.len(),
                thieu
            );
        }
    }

    /// Lệnh trần trả trực tiếp, không bọc bao thư.
    #[test]
    fn lenh_tran_tra_truc_tiep() {
        let langs = get_available_langs().expect("list langs");
        let mut sorted = langs.clone();
        sorted.sort();
        assert_eq!(langs, sorted);
        assert!(get_lang_content("..".to_string()).is_err());
    }
}
