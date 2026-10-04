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
fn default_antigravity_desktop_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/Applications/antigravity/antigravity", home)
}
fn default_alert_threshold() -> u32 { 20 }
fn default_switch_threshold() -> u32 { 5 }
fn default_retention_days() -> u32 { 15 }
fn default_webdav_dir() -> String { "cockpit-tools".to_string() }
fn default_default_terminal() -> String { "system".to_string() }
fn default_side_nav_layout() -> String { "classic".to_string() }
fn default_startup_page() -> String { "last".to_string() }
fn default_color_pack() -> String { "default".to_string() }
fn default_current_refresh_minutes() -> u32 { 1 }
fn default_scope_mode() -> String { "any_group".to_string() }
fn default_account_scope_mode() -> String { "all_accounts".to_string() }
fn default_quota_platform() -> String { "codex".to_string() }

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
    #[serde(default = "default_default_terminal")]
    pub default_terminal: String,
    #[serde(default = "default_side_nav_layout")]
    pub side_nav_layout_mode: String,
    #[serde(default)]
    pub remember_main_window_state: bool,
    #[serde(default)]
    pub floating_card_show_on_startup: bool,
    #[serde(default)]
    pub floating_card_always_on_top: bool,
    #[serde(default)]
    pub show_top_promo: bool,
    #[serde(default = "default_startup_page")]
    pub startup_page: String,
    #[serde(default = "default_color_pack")]
    pub color_pack: String,
    #[serde(default = "default_true")]
    pub allow_external_network: bool,
    #[serde(default)]
    pub webdav_allowed_domains: String,

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
    #[serde(default = "default_antigravity_desktop_path")]
    pub antigravity_desktop_app_path: String,
    #[serde(default)]
    pub cursor_app_path: String,
    #[serde(default)]
    pub trae_app_path: String,
    #[serde(default)]
    pub zed_app_path: String,
    #[serde(default)]
    pub codebuddy_app_path: String,
    #[serde(default)]
    pub codebuddy_cn_app_path: String,
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
    #[serde(default = "default_scope_mode")]
    pub auto_switch_scope_mode: String,
    #[serde(default = "default_account_scope_mode")]
    pub auto_switch_account_scope_mode: String,
    #[serde(default)]
    pub auto_switch_credits_enabled: bool,
    #[serde(default = "default_switch_threshold")]
    pub auto_switch_credits_threshold: u32,

    // 6. Backup & Cloud Sync (WebDAV)
    #[serde(default = "default_true")]
    pub auto_backup_enabled: bool,
    #[serde(default = "default_true")]
    pub auto_backup_include_accounts: bool,
    #[serde(default = "default_true")]
    pub auto_backup_include_config: bool,
    #[serde(default = "default_retention_days")]
    pub auto_backup_retention_days: u32,
    #[serde(default)]
    pub backup_directory: String,
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

    // 7. Platform Specific Settings
    // Claude
    #[serde(default = "default_auto_refresh_minutes")]
    pub claude_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub claude_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub claude_quota_display_remaining: bool,
    #[serde(default)]
    pub claude_app_path: String,
    #[serde(default)]
    pub claude_quota_alert_enabled: bool,

    // Zed
    #[serde(default = "default_auto_refresh_minutes")]
    pub zed_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub zed_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub zed_quota_alert_enabled: bool,

    // GitHub Copilot
    #[serde(default = "default_auto_refresh_minutes")]
    pub ghcp_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub ghcp_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub ghcp_quota_alert_enabled: bool,
    #[serde(default)]
    pub ghcp_account_refresh_config: std::collections::HashMap<String, u32>,
    #[serde(default = "default_true")]
    pub ghcp_launch_on_switch: bool,
    #[serde(default)]
    pub ghcp_opencode_sync_on_switch: bool,
    #[serde(default)]
    pub ghcp_opencode_auth_overwrite_on_switch: bool,

    // Devin / Windsurf
    #[serde(default = "default_auto_refresh_minutes")]
    pub windsurf_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub windsurf_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub windsurf_app_path: String,
    #[serde(default)]
    pub windsurf_quota_alert_enabled: bool,

    // Antigravity IDE
    #[serde(default = "default_auto_refresh_minutes")]
    pub antigravity_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub antigravity_current_account_refresh_minutes: u32,
    #[serde(default = "default_true")]
    pub antigravity_launch_on_switch: bool,
    #[serde(default)]
    pub antigravity_dual_switch_no_restart_enabled: bool,
    #[serde(default)]
    pub antigravity_startup_wakeup_enabled: bool,
    #[serde(default)]
    pub antigravity_startup_wakeup_delay_seconds: u32,
    #[serde(default)]
    pub antigravity_quota_alert_enabled: bool,

    // Codex
    #[serde(default = "default_auto_refresh_minutes")]
    pub codex_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub codex_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub codex_app_path: String,
    #[serde(default)]
    pub codex_specified_app_path: String,
    #[serde(default)]
    pub codex_sync_wsl: bool,
    #[serde(default)]
    pub codex_wsl_config_dir: String,
    #[serde(default = "default_true")]
    pub codex_app_ui_injection_enabled: bool,
    #[serde(default = "default_true")]
    pub codex_launch_on_switch: bool,
    #[serde(default = "default_true")]
    pub codex_local_access_entry_visible: bool,
    #[serde(default)]
    pub codex_hide_relay_quota: bool,
    #[serde(default)]
    pub codex_startup_wakeup_enabled: bool,
    #[serde(default)]
    pub codex_startup_wakeup_delay_seconds: u32,
    #[serde(default)]
    pub codex_quota_alert_enabled: bool,
    #[serde(default = "default_alert_threshold")]
    pub codex_quota_alert_primary_threshold: u32,
    #[serde(default = "default_alert_threshold")]
    pub codex_quota_alert_secondary_threshold: u32,
    #[serde(default)]
    pub codex_auto_switch_enabled: bool,
    #[serde(default = "default_alert_threshold")]
    pub codex_auto_switch_primary_threshold: u32,
    #[serde(default = "default_alert_threshold")]
    pub codex_auto_switch_secondary_threshold: u32,
    #[serde(default)]
    pub codex_hermes_auth_overwrite_on_switch: bool,
    #[serde(default)]
    pub codex_openclaw_auth_overwrite_on_switch: bool,
    #[serde(default)]
    pub codex_opencode_auth_overwrite_on_switch: bool,

    // Cursor
    #[serde(default = "default_auto_refresh_minutes")]
    pub cursor_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub cursor_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub cursor_quota_alert_enabled: bool,

    // Kiro
    #[serde(default = "default_auto_refresh_minutes")]
    pub kiro_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub kiro_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub kiro_app_path: String,
    #[serde(default)]
    pub kiro_quota_alert_enabled: bool,

    // CodeBuddy & CodeBuddy CN
    #[serde(default = "default_auto_refresh_minutes")]
    pub codebuddy_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub codebuddy_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub codebuddy_share_sessions_on_switch: bool,
    #[serde(default)]
    pub codebuddy_quota_alert_enabled: bool,
    #[serde(default = "default_auto_refresh_minutes")]
    pub codebuddy_cn_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub codebuddy_cn_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub codebuddy_cn_share_sessions_on_switch: bool,
    #[serde(default)]
    pub codebuddy_cn_quota_alert_enabled: bool,

    // WorkBuddy
    #[serde(default = "default_auto_refresh_minutes")]
    pub workbuddy_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub workbuddy_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub workbuddy_app_path: String,
    #[serde(default = "default_true")]
    pub workbuddy_share_sessions_on_switch: bool,
    #[serde(default)]
    pub workbuddy_quota_alert_enabled: bool,

    // Trae & variants
    #[serde(default = "default_auto_refresh_minutes")]
    pub trae_auto_refresh_minutes: u32,
    #[serde(default = "default_current_refresh_minutes")]
    pub trae_current_account_refresh_minutes: u32,
    #[serde(default)]
    pub trae_share_sessions_on_switch: bool,
    #[serde(default)]
    pub trae_quota_alert_enabled: bool,
    #[serde(default)]
    pub trae_solo_app_path: String,
    #[serde(default)]
    pub trae_cn_app_path: String,
    #[serde(default)]
    pub trae_solo_cn_app_path: String,

    // Qoder & ZCode
    #[serde(default = "default_auto_refresh_minutes")]
    pub qoder_auto_refresh_minutes: u32,
    #[serde(default)]
    pub qoder_app_path: String,
    #[serde(default)]
    pub qoder_quota_alert_enabled: bool,
    #[serde(default = "default_auto_refresh_minutes")]
    pub zcode_auto_refresh_minutes: u32,
    #[serde(default)]
    pub zcode_app_path: String,

    // Grok CLI
    #[serde(default = "default_auto_refresh_minutes")]
    pub grok_auto_refresh_minutes: u32,
    #[serde(default)]
    pub grok_cli_path: String,
    #[serde(default)]
    pub grok_sync_official_auth_on_switch: bool,
    #[serde(default)]
    pub grok_opencode_sync_on_switch: bool,
    #[serde(default)]
    pub grok_opencode_auth_overwrite_on_switch: bool,
    #[serde(default)]
    pub grok_quota_alert_enabled: bool,

    // 8. Menu Bar Quota
    #[serde(default)]
    pub menu_bar_quota_enabled: bool,
    #[serde(default = "default_true")]
    pub menu_bar_show_account_prefix: bool,
    #[serde(default = "default_quota_platform")]
    pub menu_bar_quota_platform: String,
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
            default_terminal: default_default_terminal(),
            side_nav_layout_mode: default_side_nav_layout(),
            remember_main_window_state: false,
            floating_card_show_on_startup: false,
            floating_card_always_on_top: false,
            show_top_promo: false,
            startup_page: default_startup_page(),
            color_pack: default_color_pack(),
            allow_external_network: true,
            webdav_allowed_domains: String::new(),
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
            antigravity_desktop_app_path: default_antigravity_desktop_path(),
            cursor_app_path: String::new(),
            trae_app_path: String::new(),
            zed_app_path: String::new(),
            codebuddy_app_path: String::new(),
            codebuddy_cn_app_path: String::new(),
            launch_on_switch: true,
            quota_alert_enabled: false,
            quota_alert_threshold: default_alert_threshold(),
            auto_switch_enabled: false,
            auto_switch_threshold: default_switch_threshold(),
            auto_switch_scope_mode: default_scope_mode(),
            auto_switch_account_scope_mode: default_account_scope_mode(),
            auto_switch_credits_enabled: false,
            auto_switch_credits_threshold: default_switch_threshold(),
            auto_backup_enabled: true,
            auto_backup_include_accounts: true,
            auto_backup_include_config: true,
            auto_backup_retention_days: default_retention_days(),
            backup_directory: String::new(),
            webdav_sync_enabled: false,
            webdav_sync_url: "https://dav.jianguoyun.com/dav/".to_string(),
            webdav_sync_username: String::new(),
            webdav_sync_password: String::new(),
            webdav_sync_remote_dir: default_webdav_dir(),
            // Platforms
            claude_auto_refresh_minutes: default_auto_refresh_minutes(),
            claude_current_account_refresh_minutes: default_current_refresh_minutes(),
            claude_quota_display_remaining: false,
            claude_app_path: String::new(),
            claude_quota_alert_enabled: false,
            zed_auto_refresh_minutes: default_auto_refresh_minutes(),
            zed_current_account_refresh_minutes: default_current_refresh_minutes(),
            zed_quota_alert_enabled: false,
            ghcp_auto_refresh_minutes: default_auto_refresh_minutes(),
            ghcp_current_account_refresh_minutes: default_current_refresh_minutes(),
            ghcp_quota_alert_enabled: false,
            ghcp_account_refresh_config: std::collections::HashMap::new(),
            ghcp_launch_on_switch: true,
            ghcp_opencode_sync_on_switch: false,
            ghcp_opencode_auth_overwrite_on_switch: false,
            windsurf_auto_refresh_minutes: default_auto_refresh_minutes(),
            windsurf_current_account_refresh_minutes: default_current_refresh_minutes(),
            windsurf_app_path: String::new(),
            windsurf_quota_alert_enabled: false,
            antigravity_auto_refresh_minutes: default_auto_refresh_minutes(),
            antigravity_current_account_refresh_minutes: default_current_refresh_minutes(),
            antigravity_launch_on_switch: true,
            antigravity_dual_switch_no_restart_enabled: false,
            antigravity_startup_wakeup_enabled: false,
            antigravity_startup_wakeup_delay_seconds: 0,
            antigravity_quota_alert_enabled: false,
            codex_auto_refresh_minutes: default_auto_refresh_minutes(),
            codex_current_account_refresh_minutes: default_current_refresh_minutes(),
            codex_app_path: String::new(),
            codex_specified_app_path: String::new(),
            codex_sync_wsl: false,
            codex_wsl_config_dir: String::new(),
            codex_app_ui_injection_enabled: true,
            codex_launch_on_switch: true,
            codex_local_access_entry_visible: true,
            codex_hide_relay_quota: false,
            codex_startup_wakeup_enabled: false,
            codex_startup_wakeup_delay_seconds: 0,
            codex_quota_alert_enabled: false,
            codex_quota_alert_primary_threshold: default_alert_threshold(),
            codex_quota_alert_secondary_threshold: default_alert_threshold(),
            codex_auto_switch_enabled: false,
            codex_auto_switch_primary_threshold: default_alert_threshold(),
            codex_auto_switch_secondary_threshold: default_alert_threshold(),
            codex_hermes_auth_overwrite_on_switch: false,
            codex_openclaw_auth_overwrite_on_switch: false,
            codex_opencode_auth_overwrite_on_switch: false,
            cursor_auto_refresh_minutes: default_auto_refresh_minutes(),
            cursor_current_account_refresh_minutes: default_current_refresh_minutes(),
            cursor_quota_alert_enabled: false,
            kiro_auto_refresh_minutes: default_auto_refresh_minutes(),
            kiro_current_account_refresh_minutes: default_current_refresh_minutes(),
            kiro_app_path: String::new(),
            kiro_quota_alert_enabled: false,
            codebuddy_auto_refresh_minutes: default_auto_refresh_minutes(),
            codebuddy_current_account_refresh_minutes: default_current_refresh_minutes(),
            codebuddy_share_sessions_on_switch: false,
            codebuddy_quota_alert_enabled: false,
            codebuddy_cn_auto_refresh_minutes: default_auto_refresh_minutes(),
            codebuddy_cn_current_account_refresh_minutes: default_current_refresh_minutes(),
            codebuddy_cn_share_sessions_on_switch: false,
            codebuddy_cn_quota_alert_enabled: false,
            workbuddy_auto_refresh_minutes: default_auto_refresh_minutes(),
            workbuddy_current_account_refresh_minutes: default_current_refresh_minutes(),
            workbuddy_app_path: String::new(),
            workbuddy_share_sessions_on_switch: true,
            workbuddy_quota_alert_enabled: false,
            trae_auto_refresh_minutes: default_auto_refresh_minutes(),
            trae_current_account_refresh_minutes: default_current_refresh_minutes(),
            trae_share_sessions_on_switch: false,
            trae_quota_alert_enabled: false,
            trae_solo_app_path: String::new(),
            trae_cn_app_path: String::new(),
            trae_solo_cn_app_path: String::new(),
            qoder_auto_refresh_minutes: default_auto_refresh_minutes(),
            qoder_app_path: String::new(),
            qoder_quota_alert_enabled: false,
            zcode_auto_refresh_minutes: default_auto_refresh_minutes(),
            zcode_app_path: String::new(),
            grok_auto_refresh_minutes: default_auto_refresh_minutes(),
            grok_cli_path: String::new(),
            grok_sync_official_auth_on_switch: false,
            grok_opencode_sync_on_switch: false,
            grok_opencode_auth_overwrite_on_switch: false,
            grok_quota_alert_enabled: false,
            menu_bar_quota_enabled: false,
            menu_bar_show_account_prefix: true,
            menu_bar_quota_platform: default_quota_platform(),
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

pub fn load_settings() -> GuiSettings {
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
pub use crate::actions::ide_detector::*;

#[tauri::command(rename_all = "snake_case")]
pub fn open_data_folder(request: Req<Empty>) -> IpcResult<String> {
    let (request_id, _) = request.validate()?;
    let data_dir = match crate::core::paths::config_dir() {
        Some(dir) => dir,
        None => std::path::PathBuf::from("./data"),
    };
    let _ = std::fs::create_dir_all(&data_dir);

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&data_dir).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&data_dir).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&data_dir).spawn();
    }

    Ok(respond(request_id, data_dir.to_string_lossy().to_string()))
}
