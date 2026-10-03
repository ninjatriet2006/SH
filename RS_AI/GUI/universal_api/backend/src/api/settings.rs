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
    #[serde(default = "default_antigravity_desktop_path")]
    pub antigravity_desktop_app_path: String,
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
            antigravity_desktop_app_path: default_antigravity_desktop_path(),
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

#[derive(Debug, Clone, Deserialize)]
pub struct DetectIdePathRequest {
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectIdePathResponse {
    pub found: bool,
    pub path: Option<String>,
    pub message: String,
}

fn scan_desktop_files_for_exec(target_key: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let search_dirs = [
        std::path::PathBuf::from("/usr/share/applications"),
        std::path::PathBuf::from(&home).join(".local/share/applications"),
        std::path::PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        std::path::PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];

    let target_lower = target_key.to_lowercase();

    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "desktop") {
                    let filename = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();

                    let matched = match target_lower.as_str() {
                        "antigravity_ide" => filename.contains("antigravity-ide"),
                        "antigravity_desktop" => {
                            filename.contains("antigravity") && !filename.contains("antigravity-ide")
                        }
                        "vscode" => {
                            filename == "code.desktop"
                                || filename.contains("visual-studio-code")
                                || filename.contains("com.microsoft.vscode")
                        }
                        "cursor" => filename.contains("cursor") && !filename.contains("url-handler"),
                        "zed" => filename.contains("zed"),
                        "trae" => filename.contains("trae"),
                        _ => false,
                    };

                    if let Ok(content) = std::fs::read_to_string(&p) {
                        let is_match = matched
                            || match target_lower.as_str() {
                                "antigravity_ide" => content.contains("Name=Antigravity IDE"),
                                "antigravity_desktop" => {
                                    content.contains("Name=Antigravity")
                                        && !content.contains("Name=Antigravity IDE")
                                }
                                "vscode" => {
                                    content.contains("Name=Visual Studio Code")
                                        || content.contains("Name=Code")
                                }
                                "cursor" => content.contains("Name=Cursor"),
                                "zed" => content.contains("Name=Zed"),
                                "trae" => content.contains("Name=Trae"),
                                _ => false,
                            };

                        if is_match {
                            for line in content.lines() {
                                if let Some(exec_part) = line.strip_prefix("Exec=") {
                                    let trimmed = exec_part.trim();
                                    let raw_path = if trimmed.starts_with('"') {
                                        if let Some(end_quote) = trimmed[1..].find('"') {
                                            &trimmed[1..=end_quote]
                                        } else {
                                            trimmed.split_whitespace().next().unwrap_or(trimmed)
                                        }
                                    } else {
                                        trimmed.split_whitespace().next().unwrap_or(trimmed)
                                    };
                                    let path_clean = raw_path.trim_matches('"');
                                    if std::path::Path::new(path_clean).is_file() {
                                        return Some(path_clean.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: check `which` binary command
    let binary_name = match target_lower.as_str() {
        "vscode" => Some("code"),
        "antigravity_ide" => Some("antigravity-ide"),
        "antigravity_desktop" => Some("antigravity"),
        "cursor" => Some("cursor"),
        "trae" => Some("trae"),
        "zed" => Some("zed"),
        _ => None,
    };
    if let Some(cmd) = binary_name {
        if let Ok(out) = std::process::Command::new("which").arg(cmd).output() {
            if out.status.success() {
                let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path_str.is_empty() && std::path::Path::new(&path_str).is_file() {
                    return Some(path_str);
                }
            }
        }
    }

    None
}

#[tauri::command(rename_all = "snake_case")]
pub fn auto_detect_ide_path(
    request: Req<DetectIdePathRequest>,
) -> IpcResult<DetectIdePathResponse> {
    let (request_id, payload) = request.validate()?;
    if let Some(found_path) = scan_desktop_files_for_exec(&payload.target) {
        Ok(respond(
            request_id,
            DetectIdePathResponse {
                found: true,
                path: Some(found_path.clone()),
                message: format!("Đã phát hiện thành công qua tệp .desktop: {}", found_path),
            },
        ))
    } else {
        Ok(respond(
            request_id,
            DetectIdePathResponse {
                found: false,
                path: None,
                message: "Không tự động tìm thấy ứng dụng qua tệp .desktop. Vui lòng bấm 'Browse' để chọn tệp thủ công.".into(),
            },
        ))
    }
}
