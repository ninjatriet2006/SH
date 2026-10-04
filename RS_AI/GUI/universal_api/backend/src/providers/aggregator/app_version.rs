//! App version detection through Electron ASAR archives and executable inspection.

use crate::ipc::{respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledAppInfo {
    pub installed: bool,
    pub name: String,
    pub version: String,
    pub exec_path: String,
    pub target_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformAppInfoRequest {
    pub platform_id: String,
    pub variant: Option<String>,
}

pub fn read_asar_package_json(asar_path: &Path) -> Option<(String, String)> {
    let mut file = File::open(asar_path).ok()?;
    let mut header = [0u8; 16];
    file.read_exact(&mut header).ok()?;
    let header_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
    let json_len = u32::from_le_bytes([header[12], header[13], header[14], header[15]]) as usize;
    if json_len > 2_000_000 {
        return None;
    }
    let mut json_buf = vec![0u8; json_len];
    file.read_exact(&mut json_buf).ok()?;
    let header_json: serde_json::Value = serde_json::from_slice(&json_buf).ok()?;
    let pkg_info = header_json.get("files")?.get("package.json")?;
    let offset_val = pkg_info.get("offset")?;
    let offset: u64 = if let Some(n) = offset_val.as_u64() {
        n
    } else if let Some(s) = offset_val.as_str() {
        s.parse().ok()?
    } else {
        return None;
    };
    let size = pkg_info.get("size")?.as_u64()? as usize;
    if size > 1_000_000 {
        return None;
    }
    file.seek(SeekFrom::Start(8 + header_size + offset)).ok()?;
    let mut pkg_buf = vec![0u8; size];
    file.read_exact(&mut pkg_buf).ok()?;
    let pkg_json: serde_json::Value = serde_json::from_slice(&pkg_buf).ok()?;
    let name = pkg_json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let version = pkg_json.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if !version.is_empty() {
        Some((name, version))
    } else {
        None
    }
}

pub fn extract_version_from_exec_path(exec_path: &Path, default_name: &str) -> (String, String) {
    let mut detected_name = default_name.to_string();

    if let Some(parent) = exec_path.parent() {
        // 1. Check resources/app/product.json (VS Code style)
        for cand in [
            parent.join("resources/app/product.json"),
            parent.join("../resources/app/product.json"),
        ] {
            if cand.is_file() {
                if let Ok(content) = std::fs::read_to_string(&cand) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(n) = json.get("nameShort").and_then(|v| v.as_str()) {
                            detected_name = n.to_string();
                        }
                        if let Some(v) = json.get("ideVersion").or_else(|| json.get("version")).and_then(|v| v.as_str()) {
                            return (detected_name, v.to_string());
                        }
                    }
                }
            }
        }

        // 2. Check resources/app/package.json
        for cand in [
            parent.join("resources/app/package.json"),
            parent.join("../resources/app/package.json"),
        ] {
            if cand.is_file() {
                if let Ok(content) = std::fs::read_to_string(&cand) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(n) = json.get("name").and_then(|v| v.as_str()) {
                            detected_name = n.to_string();
                        }
                        if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
                            return (detected_name, v.to_string());
                        }
                    }
                }
            }
        }

        // 3. Check resources/app.asar
        for cand in [
            parent.join("resources/app.asar"),
            parent.join("../resources/app.asar"),
        ] {
            if cand.is_file() {
                if let Some((n, v)) = read_asar_package_json(&cand) {
                    if detected_name.is_empty() && !n.is_empty() {
                        detected_name = n;
                    }
                    return (detected_name, v);
                }
            }
        }
    }

    // 4. Try running with --version (CLI or standard binaries)
    if let Ok(output) = std::process::Command::new(exec_path).arg("--version").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Ok(re) = regex::Regex::new(r"\b\d+\.\d+(?:\.\d+)?\b") {
                        if let Some(m) = re.find(trimmed) {
                            return (detected_name, m.as_str().to_string());
                        }
                    }
                }
            }
        }
    }

    (detected_name, "Unknown".to_string())
}

