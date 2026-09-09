use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// Mã ngôn ngữ. Rỗng = "chưa chọn / không còn hợp lệ" → frontend hiện ID.
    /// Không mặc định thành "vi" để app không phụ thuộc một file cụ thể.
    #[serde(default)]
    pub language: String,
    pub timezone: String,
    #[serde(default = "default_id")]
    pub theme_id: String,
    #[serde(default = "default_id")]
    pub font_id: String,
}

/// `"default"` ở đây là **quy ước rỗng của theme/font**, không phải tên file:
/// frontend hiểu là "dùng giá trị trong CSS gốc", nên không cần `default.json`
/// tồn tại. Với ngôn ngữ thì không có tương đương — thiếu file là hiện ID.
fn default_id() -> String {
    "default".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Không hardcode "vi": chọn file ngôn ngữ đầu tiên thực có trong
            // `langs/`. Thư mục rỗng → để trống, UI sẽ hiện ID (lộ lỗi rõ ràng).
            language: crate::lang_api::first_available_lang().unwrap_or_default(),
            timezone: "Asia/Ho_Chi_Minh".to_string(),
            theme_id: default_id(),
            font_id: default_id(),
        }
    }
}

fn canonical_settings_path() -> PathBuf {
    crate::storage::config_file_path("settings.json")
}

fn mirror_settings_path() -> PathBuf {
    crate::storage::storage_file_path("settings.json")
}

/// Ghi qua file tạm và rename nguyên tử, cùng mức an toàn crash/mất điện như
/// `data.json`. Settings không quan trọng bằng giao dịch nhưng vẫn không nên
/// bị truncate nếu máy tắt đúng lúc lưu.
fn write_settings_file(path: &std::path::Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Đường dẫn settings không hợp lệ: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("Lỗi tạo thư mục settings {}: {e}", parent.display()))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, content).map_err(|e| format!("Lỗi ghi settings tạm: {e}"))?;
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        format!("Lỗi thay thế settings: {e}")
    })
}

/// Canonical nằm trong `~/.config`; một file settings cũ cạnh app được migrate
/// khi đọc lần đầu. Không merge từng field vì canonical là bản người dùng chọn
/// gần nhất và mirror chỉ là bản sao khôi phục/di động.
fn load_settings_text() -> Option<String> {
    let canonical = canonical_settings_path();
    if let Ok(text) = fs::read_to_string(&canonical) {
        return Some(text);
    }
    let mirror = mirror_settings_path();
    let text = fs::read_to_string(&mirror).ok()?;
    if let Err(error) = write_settings_file(&canonical, &text) {
        eprintln!("[settings] không migrate được {}: {error}", canonical.display());
    }
    Some(text)
}

// Đọc cài đặt
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn get_settings() -> Result<Settings, String> {
    let mut settings = match load_settings_text() {
        Some(content) => match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("[settings] settings.json hỏng, dùng mặc định: {e}");
                Settings::default()
            }
        },
        None => Settings::default(),
    };

    // Tự chữa mã ngôn ngữ không còn file tương ứng (đổi tên/xoá file, hoặc mã
    // rác do bản cũ ghi vào). Không rơi cứng về "vi" mà lấy file đầu tiên có
    // thật; hết file thì để rỗng để frontend hiện ID.
    let available = crate::lang_api::scan_lang_codes();
    if !settings.language.is_empty() && !available.contains(&settings.language) {
        eprintln!(
            "[settings] ngôn ngữ '{}' không có trong langs/ ({:?}), chuyển sang lựa chọn đầu tiên",
            settings.language, available
        );
        settings.language = available.first().cloned().unwrap_or_default();
    } else if settings.language.is_empty() {
        settings.language = available.first().cloned().unwrap_or_default();
    }

    Ok(settings)
}

