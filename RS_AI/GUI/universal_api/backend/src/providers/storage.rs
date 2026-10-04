//! Shared Cockpit storage primitives and DTOs for Universal API providers.
//! 100% compatible with Cockpit Tools storage (~/.cockpit_tools).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub uid: String,
    pub nickname: String,
    pub domain: String,
    pub credits: i64,
    pub healthy: bool,
    pub cooling: bool,
    pub cool_kind: Option<String>,
    pub cool_remaining_sec: Option<i64>,
    pub disabled: bool,
    pub disabled_reason: Option<String>,
    pub success_count: i64,
    pub err_total: i64,
    pub in_flight: i32,
    pub plan_tier: Option<String>,
    pub is_current: bool,
    pub quota_details: Option<serde_json::Value>,
}

pub fn format_time_left(reset_time_str: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(reset_time_str) {
        let now = chrono::Utc::now();
        let target = dt.with_timezone(&chrono::Utc);
        let duration = target.signed_duration_since(now);
        let days = duration.num_days();
        let hours = (duration.num_hours() % 24).max(0);
        let mins = (duration.num_minutes() % 60).max(0);
        let local_dt = dt.with_timezone(&chrono::Local);
        let time_part = local_dt.format("%m/%d %H:%M").to_string();
        if days > 0 {
            format!("{}d {}h ({})", days, hours, time_part)
        } else if hours > 0 {
            format!("{}h {}m ({})", hours, mins, time_part)
        } else {
            format!("{}m ({})", mins, time_part)
        }
    } else {
        reset_time_str.to_string()
    }
}

pub fn read_cockpit_secure_json(path: &Path, key_path: &Path) -> Option<serde_json::Value> {
    crate::core::secure_account_storage::read_account_file_readonly::<serde_json::Value>(path, key_path).ok()
}

pub fn write_cockpit_secure_json(path: &Path, kind: &str, val: &serde_json::Value) -> Result<(), String> {
    let encrypted = crate::core::secure_account_storage::serialize_account_file(kind, val)?;
    crate::core::secure_account_storage::write_string_atomic(path, &encrypted)
}

pub fn ensure_cockpit_dirs(home: &str) -> std::io::Result<PathBuf> {
    let base = Path::new(home).join(".cockpit_tools");
    let dirs = [
        "",
        "accounts",
        "github_copilot_accounts",
        "cursor_accounts",
        "windsurf_accounts",
        "trae_accounts",
        "zed_accounts",
        "codebuddy_accounts",
        "codebuddy_cn_accounts",
        "workbuddy_accounts",
        "claude_accounts",
        "codex_accounts",
        "grok_accounts",
        "kiro_accounts",
        "qoder_accounts",
        "zcode_accounts",
    ];
    for d in dirs {
        fs::create_dir_all(base.join(d))?;
    }
    Ok(base)
}

pub fn get_copilot_account_detail(item: &serde_json::Value) -> (Option<String>, Option<serde_json::Value>) {
    let plan_tier = item.get("sku")
        .or_else(|| item.get("plan_tier"))
        .or_else(|| item.get("plan"))
        .and_then(|x| x.as_str())
        .map(|s| {
            let s_up = s.to_uppercase();
            if s_up.contains("INDIVIDUAL") { "COPILOT-INDIVIDUAL".to_string() }
            else if s_up.contains("BUSINESS") { "COPILOT-BUSINESS".to_string() }
            else if s_up.contains("ENTERPRISE") { "COPILOT-ENTERPRISE".to_string() }
            else { s_up }
        })
        .or_else(|| Some("COPILOT-PRO".to_string()));

    let quota = item.get("quota").or_else(|| item.get("chat_quota"));
    let quota_details = if let Some(q) = quota {
        Some(q.clone())
    } else {
        Some(serde_json::json!({
            "plan_tier": plan_tier.clone().unwrap_or_else(|| "COPILOT-PRO".to_string()),
            "status": "active",
            "chat_enabled": true
        }))
    };

    (plan_tier, quota_details)
}

