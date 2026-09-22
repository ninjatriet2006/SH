/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — THAO TÁC đọc thư mục tài nguyên giao diện
  (`langs/`, `themes/`, `fonts/`) để liệt kê + đọc nội dung.
- Trách nhiệm: chỉ đọc file/thư mục tài nguyên (dời từ 3 loader cũ ở `api`);
  KHÔNG giữ lựa chọn người dùng (việc đó ở `settings::appearance`).
- Tương tác: `api::appearance_manager` gọi xuống; đọc gốc qua `core::resources`.
*/

use crate::actions::types::{FontInfo, ThemeInfo};
use crate::core::resources::resource_dir;
use std::fs;
use std::path::PathBuf;

// ---------------- Ngôn ngữ (langs/) ----------------

fn langs_dir() -> PathBuf {
    resource_dir("langs")
}

/// Quét mã ngôn ngữ từ tên file `*.json`, ĐÃ SẮP XẾP.
/// `read_dir` không bảo đảm thứ tự nên không sắp thì "file đầu tiên" (fallback)
/// sẽ khác nhau giữa các máy, lỗi rất khó tái hiện.
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

/// Đọc nội dung một file từ điển. Chặn path traversal: `lang_code` ghép trực
/// tiếp vào đường dẫn nên giới hạn ký tự, tránh `../../etc/passwd`.
pub fn read_lang_content(lang_code: &str) -> Result<serde_json::Value, String> {
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

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Lỗi đọc file ngôn ngữ {}: {}", lang_code, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON ngôn ngữ {}: {}", lang_code, e))
}

// ---------------- Theme (themes/) ----------------

/// UNIVERSAL: id tài nguyên chỉ nhận chữ-số + `-`/`_` (chặn tên rác/traversal).
fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

/// Quét `themes/`: CHỈ trả theme từ file JSON có thật, đã sắp A→Z.
/// UNIVERSAL: KHÔNG nhồi phần tử mặc định — "none/mặc định" là việc của frontend
/// (backend không được chứa định danh giao diện). Không có file → vec rỗng.
/// Bỏ qua id `default` nếu file lỡ đặt, để không đụng "none" mà frontend giữ.
pub fn scan_themes() -> Vec<ThemeInfo> {
    let mut themes: Vec<ThemeInfo> = Vec::new();
    if let Ok(entries) = fs::read_dir(resource_dir("themes")) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                // UNIVERSAL: file theme đọc lỗi → warn rồi bỏ qua (đồng bộ với
                // core::resources), tránh theme biến mất khỏi UI không dấu vết.
                Err(e) => {
                    crate::core::debug::warn(None, "appearance/scan_themes", format!("bỏ qua theme đọc lỗi {}: {}", path.display(), e));
                    continue;
                }
            };
            let theme = match serde_json::from_str::<ThemeInfo>(&content) {
                Ok(t) => t,
                // UNIVERSAL: JSON theme hỏng → warn kèm tên file rồi bỏ qua.
                Err(e) => {
                    crate::core::debug::warn(None, "appearance/scan_themes", format!("bỏ qua theme JSON hỏng {}: {}", path.display(), e));
                    continue;
                }
            };
            if safe_id(&theme.id) && theme.id != "default" {
                themes.push(theme);
            }
        }
    }
    themes.sort_by(|a, b| a.id.cmp(&b.id));
    themes
}

// ---------------- Font (fonts/) ----------------

/// UNIVERSAL: id font = tên file rút gọn chỉ còn chữ-số thường (khớp CSS id).
fn font_id(stem: &str) -> String {
    stem.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

/// Quét `fonts/`: CHỈ trả font từ file có thật, đã sắp A→Z.
/// UNIVERSAL: KHÔNG nhồi phần tử mặc định (system-ui) — "none/mặc định" là việc
/// của frontend. Không có file → vec rỗng. Bỏ qua id `default` nếu file lỡ đặt.
pub fn scan_fonts() -> Vec<FontInfo> {
    let mut fonts: Vec<FontInfo> = Vec::new();
    if let Ok(entries) = fs::read_dir(resource_dir("fonts")) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            // UNIVERSAL: chỉ nhận đuôi font; file khác trong thư mục là bình thường, bỏ im lặng.
            if !path.is_file() || !matches!(ext.as_str(), "ttf" | "otf" | "woff" | "woff2") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let id = font_id(stem);
            // UNIVERSAL: bỏ id rỗng (tên toàn ký tự lạ) hoặc `default` (dành cho frontend).
            if id.is_empty() || id == "default" {
                continue;
            }
            fonts.push(FontInfo {
                id,
                name: stem.to_string(),
                family: stem.to_string(),
                src_path: Some(path.to_string_lossy().into_owned()),
            });
        }
    }
    fonts.sort_by(|a, b| a.id.cmp(&b.id));
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langs_codes_are_sorted() {
        // Danh sách phải sắp xếp để fallback ổn định giữa các máy.
        let codes = scan_lang_codes();
        let mut sorted = codes.clone();
        sorted.sort();
        assert_eq!(codes, sorted);
    }

    #[test]
    fn read_lang_rejects_traversal_and_junk() {
        for bad in ["../../etc/passwd", "..", "vi/../en", "vi.json", ""] {
            assert!(read_lang_content(bad).is_err(), "mã '{bad}' phải bị từ chối");
        }
    }

    #[test]
    fn scans_never_emit_default_identifier() {
        // UNIVERSAL: backend KHÔNG được chứa định danh "default" — đó là "none"
        // do frontend giữ. Danh sách chỉ gồm file tài nguyên thật.
        assert!(scan_themes().iter().all(|t| t.id != "default"));
        assert!(scan_fonts().iter().all(|f| f.id != "default"));
    }

    #[test]
    fn theme_and_font_lists_are_sorted() {
        let theme_ids: Vec<_> = scan_themes().into_iter().map(|t| t.id).collect();
        assert!(theme_ids.windows(2).all(|p| p[0] <= p[1]));
        let font_ids: Vec<_> = scan_fonts().into_iter().map(|f| f.id).collect();
        assert!(font_ids.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn font_id_strips_to_lower_alnum() {
        assert_eq!(font_id("DejaVuSans"), "dejavusans");
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
}
