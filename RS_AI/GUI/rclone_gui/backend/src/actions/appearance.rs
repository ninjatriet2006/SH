/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — THAO TÁC đọc thư mục tài nguyên giao diện
  (`langs/`, `themes/`, `fonts/`) để liệt kê + đọc nội dung.
- Trách nhiệm: chỉ đọc file/thư mục tài nguyên (dời từ 3 loader cũ ở `api`);
  KHÔNG giữ lựa chọn người dùng (việc đó ở `settings::appearance`).
- Tương tác: `api::appearance_manager` gọi xuống; nguồn thư mục do
  `settings::appearance` quyết (rỗng = `core::resources` mặc định).

UNIVERSAL: mỗi loại có 2 tầng hàm:
  - `*_in(dir)` — LÕI thuần, quét đúng thư mục cho sẵn (test ẤN ĐỊNH path).
  - `*()`       — VỎ, giải nguồn từ settings (điền) hoặc mặc định (rỗng) rồi gọi lõi.
*/

use crate::actions::types::{FontInfo, ThemeInfo};
use crate::core::resources::resource_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// UNIVERSAL: giải thư mục nguồn — settings điền thì dùng, rỗng thì mặc định
/// (`resource_dir(name)`); lỗi đọc settings cũng rớt về mặc định (không chặn UI).
fn resolve_source_dir(name: &str) -> PathBuf {
    let configured = crate::settings::appearance::load_appearance()
        .map(|a| match name {
            "langs" => a.langs_dir,
            "themes" => a.themes_dir,
            "fonts" => a.fonts_dir,
            // UNIVERSAL: tên nhóm lạ (không phải 3 loại) — cảnh báo rồi rớt mặc định.
            other => {
                crate::core::debug::warn(None, "appearance/resolve_source_dir", format!("nhóm tài nguyên lạ '{other}'"));
                String::new()
            }
        })
        .unwrap_or_default();
    if configured.trim().is_empty() {
        resource_dir(name)
    } else {
        PathBuf::from(configured)
    }
}

// ---------------- Ngôn ngữ (langs/) ----------------