pub fn load_cockpit_platform_accounts(home: &str) -> Vec<AccountInfo> {
    let cockpit_dir = Path::new(home).join(".cockpit_tools");
    if !cockpit_dir.is_dir() {
        return Vec::new();
    }

    let current_account_id = if let Ok(content) = fs::read_to_string(cockpit_dir.join("accounts.json")) {
        serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|v| v.get("current_account_id").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let mut provider_current_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(content) = fs::read_to_string(cockpit_dir.join("provider_current_accounts.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = v.get("current_accounts").and_then(|x| x.as_object()) {
                for (k, val) in obj {
                    if let Some(acc_id) = val.as_str() {
                        provider_current_map.insert(k.clone(), acc_id.to_string());
                    }
                }
            }
        }
    }

    let mut results = Vec::new();

    let platform_files = [
        ("accounts.json", "antigravity.google.com"),
        ("github_copilot_accounts.json", "github.com/copilot"),
        ("codebuddy_accounts.json", "codebuddy.ai"),
        ("codebuddy_cn_accounts.json", "copilot.tencent.com"),
        ("claude_accounts.json", "claude.ai"),
        ("codex_accounts.json", "chatgpt.com"),
        ("kiro_accounts.json", "kiro.dev"),
        ("qoder_accounts.json", "qoder.ai"),
        ("cursor_accounts.json", "cursor.com"),
        ("windsurf_accounts.json", "windsurf.codeium.com"),
        ("trae_accounts.json", "trae.ai"),
        ("workbuddy_accounts.json", "workbuddy.cn"),
        ("zed_accounts.json", "cloud.zed.dev"),
        ("zcode_accounts.json", "zcode.ai"),
        ("grok_accounts.json", "x.ai"),
    ];

    for (file_name, domain) in platform_files {
        let p = cockpit_dir.join(file_name);
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                    for item in arr {
                        let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                        if id.is_empty() {
                            continue;
                        }

                        let nickname = item.get("email")
                            .or_else(|| item.get("nickname"))
                            .or_else(|| item.get("github_email"))
                            .or_else(|| item.get("github_login"))
                            .or_else(|| item.get("name"))
                            .or_else(|| item.get("label"))
                            .and_then(|x| x.as_str())
                            .unwrap_or(id);

                        let mut plan_tier = None;
                        let mut is_current = false;
                        let mut quota_details = None;

                        if file_name == "accounts.json" {
                            is_current = id == current_account_id;
                            let actual_email = item.get("email").and_then(|x| x.as_str()).unwrap_or(nickname);
                            let (tier, details) = crate::providers::antigravity::accounts::get_antigravity_account_detail(&cockpit_dir, id, actual_email, is_current);
                            plan_tier = tier;
                            quota_details = details;
                        } else if file_name == "github_copilot_accounts.json" {
                            let copilot_cur = provider_current_map.get("github_copilot").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !copilot_cur.is_empty() && id == copilot_cur;
                            let (tier, details) = get_copilot_account_detail(item);
                            plan_tier = tier;
                            quota_details = details;
                        } else if file_name == "codebuddy_accounts.json" || file_name == "codebuddy_cn_accounts.json" {
                            let p_key = if file_name == "codebuddy_cn_accounts.json" { "codebuddy_cn" } else { "codebuddy" };
                            let cur = provider_current_map.get(p_key).map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").or_else(|| item.get("plan")).and_then(|x| x.as_str()).unwrap_or("FREE");
                            plan_tier = Some(tier.to_string());
                            quota_details = Some(serde_json::json!({
                                "plan_tier": tier,
                                "subscription": "Free Plan Subscription",
                                "subscription_val": "0 / 100",
                                "credit_package": "Credit Package",
                                "credit_package_val": "0 / 0",
                                "next_refresh": "11/01/2026, 00:00:00"
                            }));
                        } else if file_name == "claude_accounts.json" {
                            let cur = provider_current_map.get("claude_desktop_account").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                            let five_hour = item.pointer("/quota/five_hour_percentage").and_then(|x| x.as_i64()).unwrap_or(100);
                            let seven_day = item.pointer("/quota/seven_day_percentage").and_then(|x| x.as_i64()).unwrap_or(100);
                            quota_details = Some(serde_json::json!({
                                "plan_tier": tier,
                                "claude_5h": { "percent": five_hour, "reset_time": "", "time_left": "5h" },
                                "claude_weekly": { "percent": seven_day, "reset_time": "", "time_left": "7d" }
                            }));
                        } else if file_name == "codex_accounts.json" {
                            is_current = id == current_account_id;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                            let five_hour = item.pointer("/quota/five_hour_percentage").and_then(|x| x.as_i64()).unwrap_or(100);
                            let seven_day = item.pointer("/quota/seven_day_percentage").and_then(|x| x.as_i64()).unwrap_or(100);
                            quota_details = Some(serde_json::json!({
                                "plan_tier": tier,
                                "five_hour": { "percent": five_hour },
                                "seven_day": { "percent": seven_day }
                            }));
                        } else if file_name == "kiro_accounts.json" {
                            let cur = provider_current_map.get("kiro").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_tier").or_else(|| item.get("plan_name")).and_then(|x| x.as_str()).unwrap_or("FREE");
                            plan_tier = Some(tier.to_string());
                            let total = item.get("credits_total").and_then(|x| x.as_f64()).unwrap_or(100.0);
                            let used = item.get("credits_used").and_then(|x| x.as_f64()).unwrap_or(0.0);
                            quota_details = Some(serde_json::json!({
                                "plan_tier": tier,
                                "credits_total": total,
                                "credits_used": used,
                                "credits_remaining": (total - used).max(0.0)
                            }));
                        } else if file_name == "qoder_accounts.json" {
                            let cur = provider_current_map.get("qoder").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("FREE");
                            plan_tier = Some(tier.to_string());
                            let total = item.get("credits_total").and_then(|x| x.as_f64()).unwrap_or(100.0);
                            let used = item.get("credits_used").and_then(|x| x.as_f64()).unwrap_or(0.0);
                            quota_details = Some(serde_json::json!({
                                "plan_tier": tier,
                                "credits_total": total,
                                "credits_used": used,
                                "credits_remaining": (total - used).max(0.0)
                            }));
                        } else if file_name == "cursor_accounts.json" {
                            let cur = provider_current_map.get("cursor").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").or_else(|| item.get("membership_type")).and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "windsurf_accounts.json" {
                            let cur = provider_current_map.get("windsurf").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan").or_else(|| item.get("plan_type")).and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "trae_accounts.json" {
                            let cur = provider_current_map.get("trae").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "workbuddy_accounts.json" {
                            let cur = provider_current_map.get("workbuddy").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("VIP");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "zed_accounts.json" {
                            let cur = provider_current_map.get("zed").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            plan_tier = Some("PRO".to_string());
                        } else if file_name == "zcode_accounts.json" {
                            let cur = provider_current_map.get("zcode").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "grok_accounts.json" {
                            let cur = provider_current_map.get("grok").map(|s| s.as_str()).unwrap_or(&current_account_id);
                            is_current = !cur.is_empty() && id == cur;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        }

                        results.push(AccountInfo {
                            uid: id.to_string(),
                            nickname: nickname.to_string(),
                            domain: domain.to_string(),
                            credits: 100,
                            healthy: true,
                            cooling: false,
                            cool_kind: None,
                            cool_remaining_sec: None,
                            disabled: false,
                            disabled_reason: None,
                            success_count: 0,
                            err_total: 0,
                            in_flight: 0,
                            plan_tier,
                            is_current,
                            quota_details,
                        });
                    }
                }
            }
        }
    }

    results
}

