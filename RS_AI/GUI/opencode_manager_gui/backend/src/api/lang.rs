/*
[INTEGRITY NOTES]
Mục đích: API đa ngôn ngữ — liệt kê và đọc file từ điển trong `langs/`.
Trách nhiệm: Tự nhận diện file `*.json` có thật thay vì gắn cứng một mã ngôn
  ngữ. Không tìm được thì trả lỗi để frontend hiện raw ID (lộ lỗi rõ ràng).
Các module tương tác: frontend `store/useSettingsStore.ts`, core::resources.
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
pub fn scan_lang_codes() -> Vec<String> {
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
pub fn first_available_lang() -> Option<String> {
    scan_lang_codes().into_iter().next()
}

/// `rename_all = "snake_case"` là BẮT BUỘC: Tauri v2 mặc định đổi tên tham số
/// sang camelCase (`lang_code` → `langCode`), trong khi bridge gửi snake_case →
/// IPC báo "missing required key" và command không bao giờ chạy.
#[tauri::command(rename_all = "snake_case")]
pub fn get_available_langs() -> Result<Vec<String>, String> {
    Ok(scan_lang_codes())
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_lang_content(lang_code: String) -> Result<serde_json::Value, String> {
    // Chặn path traversal: `lang_code` ghép trực tiếp vào đường dẫn.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danh_sach_duoc_sap_xep() {
        let codes = scan_lang_codes();
        let mut sorted = codes.clone();
        sorted.sort();
        assert_eq!(codes, sorted);
    }

    #[test]
    fn fallback_lay_tu_file_co_that() {
        let codes = scan_lang_codes();
        match first_available_lang() {
            Some(code) => {
                assert!(codes.contains(&code), "fallback '{code}' không có file");
                let dict = get_lang_content(code.clone()).unwrap_or_else(|e| panic!("đọc '{code}' thất bại: {e}"));
                assert!(
                    dict.as_object().is_some_and(|m| !m.is_empty()),
                    "'{code}' phải là object không rỗng"
                );
            }
            None => assert!(codes.is_empty(), "có file nhưng fallback trả None"),
        }
    }

    #[test]
    fn chan_ma_ngon_ngu_doc_hai() {
        for bad in ["../../etc/passwd", "..", "vi/../en", "vi.json", ""] {
            assert!(get_lang_content(bad.to_string()).is_err(), "mã '{bad}' phải bị từ chối");
        }
    }

    /// Mọi key mà UI dùng qua `t('...')` phải có trong MỌI file ngôn ngữ.
    /// Thiếu là chỗ đó hiện raw key trên giao diện.
    #[test]
    fn moi_file_ngon_ngu_du_key_ma_ui_dung() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app root");

        // Thu key từ các lời gọi t('a.b') trong frontend.
        let mut keys: Vec<String> = Vec::new();
        let mut stack = vec![root.join("frontend/src")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.filter_map(Result::ok) {
                let p = entry.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if p.is_dir() {
                    stack.push(p);
                } else if (name.ends_with(".ts") || name.ends_with(".tsx")) && !name.ends_with(".test.ts") {
                    let Ok(text) = fs::read_to_string(&p) else { continue };
                    for part in text.split("t('").skip(1) {
                        if let Some(end) = part.find('\'') {
                            let key = &part[..end];
                            // Chỉ nhận key có dạng `nhóm.tên`: mỗi đoạn phải
                            // không rỗng. Bỏ nhiễu như `'...'` hoặc `'.'` (đến
                            // từ `t(\`...\`)`/chuỗi khác lọt vào regex đơn giản).
                            let segs: Vec<&str> = key.split('.').collect();
                            let hop_le = segs.len() >= 2
                                && segs
                                    .iter()
                                    .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
                            if hop_le {
                                keys.push(key.to_string());
                            }
                        }
                    }
                }
            }
        }
        keys.sort();
        keys.dedup();
        assert!(!keys.is_empty(), "không thu được key i18n nào");

        for code in scan_lang_codes() {
            let dict = get_lang_content(code.clone()).expect("đọc file ngôn ngữ");
            let thieu: Vec<&String> = keys
                .iter()
                .filter(|k| {
                    // Duyệt theo đường dẫn `a.b.c`.
                    let mut cur = &dict;
                    for seg in k.split('.') {
                        match cur.get(seg) {
                            Some(next) => cur = next,
                            None => return true,
                        }
                    }
                    !cur.is_string()
                })
                .collect();
            assert!(
                thieu.is_empty(),
                "langs/{code}.json thiếu {} key mà UI đang dùng: {:?}",
                thieu.len(),
                thieu
            );
        }
    }
}
