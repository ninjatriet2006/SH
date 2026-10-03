use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};

fn default_true() -> bool { true }
fn default_language() -> String { "en".to_string() }
fn default_theme() -> String { "default".to_string() }
fn default_font() -> String { "default".to_string() }
fn default_theme_color() -> String { "default".to_string() }
fn default_ui_scale() -> f32 { 1.0 }
fn default_close_behavior() -> String { "minimize".to_string() }
fn default_auto_refresh_minutes() -> u32 { 10 }
fn default_no_proxy() -> String { "127.0.0.1,localhost,::1".to_string() }
fn default_ws_port() -> u16 { 19528 }
fn default_vscode_path() -> String { "/usr/bin/code".to_string() }
fn default_antigravity_path() -> String { "/home/bimatkeo/Applications/antigravity-ide/antigravity-ide".to_string() }
fn default_alert_threshold() -> u32 { 20 }
fn default_switch_threshold() -> u32 { 5 }
fn default_retention_days() -> u32 { 15 }
fn default_webdav_dir() -> String { "cockpit-tools".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    // 1. Appearance & UI
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default = "default_theme_color")]
    pub theme_color: String,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default = "default_close_behavior")]
    pub close_behavior: String,
    #[serde(default)]
    pub startup_minimized: bool,
    #[serde(default)]
    pub app_auto_launch_enabled: bool,
    #[serde(default)]
    pub reduced_motion_enabled: bool,

    // 2. Session Keeper & Background Refresh
    #[serde(default = "default_true")]
    pub token_keeper_enabled: bool,
    #[serde(default = "default_auto_refresh_minutes")]
    pub auto_refresh_minutes: u32,
    #[serde(default)]
    pub auto_import_from_local_enabled: bool,
    #[serde(default)]
    pub share_sessions_on_switch: bool,

    // 3. Network & Proxy
    #[serde(default)]
    pub global_proxy_enabled: bool,
    #[serde(default)]
    pub global_proxy_url: String,
    #[serde(default = "default_no_proxy")]
    pub global_proxy_no_proxy: String,
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,

    // 4. IDE Paths & Automation
    #[serde(default = "default_vscode_path")]
    pub vscode_app_path: String,
    #[serde(default = "default_antigravity_path")]
    pub antigravity_app_path: String,
    #[serde(default)]
    pub cursor_app_path: String,
    #[serde(default)]
    pub trae_app_path: String,
    #[serde(default)]
    pub zed_app_path: String,
    #[serde(default = "default_true")]
    pub launch_on_switch: bool,

    // 5. Quota Alerts & Auto Switch
    #[serde(default)]
    pub quota_alert_enabled: bool,
    #[serde(default = "default_alert_threshold")]
    pub quota_alert_threshold: u32,
    #[serde(default)]
    pub auto_switch_enabled: bool,
    #[serde(default = "default_switch_threshold")]
    pub auto_switch_threshold: u32,

    // 6. Backup & Cloud Sync (WebDAV)
    #[serde(default = "default_true")]
    pub auto_backup_enabled: bool,
    #[serde(default = "default_retention_days")]
    pub auto_backup_retention_days: u32,
    #[serde(default)]
    pub webdav_sync_enabled: bool,
    #[serde(default)]
    pub webdav_sync_url: String,
    #[serde(default)]
    pub webdav_sync_username: String,
    #[serde(default)]
    pub webdav_sync_password: String,
    #[serde(default = "default_webdav_dir")]
    pub webdav_sync_remote_dir: String,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            font: default_font(),
            theme_color: default_theme_color(),
            ui_scale: default_ui_scale(),
            close_behavior: default_close_behavior(),
            startup_minimized: false,
            app_auto_launch_enabled: false,
            reduced_motion_enabled: false,
            token_keeper_enabled: true,
            auto_refresh_minutes: default_auto_refresh_minutes(),
            auto_import_from_local_enabled: false,
            share_sessions_on_switch: false,
            global_proxy_enabled: false,
            global_proxy_url: String::new(),
            global_proxy_no_proxy: default_no_proxy(),
            ws_port: default_ws_port(),
            vscode_app_path: default_vscode_path(),
            antigravity_app_path: default_antigravity_path(),
            cursor_app_path: String::new(),
            trae_app_path: String::new(),
            zed_app_path: String::new(),
            launch_on_switch: true,
            quota_alert_enabled: false,
            quota_alert_threshold: default_alert_threshold(),
            auto_switch_enabled: false,
            auto_switch_threshold: default_switch_threshold(),
            auto_backup_enabled: true,
            auto_backup_retention_days: default_retention_days(),
            webdav_sync_enabled: false,
            webdav_sync_url: "https://dav.jianguoyun.com/dav/".to_string(),
            webdav_sync_username: String::new(),
            webdav_sync_password: String::new(),
            webdav_sync_remote_dir: default_webdav_dir(),
        }
    }
}

/// gui_settings.json giờ nằm ở XDG config dir (ghi được khi app đã cài đặt).
/// Đọc fallback file cũ cạnh resources (bản dev/setup legacy) để không mất setting.
/// Trước đây ghi thẳng vào resource dir — app cài đặt (resources chỉ đọc) là fail.
fn settings_path() -> std::path::PathBuf {
    match crate::core::paths::config_dir() {
        Some(dir) => dir.join("gui_settings.json"),
        None => crate::core::resources::resource_base().join("gui_settings.json"),
    }
}

fn legacy_settings_path() -> std::path::PathBuf {
    crate::core::resources::resource_base().join("gui_settings.json")
}

fn load_settings() -> GuiSettings {
    for path in [settings_path(), legacy_settings_path()] {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<GuiSettings>(&content) {
                return parsed;
            }
        }
    }
    GuiSettings::default()
}

fn save_settings_inner(settings: &GuiSettings) -> Result<(), String> {
    let path = settings_path();
    crate::core::paths::ensure_parent(&path)?;
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_gui_settings(request: Req<Empty>) -> IpcResult<GuiSettings> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, load_settings()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_gui_settings(request: Req<GuiSettings>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    save_settings_inner(&payload).map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}