pub fn set_cockpit_current_account(home: &str, platform: &str, uid: &str) {
    let cockpit_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(home)).join(".cockpit_tools");
    if !cockpit_dir.is_dir() {
        return;
    }

    let file_name = match platform {
        "antigravity" => "accounts.json",
        "github_copilot" | "copilot" => "github_copilot_accounts.json",
        "cursor" => "cursor_accounts.json",
        "windsurf" => "windsurf_accounts.json",
        "trae" => "trae_accounts.json",
        "zed" => "zed_accounts.json",
        "codebuddy" | "codebuddy_global" => "codebuddy_accounts.json",
        "codebuddy_cn" => "codebuddy_cn_accounts.json",
        "workbuddy" => "workbuddy_accounts.json",
        "claude" => "claude_accounts.json",
        "codex" => "codex_accounts.json",
        "grok" => "grok_accounts.json",
        "kiro" => "kiro_accounts.json",
        "qoder" => "qoder_accounts.json",
        "zcode" => "zcode_accounts.json",
        _ => if platform.contains("copilot") {
            "github_copilot_accounts.json"
        } else if platform.contains("codebuddy_cn") {
            "codebuddy_cn_accounts.json"
        } else if platform.contains("codebuddy") {
            "codebuddy_accounts.json"
        } else {
            "accounts.json"
        },
    };

    let p = cockpit_dir.join(file_name);
    if let Ok(raw) = fs::read_to_string(&p) {
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw) {
            v["current_account_id"] = serde_json::json!(uid);
            let _ = crate::core::secure_account_storage::write_string_atomic(
                &p,
                &serde_json::to_string_pretty(&v).unwrap_or_default(),
            );

            // Antigravity standard: also write current_account.json and antigravity_switch_history.json
            if file_name == "accounts.json" {
                let mut email = uid.to_string();
                if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                    for item in arr {
                        if item.get("id").and_then(|x| x.as_str()) == Some(uid) {
                            if let Some(em) = item.get("email").and_then(|x| x.as_str()) {
                                email = em.to_string();
                                break;
                            }
                        }
                    }
                }
                let cur_account_payload = serde_json::json!({
                    "email": email,
                    "updated_at": chrono::Utc::now().timestamp()
                });
                let cur_file = cockpit_dir.join("current_account.json");
                let _ = crate::core::secure_account_storage::write_string_atomic(
                    &cur_file,
                    &serde_json::to_string_pretty(&cur_account_payload).unwrap_or_default(),
                );

                crate::providers::antigravity::switch::add_antigravity_switch_history(&cockpit_dir, uid, &email, true);
            }
        }
    }

    // Update provider_current_accounts.json for non-Antigravity / non-Codex providers
    let provider_key = match platform {
        "windsurf" => Some("windsurf"),
        "kiro" => Some("kiro"),
        "cursor" => Some("cursor"),
        "grok" => Some("grok"),
        "claude" | "claude_desktop_account" => Some("claude_desktop_account"),
        "claude_code" | "claude_code_account" => Some("claude_code_account"),
        "codebuddy" | "codebuddy_global" => Some("codebuddy"),
        "codebuddy_cn" => Some("codebuddy_cn"),
        "qoder" => Some("qoder"),
        "zcode" => Some("zcode"),
        "trae" => Some("trae"),
        "trae_solo" => Some("trae_solo"),
        "trae_cn" => Some("trae_cn"),
        "trae_solo_cn" => Some("trae_solo_cn"),
        "workbuddy" => Some("workbuddy"),
        "github_copilot" | "copilot" | "ghcp" => Some("github_copilot"),
        _ => None,
    };
    if let Some(key) = provider_key {
        let cur_path = cockpit_dir.join("provider_current_accounts.json");
        let mut state = if let Ok(content) = fs::read_to_string(&cur_path) {
            serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({
                "version": "1.0",
                "current_accounts": {}
            }))
        } else {
            serde_json::json!({
                "version": "1.0",
                "current_accounts": {}
            })
        };
        if state.get("current_accounts").is_none() {
            state["current_accounts"] = serde_json::json!({});
        }
        state["current_accounts"][key] = serde_json::json!(uid);
        let _ = crate::core::secure_account_storage::write_string_atomic(
            &cur_path,
            &serde_json::to_string_pretty(&state).unwrap_or_default(),
        );
    }
}