pub fn resolve_installed_app_info(platform_id: &str, variant: Option<&str>) -> InstalledAppInfo {
    let (target_key, display_name) = match platform_id.to_lowercase().as_str() {
        "antigravity" => {
            if variant == Some("desktop") {
                ("antigravity_desktop", "Antigravity Desktop")
            } else {
                ("antigravity_ide", "Antigravity IDE")
            }
        }
        "antigravity_ide" => ("antigravity_ide", "Antigravity IDE"),
        "antigravity_desktop" => ("antigravity_desktop", "Antigravity Desktop"),
        "codebuddy" => {
            if variant == Some("cn") {
                ("codebuddy_cn", "CodeBuddy CN")
            } else {
                ("codebuddy", "CodeBuddy Global")
            }
        }
        "codebuddy_cn" => ("codebuddy_cn", "CodeBuddy CN"),
        "codebuddy_global" => ("codebuddy", "CodeBuddy Global"),
        "vscode" | "github_copilot" => ("vscode", "VS Code"),
        "cursor" => ("cursor", "Cursor"),
        "windsurf" => ("windsurf", "Windsurf"),
        "trae" => ("trae", "Trae"),
        "claude" => ("claude", "Claude"),
        "zed" => ("zed", "Zed Cloud"),
        "codex" => ("codex", "Codex"),
        "kiro" => ("kiro", "Kiro"),
        "qoder" => ("qoder", "Qoder"),
        "zcode" => ("zcode", "ZCode"),
        "workbuddy" => ("workbuddy", "WorkBuddy"),
        "grok" => ("grok", "Grok"),
        _ => (platform_id, platform_id),
    };

    let settings = crate::actions::settings::load_settings();
    let configured_path = match target_key {
        "antigravity_ide" => &settings.antigravity_app_path,
        "antigravity_desktop" => &settings.antigravity_desktop_app_path,
        "vscode" => &settings.vscode_app_path,
        "cursor" => &settings.cursor_app_path,
        "windsurf" => &settings.windsurf_app_path,
        "trae" => &settings.trae_app_path,
        "claude" => &settings.claude_app_path,
        "zed" => &settings.zed_app_path,
        "codex" => {
            if !settings.codex_app_path.is_empty() {
                &settings.codex_app_path
            } else {
                &settings.codex_specified_app_path
            }
        }
        "codebuddy" => &settings.codebuddy_app_path,
        "codebuddy_cn" => &settings.codebuddy_cn_app_path,
        "kiro" => &settings.kiro_app_path,
        "qoder" => &settings.qoder_app_path,
        "zcode" => &settings.zcode_app_path,
        "workbuddy" => &settings.workbuddy_app_path,
        "grok" => &settings.grok_cli_path,
        _ => "",
    };

    let mut found_path: Option<PathBuf> = None;

    // 1. Check user configured path in settings
    if !configured_path.trim().is_empty() {
        let p = PathBuf::from(configured_path.trim());
        if p.is_file() {
            found_path = Some(p);
        }
    }

    // 2. Check Cockpit's ~/.cockpit_tools/config.json
    if found_path.is_none() {
        if let Some(cp_dir) = crate::core::paths::cockpit_dir() {
            let cp_config = cp_dir.join("config.json");
            if let Ok(content) = std::fs::read_to_string(&cp_config) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let field = match target_key {
                        "antigravity_ide" => "antigravity_app_path",
                        "antigravity_desktop" => "antigravity_desktop_app_path",
                        "vscode" => "vscode_app_path",
                        "cursor" => "cursor_app_path",
                        "windsurf" => "windsurf_app_path",
                        "trae" => "trae_app_path",
                        "claude" => "claude_app_path",
                        "zed" => "zed_app_path",
                        "codex" => "codex_app_path",
                        "codebuddy" => "codebuddy_app_path",
                        "codebuddy_cn" => "codebuddy_cn_app_path",
                        "kiro" => "kiro_app_path",
                        "qoder" => "qoder_app_path",
                        "zcode" => "zcode_app_path",
                        "workbuddy" => "workbuddy_app_path",
                        _ => "",
                    };
                    if let Some(p_str) = json.get(field).and_then(|v| v.as_str()) {
                        let p = PathBuf::from(p_str.trim());
                        if p.is_file() {
                            found_path = Some(p);
                        }
                    }
                }
            }
        }
    }

    // 3. Scan .desktop files
    if found_path.is_none() {
        if let Some(desk_path) = crate::actions::ide_detector::scan_desktop_files_for_exec(target_key) {
            let p = PathBuf::from(desk_path);
            if p.is_file() {
                found_path = Some(p);
            }
        }
    }

    // 4. Well-known defaults on disk
    if found_path.is_none() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_p = Path::new(&home);

        let well_known: Vec<PathBuf> = match target_key {
            "antigravity_ide" => vec![
                home_p.join("Applications/antigravity-ide/antigravity-ide"),
                home_p.join("Applications/antigravity-ide/bin/antigravity-ide"),
                PathBuf::from("/usr/bin/antigravity-ide"),
                PathBuf::from("/opt/antigravity-ide/antigravity-ide"),
            ],
            "antigravity_desktop" => vec![
                home_p.join("Applications/antigravity/antigravity"),
                PathBuf::from("/usr/bin/antigravity"),
                PathBuf::from("/opt/antigravity/antigravity"),
            ],
            "vscode" => vec![
                PathBuf::from("/usr/bin/code"),
                PathBuf::from("/usr/local/bin/code"),
            ],
            _ => vec![],
        };

        for cand in well_known {
            if cand.is_file() {
                found_path = Some(cand);
                break;
            }
        }
    }

    // 5. Check PATH using find_executable
    if found_path.is_none() {
        if let Some(exec) = crate::core::instance::discovery::find_executable(target_key) {
            if exec.is_file() {
                found_path = Some(exec);
            }
        }
    }

    match found_path {
        Some(path) => {
            let (real_name, version) = extract_version_from_exec_path(&path, display_name);
            InstalledAppInfo {
                installed: true,
                name: real_name,
                version,
                exec_path: path.to_string_lossy().to_string(),
                target_kind: target_key.to_string(),
            }
        }
        None => InstalledAppInfo {
            installed: false,
            name: display_name.to_string(),
            version: "Not Found".to_string(),
            exec_path: String::new(),
            target_kind: target_key.to_string(),
        },
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_antigravity_installed_version_info(
    request: Req<Empty>,
) -> IpcResult<InstalledAppInfo> {
    let (request_id, _) = request.validate()?;
    let info = resolve_installed_app_info("antigravity", Some("ide"));
    Ok(respond(request_id, info))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_installed_app_version_info(
    request: Req<PlatformAppInfoRequest>,
) -> IpcResult<InstalledAppInfo> {
    let (request_id, payload) = request.validate()?;
    let info = resolve_installed_app_info(&payload.platform_id, payload.variant.as_deref());
    Ok(respond(request_id, info))
}
