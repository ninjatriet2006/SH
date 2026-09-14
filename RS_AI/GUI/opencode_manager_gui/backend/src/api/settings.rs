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
use std::collections::HashMap;
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
    /// Danh sách provider yêu thích (thứ tự = vị trí hiển thị).
    #[serde(default)]
    pub favorite_providers: Vec<String>,
    /// Model ghim cho mỗi provider: provider_id → model_id.
    #[serde(default)]
    pub pinned_models: HashMap<String, String>,
    /// Provider đang chờ được kiểm tra model tự động khi GUI khởi động.
    #[serde(default)]
    pub tracked_providers: Vec<String>,
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
            favorite_providers: Vec::new(),
            pinned_models: HashMap::new(),
            tracked_providers: Vec::new(),
        }
    }
}

/// Phần sở thích hiển thị provider của `GuiSettings` — tách riêng để frontend
/// chỉ nạp/thao tác khối này khi bấm sao/ghim, không kéo theo language/theme/font.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProviderPreferences {
    /// Provider yêu thích, theo đúng thứ tự hiển thị (dấu ★ lên đầu bảng).
    #[serde(default)]
    pub favorite_providers: Vec<String>,
    /// Model ghim cho mỗi provider: provider_id → model_id.
    #[serde(default)]
    pub pinned_models: HashMap<String, String>,
    /// Provider được người dùng đánh dấu để kiểm tra model khi khởi động.
    #[serde(default)]
    pub tracked_providers: Vec<String>,
}

impl From<&GuiSettings> for ProviderPreferences {
    fn from(settings: &GuiSettings) -> Self {
        Self {
            favorite_providers: settings.favorite_providers.clone(),
            pinned_models: settings.pinned_models.clone(),
            tracked_providers: settings.tracked_providers.clone(),
        }
    }
}

impl From<GuiSettings> for ProviderPreferences {
    fn from(settings: GuiSettings) -> Self {
        Self {
            favorite_providers: settings.favorite_providers,
            pinned_models: settings.pinned_models,
            tracked_providers: settings.tracked_providers,
        }
    }
}