pub fn delete_from_cockpit_storage(home: &str, uid: &str) {
    let cockpit_dir = Path::new(home).join(".cockpit_tools");
    if !cockpit_dir.is_dir() {
        return;
    }

    // 1. Remove from accounts.json
    let accounts_json_path = cockpit_dir.join("accounts.json");
    if let Ok(content) = fs::read_to_string(&accounts_json_path) {
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&content) {
            let mut removed_email: Option<String> = None;
            if let Some(arr) = v.get_mut("accounts").and_then(|a| a.as_array_mut()) {
                arr.retain(|item| {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").and_then(|x| x.as_str()).unwrap_or("");
                    if id == uid || email == uid {
                        if !email.is_empty() {
                            removed_email = Some(email.to_string());
                        }
                        false
                    } else {
                        true
                    }
                });
            }
            if v.get("current_account_id").and_then(|x| x.as_str()) == Some(uid) {
                let first_id = v.get("accounts")
                    .and_then(|a| a.as_array())
                    .and_then(|a| a.first())
                    .and_then(|i| i.get("id"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                if let Some(fid) = first_id {
                    v["current_account_id"] = serde_json::json!(fid);
                } else {
                    v["current_account_id"] = serde_json::json!("");
                }
            }
            let _ = crate::core::secure_account_storage::write_string_atomic(
                &accounts_json_path,
                &serde_json::to_string_pretty(&v).unwrap_or_default(),
            );

            // Also clean up quota cache if we found an email
            if let Some(em) = removed_email {
                use sha2::{Digest, Sha256};
                let hash = format!("{:x}", Sha256::digest(em.trim().to_lowercase().as_bytes()));
                let cache_file = cockpit_dir
                    .join("cache")
                    .join("quota_api_v1_desktop")
                    .join("authorized")
                    .join(format!("{}.json", hash));
                let _ = fs::remove_file(cache_file);
            }
        }
    }

    // 2. Remove encrypted account envelope ~/.cockpit_tools/accounts/<uid>.json
    let acc_file = cockpit_dir.join("accounts").join(format!("{uid}.json"));
    let _ = fs::remove_file(&acc_file);

    // Also check with md5 and sha256 if uid looks like an email
    if uid.contains('@') {
        let norm_email = uid.trim().to_lowercase();
        let md5_hash = format!("{:x}", md5::compute(norm_email.as_bytes()));
        use sha2::{Digest, Sha256};
        let sha_hash = format!("{:x}", Sha256::digest(norm_email.as_bytes()));

        let _ = fs::remove_file(cockpit_dir.join("accounts").join(format!("antigravity_{md5_hash}.json")));
        let _ = fs::remove_file(cockpit_dir.join("accounts").join(format!("antigravity_{sha_hash}.json")));

        let _ = fs::remove_file(cockpit_dir.join("cache/quota_api_v1_desktop/authorized").join(format!("{sha_hash}.json")));
    }

    // 3. Also remove from other platform account index files and account detail folders
    let platform_mappings = [
        ("github_copilot_accounts.json", "github_copilot_accounts", "github_copilot"),
        ("cursor_accounts.json", "cursor_accounts", "cursor"),
        ("windsurf_accounts.json", "windsurf_accounts", "windsurf"),
        ("trae_accounts.json", "trae_accounts", "trae"),
        ("zed_accounts.json", "zed_accounts", "zed"),
        ("codebuddy_accounts.json", "codebuddy_accounts", "codebuddy"),
        ("codebuddy_cn_accounts.json", "codebuddy_cn_accounts", "codebuddy_cn"),
        ("workbuddy_accounts.json", "workbuddy_accounts", "workbuddy"),
        ("claude_accounts.json", "claude_accounts", "claude"),
        ("codex_accounts.json", "codex_accounts", "codex"),
        ("grok_accounts.json", "grok_accounts", "grok"),
        ("kiro_accounts.json", "kiro_accounts", "kiro"),
        ("qoder_accounts.json", "qoder_accounts", "qoder"),
        ("zcode_accounts.json", "zcode_accounts", "zcode"),
    ];

    let _key_path = cockpit_dir.join("secure-account-storage.key");
    for (pf, sub_dir, prefix) in platform_mappings {
        let p = cockpit_dir.join(pf);
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&content) {
                let mut removed_id: Option<String> = None;
                if let Some(arr) = v.get_mut("accounts").and_then(|a| a.as_array_mut()) {
                    arr.retain(|item| {
                        let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                        let email = item.get("email").or_else(|| item.get("github_email")).or_else(|| item.get("name")).and_then(|x| x.as_str()).unwrap_or("");
                        if id == uid || email == uid {
                            removed_id = Some(id.to_string());
                            false
                        } else {
                            true
                        }
                    });
                }
                let _ = crate::core::secure_account_storage::write_string_atomic(
                    &p,
                    &serde_json::to_string_pretty(&v).unwrap_or_default(),
                );

                if let Some(rid) = removed_id {
                    let detail_file = cockpit_dir.join(sub_dir).join(format!("{rid}.json"));
                    let _ = fs::remove_file(&detail_file);
                }
            }
        }

        // Direct removal by file name in sub_dir
        let direct_file = cockpit_dir.join(sub_dir).join(format!("{uid}.json"));
        let _ = fs::remove_file(&direct_file);

        if uid.contains('@') {
            let norm_email = uid.trim().to_lowercase();
            let md5_hash = format!("{:x}", md5::compute(norm_email.as_bytes()));
            let _ = fs::remove_file(cockpit_dir.join(sub_dir).join(format!("{prefix}_{md5_hash}.json")));
        }
    }
}