// Lưu cài đặt
// Giữ tên tham số snake_case khớp bridge (xem chú thích ở lang_api.rs).
#[tauri::command(rename_all = "snake_case")]
pub fn save_settings(language: String, timezone: String, theme_id: String, font_id: String) -> Result<(), String> {
    // Từ chối giá trị rỗng — trước đây ghi đè settings.json bằng chuỗi trống
    // khiến theme/font rơi về trạng thái không tồn tại.
    if language.trim().is_empty() {
        return Err("Ngôn ngữ không được để trống".to_string());
    }
    if timezone.trim().is_empty() {
        return Err("Múi giờ không được để trống".to_string());
    }
    if theme_id.trim().is_empty() {
        return Err("Theme không được để trống".to_string());
    }
    if font_id.trim().is_empty() {
        return Err("Font không được để trống".to_string());
    }
    // Chỉ nhận ngôn ngữ thực sự có file — chặn ghi vào settings một mã không
    // tồn tại (bug cũ: settings lưu "5555" từ file test đã xoá, mọi lần mở sau
    // đều hiện raw key mà không rõ vì sao).
    let available = crate::lang_api::scan_lang_codes();
    if !available.contains(&language) {
        return Err(format!(
            "Ngôn ngữ '{}' không có trong langs/. Có sẵn: {}",
            language,
            if available.is_empty() {
                "(không có file ngôn ngữ nào)".to_string()
            } else {
                available.join(", ")
            }
        ));
    }
    // Settings ghi file riêng nhưng vẫn serialize chung để tránh nghẽn IO dồn dập.
    let _store_guard = crate::storage::lock_store();
    let settings = Settings {
        language,
        timezone,
        theme_id,
        font_id,
    };
    let canonical = canonical_settings_path();
    let mirror = mirror_settings_path();

    match serde_json::to_string_pretty(&settings) {
        Ok(json_str) => {
            write_settings_file(&canonical, &json_str)?;
            if mirror != canonical {
                if let Err(error) = write_settings_file(&mirror, &json_str) {
                    eprintln!(
                        "[settings] canonical đã lưu nhưng không mirror được {}: {error}",
                        mirror.display()
                    );
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Lỗi chuyển đổi settings: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Settings::default()` không được hardcode "vi": ngôn ngữ mặc định phải
    /// là một file thực có trong `langs/` (hoặc rỗng khi thư mục trống).
    #[test]
    fn mac_dinh_khong_hardcode_ngon_ngu() {
        let available = crate::lang_api::scan_lang_codes();
        let d = Settings::default();
        if available.is_empty() {
            assert!(d.language.is_empty(), "langs/ rỗng thì language phải rỗng");
        } else {
            assert!(
                available.contains(&d.language),
                "language mặc định '{}' không có file trong langs/ {:?}",
                d.language,
                available
            );
            assert_eq!(d.language, available[0], "phải lấy file đầu tiên theo thứ tự đã sắp");
        }
    }

    /// Không cho ghi mã ngôn ngữ không tồn tại — nguồn gốc bug "settings lưu
    /// 5555 rồi mọi lần mở sau đều hiện raw key".
    #[test]
    fn tu_choi_luu_ngon_ngu_khong_ton_tai() {
        let err = save_settings(
            "khong_ton_tai_9999".to_string(),
            "Asia/Ho_Chi_Minh".to_string(),
            "default".to_string(),
            "default".to_string(),
        )
        .expect_err("phải từ chối mã ngôn ngữ không có file");
        assert!(err.contains("không có trong langs/"), "thông báo lỗi: {err}");
    }

    /// Theme/font mặc định là quy ước `"default"`, KHÔNG yêu cầu file
    /// `themes/default.json` tồn tại (backend không còn tự sinh file đó).
    #[test]
    fn theme_font_mac_dinh_khong_can_file() {
        let d = Settings::default();
        assert_eq!(d.theme_id, "default");
        assert_eq!(d.font_id, "default");
        let themes_dir = crate::storage::resource_dir("themes");
        let _ = std::fs::remove_file(themes_dir.join("__khong_bao_gio_ton_tai.json"));
        // Không assert sự tồn tại của default.json: đó chính là điều cần bỏ.
        assert!(
            crate::theme_api::get_available_themes().is_ok(),
            "liệt kê theme phải chạy được dù không có default.json"
        );
    }
}