/// Đọc settings, áp một chỉnh sửa lên sở thích, ghi lại (backup + atomic) rồi
/// trả về trạng thái mới. Mutation trả lỗi thì KHÔNG ghi gì cả. Mọi lệnh sửa
/// preferences đều đi qua đây để không thể quên backup/xoay vòng.
fn update_preferences(
    path: &Path,
    mutate: impl FnOnce(&mut GuiSettings) -> Result<(), String>,
) -> Result<ProviderPreferences, String> {
    let mut settings = read_gui_settings_from_path(path)?;
    mutate(&mut settings)?;
    opencode_manager::storage::backup_rotate(path, opencode_manager::storage::BACKUP_KEEP);
    let text = serde_json::to_string_pretty(&settings).map_err(|e| format!("Lỗi chuyển đổi cài đặt: {e}"))?;
    opencode_manager::storage::atomic_write(path, &text).map_err(|e| format!("Lỗi ghi file cài đặt: {e}"))?;
    Ok(ProviderPreferences::from(&settings))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_provider_preferences(request: Req<Empty>) -> IpcResult<ProviderPreferences> {
    let (request_id, _) = request.validate()?;
    migrate_settings_file();
    read_gui_settings_from_path(&settings_path())
        .map(|settings| respond(request_id, ProviderPreferences::from(&settings)))
        .map_err(from_string)
}

#[derive(Deserialize)]
pub struct ToggleFavoriteProviderRequest {
    pub provider_id: String,
}

fn toggle_favorite_inner(path: &Path, provider_id: String) -> Result<ProviderPreferences, String> {
    if provider_id.is_empty() {
        return Err("provider_id không được để trống".to_string());
    }
    update_preferences(path, |settings| {
        match settings.favorite_providers.iter().position(|id| id == &provider_id) {
            // Đã yêu thích → bỏ; chưa → thêm vào CUỐI nhóm sao (nhóm này luôn
            // đứng đầu bảng, thứ tự trong nhóm = thứ tự trong danh sách).
            Some(pos) => {
                settings.favorite_providers.remove(pos);
            }
            None => settings.favorite_providers.push(provider_id.clone()),
        }
        Ok(())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn toggle_favorite_provider(request: Req<ToggleFavoriteProviderRequest>) -> IpcResult<ProviderPreferences> {
    let (request_id, payload) = request.validate()?;
    toggle_favorite_inner(&settings_path(), payload.provider_id.trim().to_string())
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

#[derive(Deserialize)]
pub struct ReorderFavoriteProvidersRequest {
    /// Toàn bộ danh sách yêu thích theo thứ tự MỚI (phải là hoán vị của hiện có).
    pub provider_ids: Vec<String>,
}

fn reorder_favorites_inner(path: &Path, provider_ids: Vec<String>) -> Result<ProviderPreferences, String> {
    let ids: Vec<String> = provider_ids.into_iter().map(|s| s.trim().to_string()).collect();
    update_preferences(path, |settings| {
        // Chỉ chấp nhận hoán vị ĐÚNG của danh sách hiện có: chống ghi mất
        // provider khi frontend gửi danh sách lệch (stale sau khi xoá/bỏ sao).
        let mut expected = settings.favorite_providers.clone();
        let mut actual = ids.clone();
        expected.sort_unstable();
        actual.sort_unstable();
        if expected != actual {
            return Err("Danh sách đổi thứ tự không trùng khớp các provider đang được yêu thích".to_string());
        }
        settings.favorite_providers = ids;
        Ok(())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn reorder_favorite_providers(request: Req<ReorderFavoriteProvidersRequest>) -> IpcResult<ProviderPreferences> {
    let (request_id, payload) = request.validate()?;
    reorder_favorites_inner(&settings_path(), payload.provider_ids)
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

#[derive(Deserialize)]
pub struct SetPinnedModelRequest {
    pub provider_id: String,
    /// Model ghim; CHUỖI RỖNG = bỏ ghim.
    pub model_id: String,
}

fn set_pinned_model_inner(path: &Path, provider_id: String, model_id: String) -> Result<ProviderPreferences, String> {
    if provider_id.is_empty() {
        return Err("provider_id không được để trống".to_string());
    }
    update_preferences(path, |settings| {
        if model_id.is_empty() {
            settings.pinned_models.remove(&provider_id);
        } else {
            settings.pinned_models.insert(provider_id, model_id.clone());
        }
        Ok(())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_pinned_model(request: Req<SetPinnedModelRequest>) -> IpcResult<ProviderPreferences> {
    let (request_id, payload) = request.validate()?;
    set_pinned_model_inner(
        &settings_path(),
        payload.provider_id.trim().to_string(),
        payload.model_id.trim().to_string(),
    )
    .map(|data| respond(request_id, data))
    .map_err(from_string)
}

#[derive(Deserialize)]
pub struct ToggleTrackedProviderRequest {
    pub provider_id: String,
}

fn toggle_tracked_inner(path: &Path, provider_id: String) -> Result<ProviderPreferences, String> {
    if provider_id.is_empty() {
        return Err("provider_id không được để trống".to_string());
    }
    update_preferences(path, |settings| {
        if let Some(pos) = settings.tracked_providers.iter().position(|id| id == &provider_id) {
            settings.tracked_providers.remove(pos);
        } else {
            settings.tracked_providers.push(provider_id.clone());
        }
        Ok(())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn toggle_tracked_provider(request: Req<ToggleTrackedProviderRequest>) -> IpcResult<ProviderPreferences> {
    let (request_id, payload) = request.validate()?;
    toggle_tracked_inner(&settings_path(), payload.provider_id.trim().to_string())
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

#[derive(Deserialize)]
pub struct UntrackProviderRequest {
    pub provider_id: String,
}

fn untrack_provider_inner(path: &Path, provider_id: String) -> Result<ProviderPreferences, String> {
    update_preferences(path, |settings| {
        settings.tracked_providers.retain(|id| id != &provider_id);
        Ok(())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn untrack_provider(request: Req<UntrackProviderRequest>) -> IpcResult<ProviderPreferences> {
    let (request_id, payload) = request.validate()?;
    untrack_provider_inner(&settings_path(), payload.provider_id.trim().to_string())
        .map(|data| respond(request_id, data))
        .map_err(from_string)
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

    // Đọc settings hiện có để GIỮ favorites/pinned_models qua lần lưu này.
    let existing = read_gui_settings_from_path(path).unwrap_or_default();
    let settings = GuiSettings {
        language,
        theme_id,
        font_id,
        favorite_providers: existing.favorite_providers,
        pinned_models: existing.pinned_models,
        tracked_providers: existing.tracked_providers,
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

    fn temp_settings_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "opencode-manager-prefs-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    #[test]
    fn toggle_yeu_thich_them_cuoi_roi_bo_duoc() {
        let path = temp_settings_path("toggle");
        let _ = std::fs::remove_file(&path);

        // Thêm hai lượt: sao đứng cuối danh sách theo thứ tự bấm.
        toggle_favorite_inner(&path, "p1".into()).unwrap();
        toggle_favorite_inner(&path, "p2".into()).unwrap();
        let prefs = read_gui_settings_from_path(&path)
            .map(ProviderPreferences::from)
            .unwrap();
        assert_eq!(prefs.favorite_providers, vec!["p1".to_string(), "p2".to_string()]);

        // Bấm lại provider đang sao → bỏ sao, phần còn lại giữ nguyên thứ tự.
        toggle_favorite_inner(&path, "p1".into()).unwrap();
        let prefs = read_gui_settings_from_path(&path)
            .map(ProviderPreferences::from)
            .unwrap();
        assert_eq!(prefs.favorite_providers, vec!["p2".to_string()]);

        // provider_id rỗng bị chặn, không tạo file rác.
        assert!(toggle_favorite_inner(&path, "".into()).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn doi_thu_tu_yeu_thich_chi_nhan_hoan_vi_dung() {
        let path = temp_settings_path("reorder");
        let _ = std::fs::remove_file(&path);
        toggle_favorite_inner(&path, "a".into()).unwrap();
        toggle_favorite_inner(&path, "b".into()).unwrap();
        toggle_favorite_inner(&path, "c".into()).unwrap();

        // Hoán vị đúng → ghi nhận thứ tự mới.
        reorder_favorites_inner(&path, vec!["c".into(), "a".into(), "b".into()]).unwrap();
        let prefs = read_gui_settings_from_path(&path)
            .map(ProviderPreferences::from)
            .unwrap();
        assert_eq!(
            prefs.favorite_providers,
            vec!["c".to_string(), "a".to_string(), "b".to_string()]
        );

        // Danh sách lệch (thiếu/thừa/giả) → lỗi và KHÔNG ghi đè mất yêu thích.
        let err = reorder_favorites_inner(&path, vec!["a".into(), "b".into()]).unwrap_err();
        assert!(err.contains("không trùng khớp"), "lỗi: {err}");
        let prefs = read_gui_settings_from_path(&path)
            .map(ProviderPreferences::from)
            .unwrap();
        assert_eq!(
            prefs.favorite_providers.len(),
            3,
            "danh sách cũ phải sống sót sau reorder sai"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ghim_model_va_bo_ghim_bang_chuoi_rong() {
        let path = temp_settings_path("pin");
        let _ = std::fs::remove_file(&path);

        set_pinned_model_inner(&path, "p1".into(), "gpt-x".into()).unwrap();
        set_pinned_model_inner(&path, "p2".into(), "claude-y".into()).unwrap();
        let prefs = read_gui_settings_from_path(&path)
            .map(ProviderPreferences::from)
            .unwrap();
        assert_eq!(prefs.pinned_models.get("p1").map(String::as_str), Some("gpt-x"));
        assert_eq!(prefs.pinned_models.get("p2").map(String::as_str), Some("claude-y"));

        // Chuỗi rỗng = bỏ ghim đúng provider đó.
        set_pinned_model_inner(&path, "p1".into(), "".into()).unwrap();
        let prefs = read_gui_settings_from_path(&path)
            .map(ProviderPreferences::from)
            .unwrap();
        assert!(!prefs.pinned_models.contains_key("p1"));
        assert!(prefs.pinned_models.contains_key("p2"));

        assert!(set_pinned_model_inner(&path, "".into(), "m".into()).is_err());
        let _ = std::fs::remove_file(&path);
    }

    /// Settings cũ (file chỉ có language/theme/font) phải mượt mà nhận thêm
    /// favorites/pinned qua `#[serde(default)]` — migration không cần bước riêng.
    #[test]
    fn settings_cu_thieu_truong_so_thich_van_doc_duoc() {
        let path = temp_settings_path("legacy-fields");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, r#"{"language":"vi","theme_id":"default","font_id":"default"}"#).unwrap();

        let settings = read_gui_settings_from_path(&path).unwrap();
        assert!(settings.favorite_providers.is_empty());
        assert!(settings.pinned_models.is_empty());

        // Ghi preference vào file cũ → các trường cài đặt gốc không bị mất.
        toggle_favorite_inner(&path, "legacy_p".into()).unwrap();
        let settings = read_gui_settings_from_path(&path).unwrap();
        assert_eq!(settings.language, "vi");
        assert_eq!(settings.favorite_providers, vec!["legacy_p".to_string()]);
        let _ = std::fs::remove_file(&path);
    }

    /// Lưu language/theme/font (trang Cài đặt) KHÔNG được xoá yêu thích/ghim.
    #[test]
    fn save_cai_dat_giu_nguyen_yeu_thich_ghim() {
        let path = temp_settings_path("preserve");
        let _ = std::fs::remove_file(&path);
        toggle_favorite_inner(&path, "keep_me".into()).unwrap();
        set_pinned_model_inner(&path, "keep_me".into(), "m1".into()).unwrap();

        save_gui_settings_to_path(&path, "vi".into(), "default".into(), "default".into()).unwrap();
        let settings = read_gui_settings_from_path(&path).unwrap();
        assert_eq!(settings.favorite_providers, vec!["keep_me".to_string()]);
        assert_eq!(settings.pinned_models.get("keep_me").map(String::as_str), Some("m1"));
        let _ = std::fs::remove_file(&path);
    }
}
