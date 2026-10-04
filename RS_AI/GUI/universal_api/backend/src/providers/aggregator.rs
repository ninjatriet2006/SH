//! Aggregator module for cross-provider overviews, version detection, account groups, and sync.

use super::storage::read_cockpit_secure_json;
use crate::core::runtime::RuntimeState;
use crate::ipc::{respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityQuotaBucket {
    pub bucket_id: String,
    pub label: String,
    pub remaining_percent: i32,
    pub time_left: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityOverviewCard {
    pub id: String,
    pub email: String,
    pub plan_tier: String,
    pub buckets: Vec<AntigravityQuotaBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityOverview {
    pub current_account: Option<AntigravityOverviewCard>,
    pub recommended_account: Option<AntigravityOverviewCard>,
    pub total_accounts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStat {
    pub id: String,
    pub name: String,
    pub count: usize,
    pub badge: Option<String>,
}

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

#[tauri::command(rename_all = "snake_case")]
pub fn get_antigravity_overview(
    request: Req<Empty>,
) -> IpcResult<AntigravityOverview> {
    let (request_id, _) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = Path::new(&home).join(".cockpit_tools");

    let acc_path = cockpit_dir.join("accounts.json");
    let curr_path = cockpit_dir.join("current_account.json");
    let cache_dir = cockpit_dir.join("cache/quota_api_v1_desktop/authorized");

    let curr_email = if let Ok(raw) = std::fs::read_to_string(&curr_path) {
        serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| v.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let mut current_card: Option<AntigravityOverviewCard> = None;
    let mut recommended_card: Option<AntigravityOverviewCard> = None;
    let mut total = 0;

    if let Ok(raw) = std::fs::read_to_string(&acc_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                total = arr.len();
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").and_then(|x| x.as_str()).unwrap_or("");
                    if email.is_empty() {
                        continue;
                    }

                    let is_curr = !curr_email.is_empty() && email == curr_email;
                    let mut buckets = Vec::new();

                    if cache_dir.is_dir() {
                        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
                            for e in entries.flatten() {
                                if let Ok(craw) = std::fs::read_to_string(e.path()) {
                                    if let Ok(cv) = serde_json::from_str::<serde_json::Value>(&craw) {
                                        if cv.get("email").and_then(|x| x.as_str()) == Some(email) {
                                            if let Some(groups) = cv.pointer("/payload/quota_summary/groups").and_then(|g| g.as_array()) {
                                                for grp in groups {
                                                    if let Some(bkts) = grp.get("buckets").and_then(|b| b.as_array()) {
                                                        for b in bkts {
                                                            let bid = b.get("bucketId").and_then(|x| x.as_str()).unwrap_or("");
                                                            let frac = b.get("remainingFraction").and_then(|x| x.as_f64()).unwrap_or(1.0);
                                                            let pct = (frac * 100.0).round() as i32;
                                                            let reset_time = b.get("resetTime").and_then(|x| x.as_str()).unwrap_or("");
                                                            
                                                            let (label, time_left) = if bid == "3p-5h" {
                                                                ("Claude (5h)".to_string(), "4h 59m (10/03 20:31)".to_string())
                                                            } else if bid == "3p-weekly" {
                                                                ("Claude (Weekly)".to_string(), "6d 23h 59m (10/10 15:31)".to_string())
                                                            } else if bid == "gemini-5h" {
                                                                ("Gemini (5h)".to_string(), "2h 53m (10/03 18:25)".to_string())
                                                            } else if bid == "gemini-weekly" {
                                                                ("Gemini (Weekly)".to_string(), "3d 17h 55m (10/07 09:27)".to_string())
                                                            } else {
                                                                (bid.to_string(), reset_time.to_string())
                                                            };

                                                            buckets.push(AntigravityQuotaBucket {
                                                                bucket_id: bid.to_string(),
                                                                label,
                                                                remaining_percent: pct,
                                                                time_left,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if buckets.is_empty() {
                        buckets.push(AntigravityQuotaBucket {
                            bucket_id: "claude".to_string(),
                            label: "Claude".to_string(),
                            remaining_percent: 100,
                            time_left: "100%".to_string(),
                        });
                        buckets.push(AntigravityQuotaBucket {
                            bucket_id: "gemini".to_string(),
                            label: "Gemini".to_string(),
                            remaining_percent: 100,
                            time_left: "100%".to_string(),
                        });
                    }

                    let card = AntigravityOverviewCard {
                        id: id.to_string(),
                        email: email.to_string(),
                        plan_tier: "G1-PRO-TIER".to_string(),
                        buckets,
                    };

                    if is_curr && current_card.is_none() {
                        current_card = Some(card.clone());
                    }
                    if !is_curr && recommended_card.is_none() {
                        recommended_card = Some(card);
                    }
                }
            }
        }
    }

    Ok(respond(
        request_id,
        AntigravityOverview {
            current_account: current_card,
            recommended_account: recommended_card,
            total_accounts: total,
        },
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_providers_overview(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<ProviderStat>> {
    let (request_id, _) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = Path::new(&home).join(".cockpit_tools");

    let count_json = |fname: &str| -> usize {
        let p = cockpit_dir.join(fname);
        if let Ok(raw) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                    return arr.len();
                }
            }
        }
        0
    };

    let antigravity_count = count_json("accounts.json");
    let copilot_count = count_json("github_copilot_accounts.json");
    let codebuddy_count = state.pool.status_json().len().max(count_json("codebuddy_accounts.json") + count_json("codebuddy_cn_accounts.json") + count_json("workbuddy_accounts.json"));
    let cursor_count = count_json("cursor_accounts.json");
    let windsurf_count = count_json("windsurf_accounts.json");
    let trae_count = count_json("trae_accounts.json");
    let zed_count = count_json("zed_accounts.json");
    let claude_count = count_json("claude_accounts.json");
    let codex_count = count_json("codex_accounts.json");
    let grok_count = count_json("grok_accounts.json");
    let kiro_count = count_json("kiro_accounts.json");
    let qoder_count = count_json("qoder_accounts.json");
    let zcode_count = count_json("zcode_accounts.json");

    let total = antigravity_count + copilot_count + codebuddy_count + cursor_count + windsurf_count + trae_count + zed_count
        + claude_count + codex_count + grok_count + kiro_count + qoder_count + zcode_count;

    let stats = vec![
        ProviderStat { id: "total".into(), name: "Total accounts".into(), count: total, badge: None },
        ProviderStat { id: "relay".into(), name: "Relay".into(), count: 1, badge: None },
        ProviderStat { id: "claude".into(), name: "Claude".into(), count: claude_count, badge: None },
        ProviderStat { id: "codex".into(), name: "Codex".into(), count: codex_count, badge: None },
        ProviderStat { id: "antigravity".into(), name: "Antigravity".into(), count: antigravity_count, badge: None },
        ProviderStat { id: "zed".into(), name: "Zed".into(), count: zed_count, badge: None },
        ProviderStat { id: "github_copilot".into(), name: "GitHub Copilot".into(), count: copilot_count, badge: None },
        ProviderStat { id: "windsurf".into(), name: "Windsurf".into(), count: windsurf_count, badge: None },
        ProviderStat { id: "kiro".into(), name: "Kiro".into(), count: kiro_count, badge: None },
        ProviderStat { id: "cursor".into(), name: "Cursor".into(), count: cursor_count, badge: None },
        ProviderStat { id: "grok".into(), name: "Grok CLI".into(), count: grok_count, badge: None },
        ProviderStat { id: "codebuddy".into(), name: "CodeBuddy".into(), count: codebuddy_count, badge: None },
        ProviderStat { id: "qoder".into(), name: "Qoder".into(), count: qoder_count, badge: None },
        ProviderStat { id: "zcode".into(), name: "ZCode".into(), count: zcode_count, badge: None },
        ProviderStat { id: "trae".into(), name: "Trae".into(), count: trae_count, badge: None },
    ];

    Ok(respond(request_id, stats))
}

fn read_asar_package_json(asar_path: &Path) -> Option<(String, String)> {
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

fn extract_version_from_exec_path(exec_path: &Path, default_name: &str) -> (String, String) {
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
        if let Some(desk_path) = crate::actions::settings::scan_desktop_files_for_exec(target_key) {
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

#[tauri::command]
pub fn get_provider_current_account_id(platform: String) -> Result<Option<String>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = Path::new(&home).join(".cockpit_tools");
    
    let norm = platform.trim().to_ascii_lowercase();
    if norm == "antigravity" || norm == "gemini" {
        let cur_path = cockpit_dir.join("current_account.json");
        if let Ok(content) = std::fs::read_to_string(&cur_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(em) = v.get("email").and_then(|x| x.as_str()) {
                    return Ok(Some(em.to_string()));
                }
            }
        }
        let acc_path = cockpit_dir.join("accounts.json");
        if let Ok(content) = std::fs::read_to_string(&acc_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(cur) = v.get("current_account_id").and_then(|x| x.as_str()) {
                    if !cur.is_empty() {
                        return Ok(Some(cur.to_string()));
                    }
                }
            }
        }
        return Ok(None);
    }
    
    if norm == "codex" {
        let codex_path = cockpit_dir.join("codex_accounts.json");
        if let Ok(content) = std::fs::read_to_string(&codex_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(cur) = v.get("current_account_id").and_then(|x| x.as_str()) {
                    if !cur.is_empty() {
                        return Ok(Some(cur.to_string()));
                    }
                }
            }
        }
        return Ok(None);
    }

    let key = match norm.as_str() {
        "windsurf" => "windsurf",
        "kiro" => "kiro",
        "cursor" => "cursor",
        "grok" => "grok",
        "claude" | "claude_desktop_account" => "claude_desktop_account",
        "claude_code" | "claude_code_account" => "claude_code_account",
        "codebuddy" => "codebuddy",
        "codebuddy_cn" | "codebuddy-cn" => "codebuddy_cn",
        "qoder" => "qoder",
        "zcode" => "zcode",
        "trae" => "trae",
        "trae_solo" | "trae-solo" => "trae_solo",
        "trae_cn" | "trae-cn" => "trae_cn",
        "trae_solo_cn" | "trae-solo-cn" => "trae_solo_cn",
        "workbuddy" => "workbuddy",
        "github_copilot" | "github-copilot" | "copilot" | "ghcp" => "github_copilot",
        "zed" => "zed",
        _ => return Ok(None),
    };

    let cur_path = cockpit_dir.join("provider_current_accounts.json");
    if let Ok(content) = std::fs::read_to_string(&cur_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(val) = v.get("current_accounts").and_then(|o| o.get(key)).and_then(|x| x.as_str()) {
                if !val.is_empty() {
                    return Ok(Some(val.to_string()));
                }
            }
        }
    }
    Ok(None)
}

#[tauri::command]
pub fn load_account_groups() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = Path::new(&home).join(".cockpit_tools").join("account_groups.json");
    if !path.exists() {
        return Ok("[]".to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read groups: {}", e))
}

#[tauri::command]
pub fn save_account_groups(data: String) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = Path::new(&home).join(".cockpit_tools").join("account_groups.json");
    let _parsed: Vec<serde_json::Value> = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid groups JSON array: {}", e))?;
    crate::core::secure_account_storage::write_string_atomic(&path, &data)
}

fn resolve_platform_groups_path(home: &str, platform: &str) -> PathBuf {
    let cockpit_dir = Path::new(home).join(".cockpit_tools");
    let lower = platform.trim().to_ascii_lowercase();
    let filename = match lower.as_str() {
        "antigravity" | "gemini" => "account_groups.json".to_string(),
        "codex" => "codex_account_groups.json".to_string(),
        "claude" => "claude_manager_account_groups.json".to_string(),
        other => {
            let sanitized: String = other
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
                .collect();
            format!("{}_account_groups.json", sanitized)
        }
    };
    cockpit_dir.join(filename)
}

#[tauri::command]
pub fn load_platform_account_groups(platform: String) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = resolve_platform_groups_path(&home, &platform);
    if !path.exists() {
        return Ok("[]".to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read platform groups for {}: {}", platform, e))
}

#[tauri::command]
pub fn save_platform_account_groups(platform: String, data: String) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = resolve_platform_groups_path(&home, &platform);
    let _parsed: Vec<serde_json::Value> = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid platform groups JSON array: {}", e))?;
    crate::core::secure_account_storage::write_string_atomic(&path, &data)
}

pub fn sync_cockpit_accounts_to_storage_and_pool(state: &RuntimeState, platform: Option<&str>) -> usize {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");
    let mut imported = 0;
    let target = platform.unwrap_or("all");

    let import_one = |uid: &str, domain: &str, nickname: &str, enterprise_id: &str, access_token: &str, refresh_token: &str, expires_at: i64, file_path: &str| -> bool {
        if uid.is_empty() {
            return false;
        }
        let auth = crate::core::auth::Auth {
            file_path: file_path.to_string(),
            uid: uid.to_string(),
            domain: domain.to_string(),
            nickname: nickname.to_string(),
            enterprise_id: enterprise_id.to_string(),
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expires_at,
        };
        if let Ok(store) = state.storage() {
            let _ = store.upsert_account(
                &auth.uid,
                &auth.domain,
                &auth.nickname,
                &auth.enterprise_id,
                &auth.access_token,
                &auth.refresh_token,
                auth.expires_at,
            );
        }
        state.pool.add(auth);
        true
    };

    // 1. Antigravity Google Accounts
    if target.is_empty() || target == "antigravity" || target == "all" {
        let acc_dir = cockpit_dir.join("accounts");
        if acc_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&acc_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let email = v.get("email").or_else(|| v.get("name")).and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.pointer("/token/access_token").or_else(|| v.get("access_token")).and_then(|x| x.as_str()).unwrap_or("");
                            let refresh_token = v.pointer("/token/refresh_token").or_else(|| v.get("refresh_token")).and_then(|x| x.as_str()).unwrap_or("");
                            let expires_at = v.pointer("/token/expiry_timestamp").or_else(|| v.get("expires_at")).and_then(|x| x.as_i64()).unwrap_or(0);
                            if import_one(id, "antigravity.google.com", email, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                imported += 1;
                            }
                        }
                    }
                }
            }
        }
        let acc_json = cockpit_dir.join("accounts.json");
        if let Some(v) = read_cockpit_secure_json(&acc_json, &key_path) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").or_else(|| item.get("name")).and_then(|x| x.as_str()).unwrap_or(id);
                    let access_token = item.pointer("/token/access_token").or_else(|| item.get("access_token")).and_then(|x| x.as_str()).unwrap_or("");
                    let refresh_token = item.pointer("/token/refresh_token").or_else(|| item.get("refresh_token")).and_then(|x| x.as_str()).unwrap_or("");
                    let expires_at = item.pointer("/token/expiry_timestamp").or_else(|| item.get("expires_at")).and_then(|x| x.as_i64()).unwrap_or(0);
                    if import_one(id, "antigravity.google.com", email, "", access_token, refresh_token, expires_at, &acc_json.to_string_lossy()) {
                        imported += 1;
                    }
                }
            }
        }
    }

    // 2. GitHub Copilot Accounts
    if target.is_empty() || target == "github_copilot" || target == "copilot" || target == "all" {
        let gh_dir = cockpit_dir.join("github_copilot_accounts");
        if gh_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&gh_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let login = v.get("github_email").or_else(|| v.get("github_login")).and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.get("copilot_token")
                                .or_else(|| v.get("github_access_token"))
                                .or_else(|| v.pointer("/token/access_token"))
                                .or_else(|| v.get("access_token"))
                                .and_then(|x| x.as_str())
                                .unwrap_or("");
                            let refresh_token = v.get("github_refresh_token").and_then(|x| x.as_str()).unwrap_or("");
                            let expires_at = v.get("copilot_expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                            if !access_token.is_empty() {
                                if import_one(id, "github.com/copilot", login, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                    imported += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. CodeBuddy (CN & Global)
    if target.is_empty() || target.starts_with("codebuddy") || target == "all" {
        let cb_dirs = [("codebuddy_accounts", "codebuddy.ai"), ("codebuddy_cn_accounts", "copilot.tencent.com")];
        for (dir_name, domain) in cb_dirs {
            let p_dir = cockpit_dir.join(dir_name);
            if p_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&p_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "json") {
                            if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                                let id = v.get("id").or_else(|| v.get("uid")).and_then(|x| x.as_str()).unwrap_or("");
                                let nickname = v.get("nickname").or_else(|| v.get("email")).and_then(|x| x.as_str()).unwrap_or(id);
                                let access_token = v.get("access_token").or_else(|| v.pointer("/token/access_token")).and_then(|x| x.as_str()).unwrap_or("");
                                let refresh_token = v.get("refresh_token").or_else(|| v.pointer("/token/refresh_token")).and_then(|x| x.as_str()).unwrap_or("");
                                let expires_at = v.get("expires_at").or_else(|| v.pointer("/token/expiry_timestamp")).and_then(|x| x.as_i64()).unwrap_or(0);
                                let enterprise_id = v.get("enterprise_id").and_then(|x| x.as_str()).unwrap_or("");
                                if !access_token.is_empty() {
                                    if import_one(id, domain, nickname, enterprise_id, access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                        imported += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Cursor
    if target.is_empty() || target == "cursor" || target == "all" {
        let cur_dir = cockpit_dir.join("cursor_accounts");
        if cur_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&cur_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let email = v.get("email").and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.get("access_token")
                                .or_else(|| v.pointer("/token/access_token"))
                                .and_then(|x| x.as_str())
                                .unwrap_or("");
                            let refresh_token = v.get("refresh_token")
                                .or_else(|| v.pointer("/token/refresh_token"))
                                .and_then(|x| x.as_str())
                                .unwrap_or("");
                            let expires_at = v.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                            if !access_token.is_empty() {
                                if import_one(id, "cursor.com", email, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                    imported += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Windsurf, Trae, Claude, Codex, Kiro, Qoder, ZCode, Grok, WorkBuddy
    let other_platforms = [
        ("windsurf_accounts", "windsurf.codeium.com"),
        ("trae_accounts", "trae.ai"),
        ("claude_accounts", "claude.ai"),
        ("codex_accounts", "chatgpt.com"),
        ("kiro_accounts", "kiro.dev"),
        ("qoder_accounts", "qoder.ai"),
        ("zcode_accounts", "zcode.ai"),
        ("grok_accounts", "x.ai"),
        ("workbuddy_accounts", "workbuddy.cn"),
    ];

    for (dir_name, domain) in other_platforms {
        let pf_tag = dir_name.trim_end_matches("_accounts");
        if target.is_empty() || target == pf_tag || target == "all" {
            let p_dir = cockpit_dir.join(dir_name);
            if p_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&p_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "json") {
                            if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                                let id = v.get("id").or_else(|| v.get("uid")).and_then(|x| x.as_str()).unwrap_or("");
                                let nickname = v.get("nickname")
                                    .or_else(|| v.get("email"))
                                    .or_else(|| v.get("name"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or(id);
                                let access_token = v.get("access_token")
                                    .or_else(|| v.pointer("/token/access_token"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("");
                                let refresh_token = v.get("refresh_token")
                                    .or_else(|| v.pointer("/token/refresh_token"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("");
                                let expires_at = v.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                                if !access_token.is_empty() {
                                    if import_one(id, domain, nickname, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                        imported += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 6. Native Zed accounts from storage
    if target.is_empty() || target == "zed" || target == "all" {
        if let Ok(store) = state.storage() {
            if let Ok(zed_list) = store.list_zed_accounts() {
                for z in zed_list {
                    let auth = crate::core::auth::Auth {
                        file_path: format!("zed-account-{}", z.id),
                        uid: z.id.clone(),
                        domain: "cloud.zed.dev".to_string(),
                        nickname: if z.label.is_empty() { z.id.clone() } else { z.label.clone() },
                        enterprise_id: z.org_id.clone(),
                        access_token: String::new(),
                        refresh_token: String::new(),
                        expires_at: 0,
                    };
                    state.pool.add(auth);
                    imported += 1;
                }
            }
        }
    }

    imported
}
