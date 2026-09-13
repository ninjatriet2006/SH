/*
[INTEGRITY NOTES]
 - Mục đích: Cài đặt của GUI (ngôn ngữ, theme, font) — lưu tại
   `~/.config/opencode-manager/settings.json`.
- Trách nhiệm: Đọc/ghi cài đặt, TỰ CHỮA khi mã ngôn ngữ đã lưu không còn file
  tương ứng. Không hardcode "vi" — chọn file đầu tiên thực có trong `langs/`.
- Tương tác: frontend `store/useSettingsStore.ts`, api::lang.

 Tách quyền sở hữu: OpenCode giữ cấu hình chạy của nó trong
 `~/.config/opencode/`; Manager giữ trạng thái UI riêng trong
 `~/.config/opencode-manager/`. Migration chỉ đọc vị trí cũ một lần, không
 di chuyển bất kỳ file nào thuộc OpenCode.
*/

use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Mã ngôn ngữ. Rỗng = "chưa chọn / không còn hợp lệ" → frontend hiện ID.
    #[serde(default)]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme_id: String,
    #[serde(default = "default_font")]
    pub font_id: String,
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_font() -> String {
    crate::api::font::DEFAULT_FONT_ID.to_string()
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            // Không hardcode "vi": lấy file ngôn ngữ đầu tiên thực có.
            language: crate::api::lang::first_available_lang().unwrap_or_default(),
            theme_id: default_theme(),
            font_id: default_font(),
        }
    }
}

/// Vị trí MỚI: `${XDG_CONFIG_HOME:-$HOME/.config}/opencode-manager/settings.json` (dữ liệu riêng của
/// manager, tách khỏi thư mục của opencode; đổi tên manager_gui.json →
/// settings.json cho thống nhất với vai trò của nó).
fn settings_path() -> PathBuf {
    manager_config_dir().join("settings.json")
}

fn manager_config_dir() -> PathBuf {
    if std::env::var_os("OPENCODE_TEST_HOME").is_none() {
        if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").filter(|path| !path.is_empty()) {
            return PathBuf::from(config_home).join("opencode-manager");
        }
    }
    opencode_manager::storage::manager_config_dir()
}

/// Vị trí CŨ — chỉ dùng cho migration.
fn legacy_settings_path() -> PathBuf {
    let home = opencode_manager::config::get_home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".config").join("opencode").join("manager_gui.json")
}