/// LÕI: quét mã ngôn ngữ từ tên file `*.json` trong `dir`, ĐÃ SẮP XẾP.
/// `read_dir` không bảo đảm thứ tự nên không sắp thì "file đầu tiên" (fallback)
/// sẽ khác nhau giữa các máy, lỗi rất khó tái hiện.
pub fn scan_lang_codes_in(dir: &Path) -> Vec<String> {
    let mut codes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
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

/// VỎ: quét langs theo nguồn từ settings (rỗng = mặc định).
pub fn scan_lang_codes() -> Vec<String> {
    scan_lang_codes_in(&resolve_source_dir("langs"))
}

/// LÕI: đọc nội dung một file từ điển trong `dir`. Chặn path traversal:
/// `lang_code` ghép trực tiếp vào đường dẫn nên giới hạn ký tự.
pub fn read_lang_content_in(dir: &Path, lang_code: &str) -> Result<serde_json::Value, String> {
    if lang_code.is_empty()
        || !lang_code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Mã ngôn ngữ không hợp lệ: {}", lang_code));
    }

    let path = dir.join(format!("{}.json", lang_code));
    if !path.is_file() {
        return Err(format!("Không tìm thấy ngôn ngữ: {}", lang_code));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Lỗi đọc file ngôn ngữ {}: {}", lang_code, e))?;
    serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON ngôn ngữ {}: {}", lang_code, e))
}

/// VỎ: đọc nội dung ngôn ngữ theo nguồn từ settings (rỗng = mặc định).
pub fn read_lang_content(lang_code: &str) -> Result<serde_json::Value, String> {
    read_lang_content_in(&resolve_source_dir("langs"), lang_code)
}

// ---------------- Theme (themes/) ----------------

/// UNIVERSAL: id tài nguyên chỉ nhận chữ-số + `-`/`_` (chặn tên rác/traversal).
fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

/// LÕI: quét theme JSON có thật trong `dir`, đã sắp A→Z.
/// KHÔNG nhồi phần tử mặc định — "none/mặc định" là việc của frontend. Không có
/// file → vec rỗng. Bỏ id `default` nếu file lỡ đặt (dành cho "none" của FE).
pub fn scan_themes_in(dir: &Path) -> Vec<ThemeInfo> {
    let mut themes: Vec<ThemeInfo> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
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

/// VỎ: quét themes theo nguồn từ settings (rỗng = mặc định).
pub fn scan_themes() -> Vec<ThemeInfo> {
    scan_themes_in(&resolve_source_dir("themes"))
}

// ---------------- Font (fonts/) ----------------

/// UNIVERSAL: id font = tên file rút gọn chỉ còn chữ-số thường (khớp CSS id).
fn font_id(stem: &str) -> String {
    stem.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

/// LÕI: quét font có thật trong `dir`, đã sắp A→Z.
/// KHÔNG nhồi phần tử mặc định (system-ui) — "none/mặc định" là việc của
/// frontend. Không có file → vec rỗng. Bỏ id rỗng hoặc `default`.
pub fn scan_fonts_in(dir: &Path) -> Vec<FontInfo> {
    let mut fonts: Vec<FontInfo> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
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

/// VỎ: quét fonts theo nguồn từ settings (rỗng = mặc định).
pub fn scan_fonts() -> Vec<FontInfo> {
    scan_fonts_in(&resolve_source_dir("fonts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tạo thư mục tạm riêng cho từng test (ấn định path để test từ code).
    fn fixture_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rclone_gui_appearance_{}_{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("tạo fixture dir");
        dir
    }

    #[test]
    fn langs_scan_sorts_and_ignores_non_json() {
        let dir = fixture_dir("langs_scan");
        fs::write(dir.join("vi.json"), b"{}").unwrap();
        fs::write(dir.join("en.json"), b"{}").unwrap();
        fs::write(dir.join("readme.txt"), b"skip me").unwrap();
        let codes = scan_lang_codes_in(&dir);
        assert_eq!(codes, vec!["en".to_string(), "vi".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_lang_content_reads_and_rejects_junk() {
        let dir = fixture_dir("langs_read");
        fs::write(dir.join("vi.json"), r#"{"hello":"xin chào"}"#.as_bytes()).unwrap();
        let v = read_lang_content_in(&dir, "vi").expect("doc vi");
        assert_eq!(v.get("hello").and_then(|x| x.as_str()), Some("xin chào"));
        // Mã rác / traversal bị chặn trước khi chạm đĩa.
        for bad in ["../../etc/passwd", "..", "vi/../en", "vi.json", ""] {
            assert!(read_lang_content_in(&dir, bad).is_err(), "mã '{bad}' phải bị từ chối");
        }
        // Mã hợp lệ nhưng không có file → lỗi rõ.
        assert!(read_lang_content_in(&dir, "khong_co").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn themes_scan_skips_broken_and_default_and_sorts() {
        let dir = fixture_dir("themes_scan");
        fs::write(dir.join("neon.json"), br#"{"id":"neon","name":"Neon","variables":{}}"#).unwrap();
        fs::write(dir.join("amber.json"), br#"{"id":"amber","name":"Amber","variables":{}}"#).unwrap();
        // JSON hỏng → bỏ qua (warn), không làm sập danh sách.
        fs::write(dir.join("broken.json"), b"{ khong-phai-json").unwrap();
        // File tự xưng id "default" → bỏ (dành cho "none" của frontend).
        fs::write(dir.join("d.json"), br#"{"id":"default","name":"X","variables":{}}"#).unwrap();
        let themes = scan_themes_in(&dir);
        let ids: Vec<_> = themes.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["amber", "neon"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fonts_scan_filters_ext_and_builds_id() {
        let dir = fixture_dir("fonts_scan");
        fs::write(dir.join("DejaVuSans.ttf"), b"x").unwrap();
        fs::write(dir.join("Roboto.otf"), b"x").unwrap();
        fs::write(dir.join("notes.md"), b"skip me").unwrap();
        let fonts = scan_fonts_in(&dir);
        let ids: Vec<_> = fonts.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec!["dejavusans", "roboto"]);
        // src_path trỏ đúng file font.
        assert!(fonts.iter().all(|f| f.src_path.is_some()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scans_never_emit_default_identifier() {
        // UNIVERSAL: backend KHÔNG được nhả định danh "default" — đó là "none" FE giữ.
        let dir = fixture_dir("no_default");
        fs::write(dir.join("default.ttf"), b"x").unwrap();
        fs::write(dir.join("default.json"), br#"{"id":"default","name":"X","variables":{}}"#).unwrap();
        assert!(scan_fonts_in(&dir).iter().all(|f| f.id != "default"));
        assert!(scan_themes_in(&dir).iter().all(|t| t.id != "default"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn font_id_strips_to_lower_alnum() {
        assert_eq!(font_id("DejaVuSans"), "dejavusans");
        assert_eq!(font_id("Noto-Sans_2"), "notosans2");
    }

    /// INTEGRATION: mọi ID mà UI tham chiếu (`data-lang-id`) phải có trong MỌI
    /// file ngôn ngữ THẬT — thiếu là chỗ đó hiện raw ID trên giao diện.
    /// Dùng vỏ `scan_lang_codes()`/`read_lang_content()` (nguồn mặc định).
    #[test]
    fn moi_file_ngon_ngu_du_id_ma_ui_dung() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app root");

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

    /// KIỂM THỰC DỤNG trên thư mục release THẬT: IN RA kết quả đọc được (không
    /// chỉ pass/fail) để người/AI đối chiếu với đĩa. Chạy kèm `-- --nocapture`.
    /// Ấn định vị trí bằng path TƯƠNG ĐỐI từ repo (không ghi cứng /home).
    /// Thiếu thư mục trên máy lạ → bỏ qua, không đỏ.
    #[test]
    fn scan_release_dirs_prints_real_results() {
        // backend/.. = rclone_gui, /../.. = GUI, /../../.. = RS_AI (gốc repo).
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .expect("RS_AI root");
        let base = repo_root.join("release").join("rclone_gui");
        let langs_dir = base.join("langs");
        let themes_dir = base.join("themes");
        let fonts_dir = base.join("fonts");
        if !langs_dir.is_dir() && !themes_dir.is_dir() && !fonts_dir.is_dir() {
            println!("SKIP: máy này không có thư mục release thật");
            return;
        }

        let codes = scan_lang_codes_in(&langs_dir);
        println!("[REAL langs] {} -> {:?}", langs_dir.display(), codes);
        // UNIVERSAL: kiểm đối chiếu — tra KHOÁ CỤ THỂ, in GIÁ TRỊ thật ra để
        // người/AI đọc file gốc đối chiếu (không đếm số khóa — đếm không nói
        // được nội dung đúng hay sai).
        const SPOT_KEYS: [&str; 3] = ["app_title", "nav_explorer", "menu_file_quit"];
        for c in &codes {
            match read_lang_content_in(&langs_dir, c) {
                Ok(v) => {
                    for k in SPOT_KEYS {
                        let got = v.get(k).and_then(|x| x.as_str()).unwrap_or("<THIEU>");
                        println!("  - {c}.json[{k}] -> {got}");
                    }
                }
                Err(e) => println!("  - {c}.json LOI: {e}"),
            }
        }

        let themes = scan_themes_in(&themes_dir);
        let theme_ids: Vec<_> = themes.iter().map(|t| t.id.as_str()).collect();
        println!("[REAL themes] {} -> {:?}", themes_dir.display(), theme_ids);

        let fonts = scan_fonts_in(&fonts_dir);
        let font_ids: Vec<_> = fonts.iter().map(|f| f.id.as_str()).collect();
        println!("[REAL fonts] {} -> {:?}", fonts_dir.display(), font_ids);

        // Đối chiếu nội dung thật trên máy đóng gói: langs có vi.json (đọc
        // đúng giá trị từng khoá đối chiếu); themes chỉ có .css (→ rỗng);
        // fonts chưa có thư mục (→ rỗng).
        assert_eq!(codes, vec!["vi".to_string()], "langs release phai co dung vi");
        let vi = read_lang_content_in(&langs_dir, "vi").expect("doc duoc vi.json that");
        assert_eq!(
            vi.get("app_title").and_then(|x| x.as_str()),
            Some("Rclone Manager"),
            "app_title phai doc dung gia tri that"
        );
        assert_eq!(
            vi.get("nav_explorer").and_then(|x| x.as_str()),
            Some("Explorer")
        );
        assert_eq!(
            vi.get("menu_file_quit").and_then(|x| x.as_str()),
            Some("Thoát")
        );
        assert!(themes.is_empty(), "themes release khong co .json");
        assert!(fonts.is_empty(), "fonts release chua co thu muc");
    }

    /// KIỂM THỰC DỤNG theme trên tài nguyên THẬT của repo (`GUI/rclone_gui/themes/`
    /// có `amber.json` thật): IN RA id/tên/giá trị biến để đối chiếu với file gốc.
    /// Ấn định bằng path tương đối từ repo; thiếu thư mục → bỏ qua, không đỏ.
    #[test]
    fn scan_repo_themes_prints_real_values() {
        let app_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app root");
        let dir = app_root.join("themes");
        if !dir.is_dir() {
            println!("SKIP: máy này không có thư mục themes của repo");
            return;
        }
        let themes = scan_themes_in(&dir);
        let ids: Vec<_> = themes.iter().map(|t| t.id.as_str()).collect();
        println!("[REAL repo-themes] {} -> {:?}", dir.display(), ids);
        for t in &themes {
            let cyan = t.variables.get("colors-neon-cyan").map(|s| s.as_str()).unwrap_or("<THIEU>");
            println!("  - id='{}' name='{}' colors-neon-cyan={}", t.id, t.name, cyan);
        }
        // Đối chiếu file gốc GUI/rclone_gui/themes/amber.json đã đọc trực tiếp.
        assert_eq!(ids, vec!["amber"], "repo themes phai co dung amber");
        let amber = &themes[0];
        assert_eq!(amber.name, "Amber Signal");
        assert_eq!(
            amber.variables.get("colors-neon-cyan").map(|s| s.as_str()),
            Some("#ffb800")
        );
    }

    /// KIỂM THỰC DỤNG font trên tài nguyên THẬT của repo (`GUI/rclone_gui/fonts/`
    /// có `DejaVuSans.ttf` thật, kèm `LICENSE.txt` phải bị bỏ qua): IN RA
    /// id/tên/family để đối chiếu với tên file trên đĩa.
    #[test]
    fn scan_repo_fonts_prints_real_values() {
        let app_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("app root");
        let dir = app_root.join("fonts");
        if !dir.is_dir() {
            println!("SKIP: máy này không có thư mục fonts của repo");
            return;
        }
        let fonts = scan_fonts_in(&dir);
        let ids: Vec<_> = fonts.iter().map(|f| f.id.as_str()).collect();
        println!("[REAL repo-fonts] {} -> {:?}", dir.display(), ids);
        for f in &fonts {
            println!(
                "  - id='{}' name='{}' family='{}' src='{}'",
                f.id,
                f.name,
                f.family,
                f.src_path.as_deref().unwrap_or("<THIEU>")
            );
        }
        // Đối chiếu đĩa thật: DejaVuSans.ttf → id dejavusans; LICENSE.txt bị bỏ.
        assert_eq!(ids, vec!["dejavusans"], "repo fonts phai co dung dejavusans");
        let d = &fonts[0];
        assert_eq!((d.name.as_str(), d.family.as_str()), ("DejaVuSans", "DejaVuSans"));
        assert!(
            d.src_path.as_deref().is_some_and(|p| p.ends_with("DejaVuSans.ttf")),
            "src_path phai tro dung file that"
        );
    }
}