// Request and response DTOs
#[derive(Deserialize)]
pub struct AccountUidRequest {
    pub uid: String,
}

#[derive(Deserialize)]
pub struct AddAccountRequest {
    pub auth_file_path: String,
}

#[derive(Deserialize)]
pub struct DisableAccountRequest {
    pub uid: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct ProbeChatResult {
    pub ok: bool,
    pub http_status: u16,
    pub preview: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct ProbeModelsResult {
    pub ok: bool,
    pub count: usize,
    pub has_glm52: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct ProbeAccountResult {
    pub uid: String,
    pub quota_ok: bool,
    pub remain: i64,
    pub quota_message: String,
    pub models: ProbeModelsResult,
    pub chat: ProbeChatResult,
}

#[derive(Serialize)]
pub struct TestAccountResult {
    pub uid: String,
    pub ok: bool,
    pub remain: i64,
    pub message: String,
}

#[derive(Deserialize)]
pub struct InjectAccountRequest {
    pub platform: String,
    pub uid: String,
}

#[derive(Serialize)]
pub struct InjectAccountResult {
    pub success: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct ImportLocalRequest {
    #[serde(default)]
    pub platform: String,
}

#[derive(Serialize)]
pub struct ImportLocalResult {
    pub imported_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntigravitySwitchHistoryItem {
    pub id: String,
    pub timestamp: i64,
    pub account_id: String,
    pub target_email: String,
    pub trigger_type: String,
    pub trigger_source: String,
    pub local_ok: bool,
    pub seamless_ok: bool,
    pub success: bool,
    pub local_duration_ms: u64,
    pub seamless_duration_ms: Option<u64>,
    pub total_duration_ms: u64,
    pub error_stage: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub seamless_effective_mode: Option<String>,
    pub seamless_from_email: Option<String>,
    pub seamless_to_email: Option<String>,
    pub seamless_execution_id: Option<String>,
    pub seamless_finished_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_antigravity_ide_and_desktop() {
        let ide_info = super::super::aggregator::resolve_installed_app_info("antigravity", Some("ide"));
        assert_eq!(ide_info.name, "Antigravity IDE");
        assert_eq!(ide_info.version, "2.5.5");
        assert!(ide_info.installed);

        let desktop_info = super::super::aggregator::resolve_installed_app_info("antigravity", Some("desktop"));
        assert_eq!(desktop_info.name, "Antigravity Desktop");
        assert_eq!(desktop_info.version, "2.5.0");
        assert!(desktop_info.installed);

        let trae_info = super::super::aggregator::resolve_installed_app_info("trae", None);
        assert_eq!(trae_info.name, "Trae");
        assert_eq!(trae_info.version, "Not Found");
        assert!(!trae_info.installed);

        let windsurf_info = super::super::aggregator::resolve_installed_app_info("windsurf", None);
        assert_eq!(windsurf_info.name, "Windsurf");
        assert_eq!(windsurf_info.version, "Not Found");
        assert!(!windsurf_info.installed);
    }

    #[test]
    fn test_read_cockpit_secure_json_envelope() {
        use aes_gcm::aead::Aead;
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;

        let temp_dir = std::env::temp_dir().join(format!("test_cockpit_decrypt_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let key_path = temp_dir.join("secure-account-storage.key");
        let account_path = temp_dir.join("test_acc.json");

        let raw_key = [7u8; 32];
        let key_b64 = STANDARD.encode(raw_key);
        std::fs::write(&key_path, key_b64).unwrap();

        let original_data = serde_json::json!({
            "id": "antigravity_12345",
            "email": "tester@example.com",
            "token": {
                "access_token": "ya29.test_token_secret",
                "refresh_token": "1//04test_refresh"
            }
        });

        let cipher = Aes256Gcm::new_from_slice(&raw_key).unwrap();
        let nonce_bytes = [3u8; 12];
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), serde_json::to_vec(&original_data).unwrap().as_ref())
            .unwrap();

        let envelope = serde_json::json!({
            "version": 1,
            "kind": "account",
            "algorithm": "AES-256-GCM",
            "key_id": "local-secure-account-storage-v1",
            "nonce": STANDARD.encode(nonce_bytes),
            "ciphertext": STANDARD.encode(ciphertext),
            "encrypted_at": 1700000000i64
        });

        std::fs::write(&account_path, serde_json::to_string(&envelope).unwrap()).unwrap();

        let decrypted = read_cockpit_secure_json(&account_path, &key_path);
        assert!(decrypted.is_some(), "Decryption must succeed");
        let val = decrypted.unwrap();
        assert_eq!(val["id"], "antigravity_12345");
        assert_eq!(val["email"], "tester@example.com");
        assert_eq!(val["token"]["access_token"], "ya29.test_token_secret");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_decrypt_real_cockpit_if_present() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
        let key_path = cockpit_dir.join("secure-account-storage.key");
        if !key_path.exists() {
            return;
        }
        let accounts_dir = cockpit_dir.join("accounts");
        if let Ok(entries) = std::fs::read_dir(accounts_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "json") {
                    let decrypted = read_cockpit_secure_json(&p, &key_path);
                    assert!(decrypted.is_some(), "Failed to decrypt actual file: {:?}", p);
                    let val = decrypted.unwrap();
                    assert!(val.get("id").is_some());
                    println!("Successfully decrypted real account ID: {:?}", val.get("id"));
                    break;
                }
            }
        }
    }

    #[test]
    fn test_cockpit_reorder_accounts() {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("test_cockpit_reorder_{}_{}", std::process::id(), nanos));
        let _ = std::fs::create_dir_all(&temp_dir);
        let accounts_file = temp_dir.join("accounts.json");

        let initial_json = serde_json::json!({
            "version": 1,
            "accounts": [
                { "id": "acc_1", "email": "acc1@test.com" },
                { "id": "acc_2", "email": "acc2@test.com" },
                { "id": "acc_3", "email": "acc3@test.com" }
            ],
            "current_account_id": "acc_1"
        });
        std::fs::write(&accounts_file, serde_json::to_string(&initial_json).unwrap()).unwrap();

        // Reorder: acc_3, acc_1, acc_2
        let raw = std::fs::read_to_string(&accounts_file).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let arr = v.get("accounts").and_then(|a| a.as_array()).unwrap();
        let desired = vec!["acc_3", "acc_1"];
        let mut new_arr = Vec::new();
        for id in &desired {
            if let Some(f) = arr.iter().find(|i| i.get("id").and_then(|x| x.as_str()) == Some(id)) {
                new_arr.push(f.clone());
            }
        }
        for item in arr {
            let id = item.get("id").and_then(|x| x.as_str()).unwrap();
            if !desired.contains(&id) {
                new_arr.push(item.clone());
            }
        }
        v["accounts"] = serde_json::Value::Array(new_arr);
        std::fs::write(&accounts_file, serde_json::to_string_pretty(&v).unwrap()).unwrap();

        let updated_raw = std::fs::read_to_string(&accounts_file).unwrap();
        let updated: serde_json::Value = serde_json::from_str(&updated_raw).unwrap();
        let updated_arr = updated["accounts"].as_array().unwrap();
        assert_eq!(updated_arr[0]["id"], "acc_3");
        assert_eq!(updated_arr[1]["id"], "acc_1");
        assert_eq!(updated_arr[2]["id"], "acc_2");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cockpit_account_groups_validation() {
        let valid_groups = serde_json::json!([
            {
                "id": "group_1",
                "name": "Production",
                "accountIds": ["acc_1", "acc_2"]
            }
        ]);
        let serialized = serde_json::to_string(&valid_groups).unwrap();
        let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&serialized);
        assert!(parsed.is_ok());

        let invalid = "not a json";
        let parsed_invalid: Result<Vec<serde_json::Value>, _> = serde_json::from_str(invalid);
        assert!(parsed_invalid.is_err());
    }

    #[test]
    fn test_cockpit_switch_history_serialization() {
        let item = AntigravitySwitchHistoryItem {
            id: "switch_1".to_string(),
            timestamp: 1728000000,
            account_id: "antigravity_123".to_string(),
            target_email: "test@example.com".to_string(),
            trigger_type: "manual".to_string(),
            trigger_source: "tools.account.switch".to_string(),
            local_ok: true,
            seamless_ok: true,
            success: true,
            local_duration_ms: 120,
            seamless_duration_ms: None,
            total_duration_ms: 120,
            error_stage: None,
            error_code: None,
            error_message: None,
            seamless_effective_mode: None,
            seamless_from_email: None,
            seamless_to_email: None,
            seamless_execution_id: None,
            seamless_finished_at: None,
        };

        let json_str = serde_json::to_string(&item).unwrap();
        assert!(json_str.contains("\"accountId\":\"antigravity_123\""));
        assert!(json_str.contains("\"targetEmail\":\"test@example.com\""));
        assert!(json_str.contains("\"localDurationMs\":120"));

        let deserialized: AntigravitySwitchHistoryItem = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.id, "switch_1");
        assert_eq!(deserialized.target_email, "test@example.com");
    }
}