/// Migration sang vị trí mới (không xoá dữ liệu: bản cũ thành .legacy).
fn migrate_settings_file() {
    opencode_manager::storage::migrate_legacy(&legacy_settings_path(), &settings_path());
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_gui_settings(request: Req<Empty>) -> IpcResult<GuiSettings> {
    let (request_id, _) = request.validate()?;
    migrate_settings_file();
    read_gui_settings_from_path(&settings_path())
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

fn read_gui_settings_from_path(path: &Path) -> Result<GuiSettings, String> {
    let mut settings = if let Some(text) = opencode_manager::storage::read_if_exists(path) {
        serde_json::from_str::<GuiSettings>(&text).unwrap_or_else(|e| {
            eprintln!("[settings] settings.json hỏng, dùng mặc định: {e}");
            GuiSettings::default()
        })
    } else {
        GuiSettings::default()
    };

    // Tự chữa mã ngôn ngữ không còn file tương ứng (đổi tên/xoá file, hoặc mã
    // rác). Lấy file đầu tiên có thật; hết file thì để rỗng để UI hiện ID.
    let available = crate::api::lang::scan_lang_codes();
    if settings.language.is_empty() || !available.contains(&settings.language) {
        if !settings.language.is_empty() {
            eprintln!(
                "[settings] ngôn ngữ '{}' không có trong langs/ {:?}, chuyển sang lựa chọn đầu tiên",
                settings.language, available
            );
        }
        settings.language = available.first().cloned().unwrap_or_default();
    }

    if !crate::api::font::font_exists(&settings.font_id) {
        settings.font_id = default_font();
    }

    Ok(settings)
}

#[derive(Deserialize)]
pub struct SaveGuiSettingsRequest {
    pub language: String,
    pub theme_id: String,
    pub font_id: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_gui_settings(request: Req<SaveGuiSettingsRequest>) -> IpcResult<()> {
    let (request_id, payload) = request.validate()?;
    save_gui_settings_to_path(&settings_path(), payload.language, payload.theme_id, payload.font_id)
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

fn save_gui_settings_to_path(path: &Path, language: String, theme_id: String, font_id: String) -> Result<(), String> {
    if language.trim().is_empty() {
        return Err("Ngôn ngữ không được để trống".to_string());
    }
    if theme_id.trim().is_empty() {
        return Err("Theme không được để trống".to_string());
    }
    if !crate::api::font::font_exists(&font_id) {
        return Err(format!("Font '{}' không có trong fonts/", font_id));
    }

    // Chỉ nhận ngôn ngữ thực sự có file — chặn ghi vào settings một mã không
    // tồn tại, nguyên nhân của lỗi "mọi lần mở sau đều hiện raw key".
    let available = crate::api::lang::scan_lang_codes();
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

    let settings = GuiSettings {
        language,
        theme_id,
        font_id,
    };
    // Backup xoay vòng + ghi atomic (xem opencode_manager::storage).
    opencode_manager::storage::backup_rotate(path, opencode_manager::storage::BACKUP_KEEP);
    let text = serde_json::to_string_pretty(&settings).map_err(|e| format!("Lỗi chuyển đổi cài đặt: {e}"))?;
    opencode_manager::storage::atomic_write(path, &text).map_err(|e| format!("Lỗi ghi file cài đặt: {e}"))?;
    Ok(())
}

/// Đường dẫn các file cấu hình — hiển thị ở trang Cài đặt để người dùng biết
/// app đang đọc/ghi vào đâu (khi họ sửa file tay hoặc chạy song song TUI).
#[derive(Debug, Clone, Serialize)]
pub struct ConfigPaths {
    /// Thư mục OpenCode sở hữu: config runtime, không bị Manager di chuyển.
    pub opencode_config_dir: String,
    /// Thư mục Manager sở hữu: settings, CKey profiles và arbiter history.
    pub manager_config_dir: String,
    pub opencode_json: String,
    pub auth_json: String,
    pub ckey_json: String,
    pub arbiter_json: String,
    pub settings_json: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_config_paths(request: Req<Empty>) -> IpcResult<ConfigPaths> {
    let (request_id, _) = request.validate()?;
    Ok(respond(
        request_id,
        ConfigPaths {
            opencode_config_dir: opencode_manager::config::OpencodeConfig::file_path()
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            manager_config_dir: manager_config_dir().display().to_string(),
            opencode_json: opencode_manager::config::OpencodeConfig::file_path()
                .display()
                .to_string(),
            auth_json: opencode_manager::config::AuthEntry::file_path().display().to_string(),
            ckey_json: opencode_manager::config::CkeyConfig::file_path().display().to_string(),
            arbiter_json: opencode_manager::arbiter::ArbiterConfig::file_path()
                .display()
                .to_string(),
            settings_json: settings_path().display().to_string(),
        },
    ))
}

#[derive(Deserialize)]
pub struct OpenExternalUrlRequest {
    pub url: String,
}

/// Mở một URL https trong trình duyệt/người xem ngoài của hệ điều hành.
///
/// `window.open` của webview bị Tauri v2 chặn mặc định (đi hướng trang). Tự làm
/// command thay vì thêm plugin-opener: chỉ chấp nhận `https://` để không thể
/// dùng URL lạ chạy chương trình tuỳ ý; `spawn` truyền arg trực tiếp (không
/// qua shell) nên không tiêm lệnh được.
#[tauri::command(rename_all = "snake_case")]
pub fn open_external_url(request: Req<OpenExternalUrlRequest>) -> IpcResult<()> {
    let (request_id, payload) = request.validate()?;
    open_external_url_inner(payload.url)
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

fn open_external_url_inner(url: String) -> Result<(), String> {
    let url = url.trim();
    if !url.starts_with("https://") || url.contains(|c: char| c.is_whitespace()) {
        return Err("Chỉ mở được đường dẫn https:// hợp lệ.".to_string());
    }

    #[cfg(target_os = "windows")]
    let spawn = || std::process::Command::new("cmd").args(["/c", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let spawn = || std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let spawn = || std::process::Command::new("xdg-open").arg(url).spawn();

    spawn()
        .map(|_| ())
        .map_err(|e| format!("Không mở được trình duyệt: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_dinh_khong_hardcode_ngon_ngu() {
        let available = crate::api::lang::scan_lang_codes();
        let d = GuiSettings::default();
        if available.is_empty() {
            assert!(d.language.is_empty());
        } else {
            assert_eq!(d.language, available[0], "phải lấy file đầu tiên đã sắp");
        }
    }

    #[test]
    fn tu_choi_luu_ngon_ngu_khong_ton_tai() {
        let err = save_gui_settings_to_path(
            &settings_path(),
            "khong_ton_tai_9999".into(),
            "default".into(),
            "default".into(),
        )
        .expect_err("phải từ chối mã không có file");
        assert!(err.contains("không có trong langs/"), "lỗi: {err}");
    }

    #[test]
    fn save_then_fresh_read_giu_nguyen_language_theme_font() {
        let unique = format!(
            "opencode-manager-settings-restart-{}-{:?}-{}",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock phải sau UNIX epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique).join("settings.json");

        save_gui_settings_to_path(&path, "vi".into(), "red_blood".into(), "dejavusans".into())
            .expect("lưu OpenCode GUI settings vào temp path");
        let actual = read_gui_settings_from_path(&path).expect("fresh-read OpenCode GUI settings từ temp path");

        assert_eq!(actual.language, "vi");
        assert_eq!(actual.theme_id, "red_blood");
        assert_eq!(actual.font_id, "dejavusans");
        assert!(path.starts_with(std::env::temp_dir()));
    }

    /// Migration vị trí: manager_gui.json cũ → settings.json mới (không xoá
    /// dữ liệu — bản cũ thành .legacy); cài đặt đọc đúng từ vị trí mới.
    #[test]
    fn migration_vi_tri_settings_khong_xoa_du_lieu() {
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let home = crate::test_support::isolate_home("settings_migration");

        // File cũ (vị trí legacy) với cài đặt thật.
        let old = legacy_settings_path();
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, r#"{"language":"vi","theme_id":"red_blood"}"#).unwrap();

        // Đọc cài đặt → migration tự chạy, dữ liệu còn nguyên vẹn.
        migrate_settings_file();
        let settings = read_gui_settings_from_path(&settings_path()).unwrap();
        assert_eq!(settings.language, "vi", "ngôn ngữ phải sống sót qua migration");
        assert_eq!(settings.theme_id, "red_blood");
        assert!(settings_path().exists(), "file phải ở vị trí mới");
        assert!(
            old.with_extension("json.legacy").exists(),
            "dữ liệu cũ phải còn (đổi tên .legacy, không xoá)"
        );

        // Lưu lại → atomic + backup xoay vòng xuất hiện ở vị trí MỚI.
        save_gui_settings_to_path(&settings_path(), "vi".into(), "red_blood".into(), "default".into()).unwrap();
        assert!(settings_path().exists());
        let baks = std::fs::read_dir(settings_path().parent().unwrap())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_str().map(|n| n.contains(".bak_")).unwrap_or(false))
            .count();
        assert_eq!(baks, 1, "một backup sau lần lưu đầu (có file cũ)");

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn config_paths_phan_tach_opencode_va_manager() {
        let paths = get_config_paths(Req {
            schema_version: crate::ipc::SCHEMA_VERSION,
            request_id: None,
            payload: Empty {},
        })
        .unwrap()
        .data;

        assert!(paths.opencode_config_dir.ends_with(".config/opencode"));
        assert!(paths.manager_config_dir.ends_with(".config/opencode-manager"));
        assert_ne!(paths.opencode_config_dir, paths.manager_config_dir);
        assert!(paths.opencode_json.starts_with(&paths.opencode_config_dir));
        assert!(paths.ckey_json.starts_with(&paths.manager_config_dir));
        assert!(paths.arbiter_json.starts_with(&paths.manager_config_dir));
        assert!(paths.settings_json.starts_with(&paths.manager_config_dir));
    }
}
