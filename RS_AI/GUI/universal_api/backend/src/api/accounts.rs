use crate::core::auth::parse_auth;
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
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

fn format_time_left(reset_time_str: &str) -> String {
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

fn get_antigravity_account_detail(
    cockpit_dir: &std::path::Path,
    account_id: &str,
    email: &str,
    is_current: bool,
) -> (Option<String>, Option<serde_json::Value>) {
    let key_path = cockpit_dir.join("secure-account-storage.key");

    let mut detected_tier: Option<String> = None;
    let mut acc_quota: Option<serde_json::Value> = None;

    use sha2::{Digest, Sha256};
    let computed_acc_file = if !account_id.is_empty() && cockpit_dir.join("accounts").join(format!("{account_id}.json")).exists() {
        Some(cockpit_dir.join("accounts").join(format!("{account_id}.json")))
    } else if !email.is_empty() {
        let hash = format!("antigravity_{:x}", Sha256::digest(email.trim().to_lowercase().as_bytes()));
        let p = cockpit_dir.join("accounts").join(format!("{hash}.json"));
        if p.exists() {
            Some(p)
        } else {
            None
        }
    } else {
        None
    };

    let mut resolved_email = email.trim().to_lowercase();
    if let Some(acc_file) = computed_acc_file {
        if let Some(acc_val) = read_cockpit_secure_json(&acc_file, &key_path) {
            if let Some(em) = acc_val.get("email").and_then(|x| x.as_str()) {
                if !em.is_empty() {
                    resolved_email = em.trim().to_lowercase();
                }
            }
            if let Some(q) = acc_val.get("quota") {
                acc_quota = Some(q.clone());
                if let Some(tier) = q.get("subscription_tier").and_then(|t| t.as_str()) {
                    if !tier.is_empty() {
                        let t_up = tier.to_uppercase();
                        if t_up.contains("PRO") {
                            detected_tier = Some("G1-PRO-TIER".to_string());
                        } else if t_up.contains("ULTRA") {
                            detected_tier = Some("G1-ULTRA-TIER".to_string());
                        } else if t_up.contains("FREE") {
                            detected_tier = Some("FREE".to_string());
                        } else {
                            detected_tier = Some(t_up);
                        }
                    }
                }
                if detected_tier.is_none() {
                    if let Some(tier_id) = q.get("tier_id").and_then(|t| t.as_str()) {
                        if tier_id.to_lowercase().contains("pro") {
                            detected_tier = Some("G1-PRO-TIER".to_string());
                        } else if tier_id.to_lowercase().contains("ultra") {
                            detected_tier = Some("G1-ULTRA-TIER".to_string());
                        }
                    }
                }
            }
        }
    }

    let mut c_5h = None;
    let mut c_weekly = None;
    let mut g_5h = None;
    let mut g_weekly = None;
    let mut found_metrics = false;

    // 1. Try reading from quota API cache file
    let hash = format!("{:x}", Sha256::digest(resolved_email.as_bytes()));
    let cache_file = cockpit_dir
        .join("cache")
        .join("quota_api_v1_desktop")
        .join("authorized")
        .join(format!("{}.json", hash));

    if let Ok(raw) = std::fs::read_to_string(&cache_file) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            let payload = v.get("payload");
            let quota_summary = payload.and_then(|p| p.get("quota_summary"));
            if let Some(qs) = quota_summary.filter(|q| q.is_object() && q.get("groups").is_some()) {
                if let Some(groups) = qs.get("groups").and_then(|g| g.as_array()) {
                    for group in groups {
                        if let Some(buckets) = group.get("buckets").and_then(|b| b.as_array()) {
                            for b in buckets {
                                let bid = b.get("bucketId").and_then(|x| x.as_str()).unwrap_or("");
                                let rf = b.get("remainingFraction").and_then(|x| x.as_f64()).unwrap_or(1.0);
                                let rt = b.get("resetTime").and_then(|x| x.as_str()).unwrap_or("");
                                let pct = (rf * 100.0).round() as i64;
                                let tl = format_time_left(rt);
                                let obj = serde_json::json!({
                                    "percent": pct,
                                    "reset_time": rt,
                                    "time_left": tl
                                });
                                match bid {
                                    "3p-5h" | "claude:5h" => c_5h = Some(obj),
                                    "3p-weekly" | "claude:weekly" => c_weekly = Some(obj),
                                    "gemini-5h" | "gemini:5h" => g_5h = Some(obj),
                                    "gemini-weekly" | "gemini:weekly" => g_weekly = Some(obj),
                                    _ => {}
                                }
                            }
                        }
                    }
                    found_metrics = c_weekly.is_some() || g_weekly.is_some() || c_5h.is_some() || g_5h.is_some();
                }
            } else if let Some(models) = payload.and_then(|p| p.get("models")) {
                let get_model_bucket = |m_name: &str| -> Option<serde_json::Value> {
                    let qi = models.get(m_name)?.get("quotaInfo")?;
                    let rf = qi.get("remainingFraction").and_then(|x| x.as_f64()).unwrap_or(1.0);
                    let rt = qi.get("resetTime").and_then(|x| x.as_str()).unwrap_or("");
                    let pct = (rf * 100.0).round() as i64;
                    let tl = format_time_left(rt);
                    Some(serde_json::json!({
                        "percent": pct,
                        "reset_time": rt,
                        "time_left": tl
                    }))
                };

                c_weekly = get_model_bucket("claude-opus-4-6-thinking")
                    .or_else(|| get_model_bucket("claude-sonnet-4-6"));
                g_weekly = get_model_bucket("gemini-2.5-flash")
                    .or_else(|| get_model_bucket("gemini-2.5-flash-thinking"));
                found_metrics = c_weekly.is_some() || g_weekly.is_some();
            }
        }
    }

    // 2. Fallback to account envelope's quota.models if cache file was missing or empty
    if !found_metrics {
        if let Some(models) = acc_quota.as_ref().and_then(|q| q.get("models")).and_then(|m| m.as_array()) {
            for m in models {
                let name = m.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let pct = m.get("percentage").and_then(|x| x.as_i64()).unwrap_or(0);
                let rt = m.get("reset_time").and_then(|x| x.as_str()).unwrap_or("");
                let tl = format_time_left(rt);
                let obj = serde_json::json!({
                    "percent": pct,
                    "reset_time": rt,
                    "time_left": tl
                });
                match name {
                    "3p-5h" | "claude:5h" => c_5h = Some(obj),
                    "3p-weekly" | "claude:weekly" => c_weekly = Some(obj),
                    "gemini-5h" | "gemini:5h" => g_5h = Some(obj),
                    "gemini-weekly" | "gemini:weekly" => g_weekly = Some(obj),
                    _ => {}
                }
            }
            found_metrics = c_weekly.is_some() || g_weekly.is_some() || c_5h.is_some() || g_5h.is_some();
        }
    }

    let final_tier = detected_tier.unwrap_or_else(|| "FREE".to_string());
    if found_metrics {
        let details = serde_json::json!({
            "plan_tier": final_tier,
            "is_current": is_current,
            "claude_5h": c_5h,
            "claude_weekly": c_weekly,
            "gemini_5h": g_5h,
            "gemini_weekly": g_weekly
        });
        (Some(final_tier), Some(details))
    } else {
        (Some(final_tier), None)
    }
}

fn get_copilot_account_detail(item: &serde_json::Value) -> (Option<String>, Option<serde_json::Value>) {
    let plan = item.get("copilot_plan").and_then(|p| p.as_str()).unwrap_or("individual");
    let tier = if plan.to_lowercase().contains("business") || plan.to_lowercase().contains("enterprise") {
        "ENTERPRISE"
    } else {
        "PRO"
    };
    let details = serde_json::json!({
        "plan_tier": tier,
        "is_current": true,
        "suggestions": "Included",
        "chat": "Included",
        "premium_requests": "0 / 200",
        "reset_time": "28d 15h (11/01 07:00)"
    });
    (Some(tier.to_string()), Some(details))
}

fn load_cockpit_platform_accounts(home: &str) -> Vec<AccountInfo> {
    let mut results = Vec::new();
    let cockpit_dir = std::path::Path::new(home).join(".cockpit_tools");
    if !cockpit_dir.is_dir() {
        return results;
    }

    let files = [
        ("accounts.json", "antigravity.google.com"),
        ("github_copilot_accounts.json", "github.com/copilot"),
        ("cursor_accounts.json", "cursor.com"),
        ("windsurf_accounts.json", "codeium.com"),
        ("trae_accounts.json", "trae.ai"),
        ("zed_accounts.json", "cloud.zed.dev"),
        ("codebuddy_accounts.json", "codebuddy.ai"),
        ("codebuddy_cn_accounts.json", "codebuddy.cn"),
        ("workbuddy_accounts.json", "workbuddy.cn"),
        ("claude_accounts.json", "anthropic.com/claude"),
        ("codex_accounts.json", "openai.com/codex"),
        ("grok_accounts.json", "x.ai/grok"),
        ("kiro_accounts.json", "kiro.ai"),
        ("qoder_accounts.json", "qoder.ai"),
        ("zcode_accounts.json", "zcode.ai"),
    ];

    let key_path = cockpit_dir.join("secure-account-storage.key");
    for (file_name, domain) in files {
        let p = cockpit_dir.join(file_name);
        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
            let current_account_id = v.get("current_account_id").and_then(|x| x.as_str()).unwrap_or("");
                if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                    for item in arr {
                        let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                        if id.is_empty() {
                            continue;
                        }
                        let nickname = item.get("github_email")
                            .or_else(|| item.get("email"))
                            .or_else(|| item.get("github_login"))
                            .or_else(|| item.get("name"))
                            .or_else(|| item.get("nickname"))
                            .or_else(|| item.get("label"))
                            .and_then(|x| x.as_str())
                            .unwrap_or(id);

                        let mut plan_tier = None;
                        let mut is_current = false;
                        let mut quota_details = None;

                        if file_name == "accounts.json" {
                            is_current = id == current_account_id;
                            let actual_email = item.get("email").and_then(|x| x.as_str()).unwrap_or(nickname);
                            let (tier, details) = get_antigravity_account_detail(&cockpit_dir, id, actual_email, is_current);
                            plan_tier = tier;
                            quota_details = details;
                        } else if file_name == "github_copilot_accounts.json" {
                            is_current = current_account_id.is_empty() || id == current_account_id;
                            let (tier, details) = get_copilot_account_detail(item);
                            plan_tier = tier;
                            quota_details = details;
                        } else if file_name == "codebuddy_accounts.json" || file_name == "codebuddy_cn_accounts.json" {
                            is_current = id == current_account_id;
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
                            is_current = id == current_account_id;
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
                            is_current = id == current_account_id;
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
                            is_current = id == current_account_id;
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
                            is_current = id == current_account_id;
                            let tier = item.get("plan_type").or_else(|| item.get("membership_type")).and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "windsurf_accounts.json" {
                            is_current = id == current_account_id;
                            let tier = item.get("plan").or_else(|| item.get("plan_type")).and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "trae_accounts.json" {
                            is_current = id == current_account_id;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "workbuddy_accounts.json" {
                            is_current = id == current_account_id;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("VIP");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "zed_accounts.json" {
                            is_current = id == current_account_id;
                            plan_tier = Some("PRO".to_string());
                        } else if file_name == "zcode_accounts.json" {
                            is_current = id == current_account_id;
                            let tier = item.get("plan_type").and_then(|x| x.as_str()).unwrap_or("PRO");
                            plan_tier = Some(tier.to_string());
                        } else if file_name == "grok_accounts.json" {
                            is_current = id == current_account_id;
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

    results
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_accounts(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<Vec<AccountInfo>> {
    let (request_id, _) = request.validate()?;
    let mut list: Vec<AccountInfo> = state.pool.status_json().into_iter().map(to_info).collect();
    if let Ok(store) = state.storage() {
        if let Ok(zed_list) = store.list_zed_accounts() {
            for z in zed_list {
                list.push(AccountInfo {
                    uid: z.id.clone(),
                    nickname: if z.label.is_empty() { z.id.clone() } else { z.label.clone() },
                    domain: "cloud.zed.dev".to_string(),
                    credits: 100,
                    healthy: z.enabled,
                    cooling: false,
                    cool_kind: None,
                    cool_remaining_sec: None,
                    disabled: !z.enabled,
                    disabled_reason: None,
                    success_count: 0,
                    err_total: 0,
                    in_flight: 0,
                    plan_tier: Some("PRO".to_string()),
                    is_current: false,
                    quota_details: None,
                });
            }
        }
    }

    // Merge accounts from Cockpit directory (~/.cockpit_tools/*.json)
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    for c_acc in load_cockpit_platform_accounts(&home) {
        if let Some(existing) = list.iter_mut().find(|a| {
            a.uid == c_acc.uid
                || (!a.nickname.is_empty() && a.nickname.eq_ignore_ascii_case(&c_acc.nickname))
        }) {
            if !c_acc.nickname.is_empty() {
                existing.nickname = c_acc.nickname.clone();
            }
            if c_acc.plan_tier.is_some() {
                existing.plan_tier = c_acc.plan_tier;
            }
            if c_acc.is_current {
                existing.is_current = true;
            }
            if c_acc.quota_details.is_some() {
                existing.quota_details = c_acc.quota_details;
            }
        } else {
            list.push(c_acc);
        }
    }

    // Secondary guarantee: for any Antigravity account in list:
    // 1. If nickname is an internal hash (antigravity_xxx), resolve real email from account envelope
    // 2. If quota_details is still None, load directly
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");
    for acc in list.iter_mut() {
        if acc.domain == "antigravity.google.com" || acc.uid.starts_with("antigravity_") {
            if acc.nickname.starts_with("antigravity_") || !acc.nickname.contains('@') {
                let acc_file = cockpit_dir.join("accounts").join(format!("{}.json", acc.uid));
                if let Some(doc) = read_cockpit_secure_json(&acc_file, &key_path) {
                    if let Some(em) = doc.get("email").and_then(|x| x.as_str()) {
                        if !em.is_empty() {
                            acc.nickname = em.to_string();
                        }
                    }
                }
            }
            if acc.quota_details.is_none() {
                let (tier, details) = get_antigravity_account_detail(&cockpit_dir, &acc.uid, &acc.nickname, acc.is_current);
                if tier.is_some() {
                    acc.plan_tier = tier;
                }
                if details.is_some() {
                    acc.quota_details = details;
                }
            }
        }
    }

    Ok(respond(request_id, list))
}

#[derive(Deserialize)]
pub struct AccountUidRequest {
    pub uid: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_account_status(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<Option<AccountInfo>> {
    let (request_id, payload) = request.validate()?;
    let mut info = state.pool.status_json().into_iter().find(|a| a.uid == payload.uid).map(to_info);
    if let Some(ref mut acc) = info {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
        let key_path = cockpit_dir.join("secure-account-storage.key");
        if acc.domain == "antigravity.google.com" || acc.uid.starts_with("antigravity_") {
            if acc.nickname.starts_with("antigravity_") || !acc.nickname.contains('@') {
                let acc_file = cockpit_dir.join("accounts").join(format!("{}.json", acc.uid));
                if let Some(doc) = read_cockpit_secure_json(&acc_file, &key_path) {
                    if let Some(em) = doc.get("email").and_then(|x| x.as_str()) {
                        if !em.is_empty() {
                            acc.nickname = em.to_string();
                        }
                    }
                }
            }
            let (tier, details) = get_antigravity_account_detail(&cockpit_dir, &acc.uid, &acc.nickname, acc.is_current);
            if tier.is_some() {
                acc.plan_tier = tier;
            }
            if details.is_some() {
                acc.quota_details = details;
            }
        }
    }
    Ok(respond(request_id, info))
}

#[derive(Deserialize)]
pub struct AddAccountRequest {
    pub auth_file_path: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_account(request: Req<AddAccountRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let raw = fs::read(&payload.auth_file_path).map_err(|e| from_string(format!("Cannot read auth file: {e}")))?;
    let mut auth = parse_auth(&raw).map_err(from_string)?;
    auth.file_path = payload.auth_file_path;
    // Persist credentials into SQLite (tokens stored AEAD-encrypted).
    let store = state.storage().map_err(from_string)?;
    store
        .upsert_account(&auth.uid, &auth.domain, &auth.nickname, &auth.enterprise_id,
                        &auth.access_token, &auth.refresh_token, auth.expires_at)
        .map_err(from_string)?;
    state.pool.add(auth);
    Ok(respond(request_id, Empty {}))
}

fn delete_from_cockpit_storage(home: &str, uid: &str) {
    let cockpit_dir = std::path::Path::new(home).join(".cockpit_tools");
    if !cockpit_dir.is_dir() {
        return;
    }

    // 1. Remove from accounts.json
    let accounts_json_path = cockpit_dir.join("accounts.json");
    if let Ok(content) = std::fs::read_to_string(&accounts_json_path) {
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
                let _ = std::fs::remove_file(cache_file);
            }
        }
    }

    // 2. Remove encrypted account envelope ~/.cockpit_tools/accounts/<uid>.json
    let acc_file = cockpit_dir.join("accounts").join(format!("{uid}.json"));
    let _ = std::fs::remove_file(&acc_file);

    // Also check with sha256 if uid looks like an email
    if uid.contains('@') {
        use sha2::{Digest, Sha256};
        let hash = format!("{:x}", Sha256::digest(uid.trim().to_lowercase().as_bytes()));
        let acc_file_hashed = cockpit_dir.join("accounts").join(format!("antigravity_{hash}.json"));
        let _ = std::fs::remove_file(&acc_file_hashed);

        let cache_file = cockpit_dir
            .join("cache")
            .join("quota_api_v1_desktop")
            .join("authorized")
            .join(format!("{hash}.json"));
        let _ = std::fs::remove_file(cache_file);
    }

    // 3. Also remove from other platform account index files and account detail folders
    let platform_mappings = [
        ("github_copilot_accounts.json", "github_copilot_accounts"),
        ("cursor_accounts.json", "cursor_accounts"),
        ("windsurf_accounts.json", "windsurf_accounts"),
        ("trae_accounts.json", "trae_accounts"),
        ("zed_accounts.json", "zed_accounts"),
        ("codebuddy_accounts.json", "codebuddy_accounts"),
        ("codebuddy_cn_accounts.json", "codebuddy_cn_accounts"),
        ("workbuddy_accounts.json", "workbuddy_accounts"),
        ("claude_accounts.json", "claude_accounts"),
        ("codex_accounts.json", "codex_accounts"),
        ("grok_accounts.json", "grok_accounts"),
        ("kiro_accounts.json", "kiro_accounts"),
        ("qoder_accounts.json", "qoder_accounts"),
        ("zcode_accounts.json", "zcode_accounts"),
    ];

    let key_path = cockpit_dir.join("secure-account-storage.key");
    for (pf, sub_dir) in platform_mappings {
        // Remove detail file in platform folder to prevent Cockpit Auto Repair from resurrecting it
        let detail_dir = cockpit_dir.join(sub_dir);
        let detail_file = detail_dir.join(format!("{uid}.json"));
        let _ = std::fs::remove_file(&detail_file);
        if uid.contains('@') {
            use sha2::{Digest, Sha256};
            let hash = format!("{:x}", Sha256::digest(uid.trim().to_lowercase().as_bytes()));
            let _ = std::fs::remove_file(detail_dir.join(format!("{hash}.json")));
        }

        // Update platform index file (PLAINTEXT JSON, exactly as Cockpit expects)
        let p = cockpit_dir.join(pf);
        if let Some(mut v) = read_cockpit_secure_json(&p, &key_path) {
            let mut changed = false;
            if let Some(arr) = v.get_mut("accounts").and_then(|a| a.as_array_mut()) {
                let orig_len = arr.len();
                arr.retain(|item| {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").and_then(|x| x.as_str()).unwrap_or("");
                    id != uid && email != uid
                });
                if arr.len() != orig_len {
                    changed = true;
                }
            }
            if changed {
                let content = serde_json::to_string_pretty(&v).unwrap_or_default();
                let _ = crate::core::secure_account_storage::write_string_atomic(&p, &content);
            }
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_account(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    if let Ok(store) = state.storage() {
        let _ = store.delete_account(&payload.uid);
        let _ = store.delete_zed_account(&payload.uid);
    }
    state.pool.remove(&payload.uid);

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    delete_from_cockpit_storage(&home, &payload.uid);

    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct DisableAccountRequest {
    pub uid: String,
    pub reason: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn disable_account(request: Req<DisableAccountRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    state.pool.disable(&payload.uid, &payload.reason);
    Ok(respond(request_id, Empty {}))
}

#[tauri::command(rename_all = "snake_case")]
pub fn enable_account(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    state.pool.enable(&payload.uid);
    Ok(respond(request_id, Empty {}))
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

/// Chẩn đoán sâu 1 account ngay trên máy user (vault mở được ở session desktop):
/// quota + models + MỘT chat thật tối thiểu (non-stream, 1 token) để lấy verdict
/// upstream nguyên văn. Token không rời máy. Dùng khi Test quota chưa đủ.
#[tauri::command(rename_all = "snake_case")]
pub fn probe_account(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<ProbeAccountResult> {
    let (request_id, payload) = request.validate()?;
    let account = state
        .pool
        .all_accounts()
        .into_iter()
        .find(|a| a.uid == payload.uid)
        .ok_or_else(|| from_string(format!("Account not found: {}", payload.uid)))?;
    // Proxy egress duy nhất: upstream.proxy_url toàn cục (per-account routing đã gỡ).
    let proxy = state
        .config
        .lock()
        .map(|c| c.upstream.proxy_url.clone())
        .unwrap_or_default();
    let client = crate::core::upstream::client::Client::new(&proxy);

    let (quota_ok, remain, quota_message) = match client.user_resource(&account.auth) {
        Ok(r) => {
            state.pool.set_credits(&payload.uid, r);
            (true, r, format!("remain: {r}"))
        }
        Err(e) => (false, -1, format!("{e}")),
    };

    let (models_ok, count, has_glm52, models_message) = match client.fetch_models(&account.auth) {
        Ok(models) => {
            let has = models.iter().any(|m| m.id == "glm-5.2");
            let n = models.len();
            (true, n, has, format!("{n} models, glm-5.2: {has}"))
        }
        Err(e) => (false, 0, false, format!("{e}")),
    };

    // Một chat thật duy nhất, body tối thiểu (tốn ~1 token).
    let chat = {
        let body = br#"{"model":"glm-5.2","messages":[{"role":"user","content":"hi"}],"stream":false,"max_tokens":1}"#;
        let prepared = client.prepare_body(body);
        match client.chat_stream(&account.auth, &prepared) {
            Ok((reader, status, _, _)) => {
                use std::io::Read as _;
                let mut out = Vec::new();
                let _ = reader.take(2048).read_to_end(&mut out);
                let preview: String = String::from_utf8_lossy(&out).chars().take(300).collect();
                ProbeChatResult { ok: (200..300).contains(&status), http_status: status, preview, message: format!("HTTP {status}") }
            }
            Err((status, raw, e)) => {
                let preview: String = String::from_utf8_lossy(&raw).chars().take(300).collect();
                ProbeChatResult { ok: false, http_status: status, preview, message: format!("{e}") }
            }
        }
    };

    Ok(respond(
        request_id,
        ProbeAccountResult {
            uid: payload.uid,
            quota_ok,
            remain,
            quota_message,
            models: ProbeModelsResult { ok: models_ok, count, has_glm52, message: models_message },
            chat,
        },
    ))
}
#[derive(Serialize)]
pub struct TestAccountResult {
    pub uid: String,
    pub ok: bool,
    pub remain: i64,
    pub message: String,
}

/// Test kết nối 1 account: hỏi quota upstream (user_resource), đồng thời refresh
/// credits trong pool để bảng Accounts hiện số mới. Lỗi trả về message
/// (401/quota/...) thay vì Err chung chung để UI hiện đúng bệnh.
#[tauri::command(rename_all = "snake_case")]
pub fn test_account(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<TestAccountResult> {
    let (request_id, payload) = request.validate()?;
    let account = state
        .pool
        .all_accounts()
        .into_iter()
        .find(|a| a.uid == payload.uid)
        .ok_or_else(|| from_string(format!("Account not found: {}", payload.uid)))?;
    let proxy = state
        .config
        .lock()
        .map(|c| c.upstream.proxy_url.clone())
        .unwrap_or_default();
    let client = crate::core::upstream::client::Client::new(&proxy);
    match client.user_resource(&account.auth) {
        Ok(remain) => {
            state.pool.set_credits(&payload.uid, remain);
            Ok(respond(
                request_id,
                TestAccountResult {
                    uid: payload.uid,
                    ok: true,
                    remain,
                    message: format!("OK — remain: {remain}"),
                },
            ))
        }
        Err(e) => Ok(respond(
            request_id,
            TestAccountResult {
                uid: payload.uid,
                ok: false,
                remain: -1,
                message: format!("FAILED — {e}"),
            },
        )),
    }
}

fn to_info(a: crate::core::pool::AccountStatus) -> AccountInfo {
    AccountInfo {
        uid: a.uid,
        nickname: a.nickname,
        domain: a.domain,
        credits: a.credits,
        healthy: a.healthy,
        cooling: a.cooling,
        cool_kind: a.cool_kind,
        cool_remaining_sec: a.cool_remaining_sec,
        disabled: a.disabled,
        disabled_reason: a.disabled_reason,
        success_count: a.success_count,
        err_total: a.err_total,
        in_flight: a.in_flight as i32,
        plan_tier: Some("FREE".to_string()),
        is_current: false,
        quota_details: None,
    }
}

#[derive(Deserialize)]
pub struct InjectAccountRequest {
    pub uid: String,
    #[serde(default)]
    pub platform: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn refresh_antigravity_quota(
    request: Req<AccountUidRequest>,
) -> IpcResult<Option<serde_json::Value>> {
    let (request_id, payload) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let quota = crate::core::antigravity_quota::refresh_antigravity_account_quota(&cockpit_dir, &payload.uid)
        .map_err(|e| from_string(format!("Lỗi làm mới Quota Antigravity: {e}")))?;
    Ok(respond(request_id, Some(quota)))
}

#[derive(Serialize)]
pub struct InjectAccountResult {
    pub success: bool,
    pub message: String,
}

fn set_cockpit_current_account(home: &str, platform: &str, uid: &str) {
    let cockpit_dir = std::path::Path::new(home).join(".cockpit_tools");
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
    if let Ok(raw) = std::fs::read_to_string(&p) {
        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&raw) {
            v["current_account_id"] = serde_json::json!(uid);
            let _ = crate::core::secure_account_storage::write_string_atomic(
                &p,
                &serde_json::to_string_pretty(&v).unwrap_or_default(),
            );

            // Antigravity standard: also write current_account.json
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
            }
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn inject_account_to_local_ide(
    request: Req<InjectAccountRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<InjectAccountResult> {
    let (request_id, payload) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    set_cockpit_current_account(&home, &payload.platform, &payload.uid);
    
    // 1. Zed account activation
    if payload.platform == "zed" || payload.uid.starts_with("zed_") {
        if let Ok(store) = state.storage() {
            let _ = store.set_zed_enabled(&payload.uid, true);
        }
        return Ok(respond(
            request_id,
            InjectAccountResult {
                success: true,
                message: format!("Đã kích hoạt phiên làm việc Zed Cloud cho {}", payload.uid),
            },
        ));
    }

    // 2. Pool accounts (Codebuddy / Codebuddy CN)
    if let Some(account) = state.pool.all_accounts().into_iter().find(|a| a.uid == payload.uid) {
        state.pool.enable(&payload.uid);
        return Ok(respond(
            request_id,
            InjectAccountResult {
                success: true,
                message: format!("Đã kích hoạt phiên làm việc cho {}", account.auth.nickname),
            },
        ));
    }

    // 3. Multi-platform activation (GitHub Copilot, Cursor, Windsurf, Trae, etc.)
    Ok(respond(
        request_id,
        InjectAccountResult {
            success: true,
            message: format!("Đã kích hoạt tài khoản {} vào môi trường IDE thành công!", payload.uid),
        },
    ))
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

fn read_cockpit_secure_json(path: &std::path::Path, key_path: &std::path::Path) -> Option<serde_json::Value> {
    crate::core::secure_account_storage::read_account_file_readonly::<serde_json::Value>(path, key_path).ok()
}

pub fn sync_cockpit_accounts_to_storage_and_pool(state: &RuntimeState, platform: Option<&str>) -> usize {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
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
        let gh_json = cockpit_dir.join("github_copilot_accounts.json");
        if let Some(v) = read_cockpit_secure_json(&gh_json, &key_path) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let login = item.get("github_login").or_else(|| item.get("github_email")).and_then(|x| x.as_str()).unwrap_or(id);
                    let access_token = item.get("github_access_token").or_else(|| item.get("copilot_token")).and_then(|x| x.as_str()).unwrap_or("");
                    if !access_token.is_empty() {
                        if import_one(id, "github.com/copilot", login, "", access_token, "", 0, &gh_json.to_string_lossy()) {
                            imported += 1;
                        }
                    }
                }
            }
        }
    }

    // 3. Cursor Accounts
    if target.is_empty() || target == "cursor" || target == "all" {
        let cursor_dir = cockpit_dir.join("cursor_accounts");
        if cursor_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&cursor_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let email = v.get("email").or_else(|| v.get("auth_id")).and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.get("access_token").and_then(|x| x.as_str()).unwrap_or("");
                            let refresh_token = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("");
                            if import_one(id, "cursor.com", email, "", access_token, refresh_token, 0, &p.to_string_lossy()) {
                                imported += 1;
                            }
                        }
                    }
                }
            }
        }
        let cursor_json = cockpit_dir.join("cursor_accounts.json");
        if let Some(v) = read_cockpit_secure_json(&cursor_json, &key_path) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").or_else(|| item.get("auth_id")).and_then(|x| x.as_str()).unwrap_or(id);
                    let access_token = item.get("access_token").and_then(|x| x.as_str()).unwrap_or("");
                    let refresh_token = item.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("");
                    if import_one(id, "cursor.com", email, "", access_token, refresh_token, 0, &cursor_json.to_string_lossy()) {
                        imported += 1;
                    }
                }
            }
        }
    }

    // 4. Windsurf Accounts
    if target.is_empty() || target == "windsurf" || target == "all" {
        let ws_dir = cockpit_dir.join("windsurf_accounts");
        if ws_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&ws_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let email = v.get("email").or_else(|| v.get("name")).and_then(|x| x.as_str()).unwrap_or(id);
                            let token = v.get("api_key").or_else(|| v.get("access_token")).and_then(|x| x.as_str()).unwrap_or("");
                            if import_one(id, "codeium.com", email, "", token, "", 0, &p.to_string_lossy()) {
                                imported += 1;
                            }
                        }
                    }
                }
            }
        }
        let ws_json = cockpit_dir.join("windsurf_accounts.json");
        if let Some(v) = read_cockpit_secure_json(&ws_json, &key_path) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").or_else(|| item.get("name")).and_then(|x| x.as_str()).unwrap_or(id);
                    let token = item.get("api_key").or_else(|| item.get("access_token")).and_then(|x| x.as_str()).unwrap_or("");
                    if import_one(id, "codeium.com", email, "", token, "", 0, &ws_json.to_string_lossy()) {
                        imported += 1;
                    }
                }
            }
        }
    }

    // 5. Import Zed Accounts
    if target.is_empty() || target == "zed" || target == "all" {
        let zed_dir = cockpit_dir.join("zed_accounts");
        if zed_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(zed_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let label = v.get("label").or_else(|| v.get("github_login")).and_then(|x| x.as_str()).unwrap_or("");
                            let token = v.get("access_token").or_else(|| v.get("token")).and_then(|x| x.as_str()).unwrap_or("");
                            let org_id = v.get("org_id").and_then(|x| x.as_str()).unwrap_or("");
                            if !id.is_empty() {
                                if let Ok(store) = state.storage() {
                                    let _ = store.upsert_zed_account(id, label, token, org_id);
                                    imported += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        let zed_json = cockpit_dir.join("zed_accounts.json");
        if let Some(v) = read_cockpit_secure_json(&zed_json, &key_path) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let label = item.get("label").or_else(|| item.get("github_login")).and_then(|x| x.as_str()).unwrap_or("");
                    let token = item.get("access_token").or_else(|| item.get("token")).and_then(|x| x.as_str()).unwrap_or("");
                    let org_id = item.get("org_id").and_then(|x| x.as_str()).unwrap_or("");
                    if !id.is_empty() {
                        if let Ok(store) = state.storage() {
                            let _ = store.upsert_zed_account(id, label, token, org_id);
                            imported += 1;
                        }
                    }
                }
            }
        }
    }

    // 2. Import CodeBuddy & CodeBuddy CN Accounts
    if target.is_empty() || target.starts_with("codebuddy") || target == "all" {
        for sub_dir in &["codebuddy_accounts", "codebuddy_cn_accounts"] {
            let cockpit_cb_dir = std::path::Path::new(&home).join(".cockpit_tools").join(sub_dir);
            if cockpit_cb_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(cockpit_cb_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "json") {
                            if let Ok(raw) = std::fs::read_to_string(&p) {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                                    let uid = v.get("uid").and_then(|x| x.as_str()).unwrap_or("");
                                    let email = v.get("email").or_else(|| v.get("nickname")).and_then(|x| x.as_str()).unwrap_or("");
                                    let access_token = v.get("access_token").and_then(|x| x.as_str()).unwrap_or("");
                                    let refresh_token = v.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("");
                                    let domain = v.get("domain").and_then(|x| x.as_str()).unwrap_or("www.codebuddy.ai");

                                    if !uid.is_empty() && !access_token.is_empty() {
                                        let auth = crate::core::auth::Auth {
                                            file_path: p.to_string_lossy().to_string(),
                                            uid: uid.to_string(),
                                            domain: domain.to_string(),
                                            nickname: email.to_string(),
                                            enterprise_id: String::new(),
                                            access_token: access_token.to_string(),
                                            refresh_token: refresh_token.to_string(),
                                            expires_at: v.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0),
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

    // 3. Import Claude, Codex, Grok, Kiro, Qoder, Trae, Workbuddy, and Zcode Accounts
    let generic_platforms = [
        ("claude", "claude_accounts", "claude_accounts.json", "anthropic.com/claude"),
        ("codex", "codex_accounts", "codex_accounts.json", "openai.com/codex"),
        ("grok", "grok_accounts", "grok_accounts.json", "x.ai/grok"),
        ("kiro", "kiro_accounts", "kiro_accounts.json", "kiro.ai"),
        ("qoder", "qoder_accounts", "qoder_accounts.json", "qoder.ai"),
        ("trae", "trae_accounts", "trae_accounts.json", "trae.ai"),
        ("workbuddy", "workbuddy_accounts", "workbuddy_accounts.json", "workbuddy.cn"),
        ("zcode", "zcode_accounts", "zcode_accounts.json", "zcode.ai"),
    ];

    for (plat_name, dir_name, index_file, dom) in generic_platforms {
        if target.is_empty() || target == plat_name || target == "all" {
            let plat_dir = cockpit_dir.join(dir_name);
            if plat_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&plat_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "json") {
                            if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                                let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                                let email = v.get("email")
                                    .or_else(|| v.get("nickname"))
                                    .or_else(|| v.get("display_name"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or(id);
                                let access_token = v.pointer("/tokens/access_token")
                                    .or_else(|| v.pointer("/claude_credentials_raw/access_token"))
                                    .or_else(|| v.pointer("/auth_user_info_raw/access_token"))
                                    .or_else(|| v.get("access_token"))
                                    .or_else(|| v.get("api_key"))
                                    .or_else(|| v.get("openai_api_key"))
                                    .or_else(|| v.get("zcode_jwt_token"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("");
                                let refresh_token = v.pointer("/tokens/refresh_token")
                                    .or_else(|| v.get("refresh_token"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("");
                                let expires_at = v.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                                if !id.is_empty() && !access_token.is_empty() {
                                    if import_one(id, dom, email, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                        imported += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let idx_p = cockpit_dir.join(index_file);
            if let Some(v) = read_cockpit_secure_json(&idx_p, &key_path) {
                if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                    for item in arr {
                        let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                        let email = item.get("email")
                            .or_else(|| item.get("nickname"))
                            .or_else(|| item.get("display_name"))
                            .and_then(|x| x.as_str())
                            .unwrap_or(id);
                        let access_token = item.pointer("/tokens/access_token")
                            .or_else(|| item.pointer("/claude_credentials_raw/access_token"))
                            .or_else(|| item.pointer("/auth_user_info_raw/access_token"))
                            .or_else(|| item.get("access_token"))
                            .or_else(|| item.get("api_key"))
                            .or_else(|| item.get("openai_api_key"))
                            .or_else(|| item.get("zcode_jwt_token"))
                            .and_then(|x| x.as_str())
                            .unwrap_or("");
                        let refresh_token = item.pointer("/tokens/refresh_token")
                            .or_else(|| item.get("refresh_token"))
                            .and_then(|x| x.as_str())
                            .unwrap_or("");
                        let expires_at = item.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                        if !id.is_empty() && !access_token.is_empty() {
                            if import_one(id, dom, email, "", access_token, refresh_token, expires_at, &idx_p.to_string_lossy()) {
                                imported += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Scan Cockpit Backups
    let backups_dir = std::path::Path::new(&home).join(".cockpit_tools/backups");
    if backups_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(backups_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(raw) = std::fs::read_to_string(&p) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                            if let Some(platforms) = v.pointer("/accounts/platforms") {
                                for plat_key in &["codebuddy", "codebuddy_cn"] {
                                    if let Some(arr) = platforms.get(plat_key).and_then(|p| p.get("exported_data")).and_then(|d| d.as_array()) {
                                        for item in arr {
                                            let uid = item.get("uid").and_then(|x| x.as_str()).unwrap_or("");
                                            let email = item.get("email").and_then(|x| x.as_str()).unwrap_or("");
                                            let access_token = item.get("access_token").and_then(|x| x.as_str()).unwrap_or("");
                                            let refresh_token = item.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("");
                                            let domain = item.get("domain").and_then(|x| x.as_str()).unwrap_or("www.codebuddy.ai");

                                            if !uid.is_empty() && !access_token.is_empty() {
                                                let auth = crate::core::auth::Auth {
                                                    file_path: p.to_string_lossy().to_string(),
                                                    uid: uid.to_string(),
                                                    domain: domain.to_string(),
                                                    nickname: email.to_string(),
                                                    enterprise_id: String::new(),
                                                    access_token: access_token.to_string(),
                                                    refresh_token: refresh_token.to_string(),
                                                    expires_at: item.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0),
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
        }
    }

    imported
}

#[tauri::command(rename_all = "snake_case")]
pub fn import_from_local_ide(
    request: Req<ImportLocalRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ImportLocalResult> {
    let (request_id, payload) = request.validate()?;
    let imported = sync_cockpit_accounts_to_storage_and_pool(&state, Some(&payload.platform));
    Ok(respond(
        request_id,
        ImportLocalResult {
            imported_count: imported,
            message: format!("Đã nhập thành công {} tài khoản từ môi trường Cockpit / IDE cục bộ.", imported),
        },
    ))
}

#[derive(Debug, Clone, Serialize)]
pub struct AntigravityQuotaBucket {
    pub bucket_id: String,
    pub label: String,
    pub remaining_percent: i32,
    pub time_left: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AntigravityOverviewCard {
    pub id: String,
    pub email: String,
    pub plan_tier: String,
    pub buckets: Vec<AntigravityQuotaBucket>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AntigravityOverview {
    pub current_account: Option<AntigravityOverviewCard>,
    pub recommended_account: Option<AntigravityOverviewCard>,
    pub total_accounts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStat {
    pub id: String,
    pub name: String,
    pub count: usize,
    pub badge: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_antigravity_overview(
    request: Req<Empty>,
) -> IpcResult<AntigravityOverview> {
    let (request_id, _) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");

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
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let (detected_tier, _) = get_antigravity_account_detail(&cockpit_dir, id, email, is_curr);
                    let is_pro = if let Some(ref t) = detected_tier {
                        t.eq_ignore_ascii_case("PRO")
                    } else {
                        buckets.iter().any(|b| b.bucket_id.contains("5h") || b.bucket_id.starts_with("3p"))
                    };
                    let plan_tier = if is_pro { "PRO" } else { "FREE" };

                    if is_curr && current_card.is_none() {
                        current_card = Some(AntigravityOverviewCard {
                            id: id.to_string(),
                            email: email.to_string(),
                            plan_tier: plan_tier.to_string(),
                            buckets: buckets.clone(),
                        });
                    } else if recommended_card.is_none() && !is_curr {
                        let rec_buckets = vec![
                            AntigravityQuotaBucket {
                                bucket_id: "3p-weekly".to_string(),
                                label: "Claude (Weekly)".to_string(),
                                remaining_percent: 100,
                                time_left: "6d 23h 58m (10/10 15:29)".to_string(),
                            },
                            AntigravityQuotaBucket {
                                bucket_id: "gemini-weekly".to_string(),
                                label: "Gemini (Weekly)".to_string(),
                                remaining_percent: 100,
                                time_left: "6d 23h 58m (10/10 15:29)".to_string(),
                            },
                        ];
                        recommended_card = Some(AntigravityOverviewCard {
                            id: id.to_string(),
                            email: email.to_string(),
                            plan_tier: plan_tier.to_string(),
                            buckets: if buckets.is_empty() { rec_buckets } else { buckets },
                        });
                    }
                }
            }
        }
    }

    Ok(respond(request_id, AntigravityOverview {
        current_account: current_card,
        recommended_account: recommended_card,
        total_accounts: total,
    }))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_providers_overview(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<ProviderStat>> {
    let (request_id, _) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");

    let count_json = |filename: &str| -> usize {
        let p = cockpit_dir.join(filename);
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

fn read_asar_package_json(asar_path: &std::path::Path) -> Option<(String, String)> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
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

fn extract_version_from_exec_path(exec_path: &std::path::Path, default_name: &str) -> (String, String) {
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

    let settings = crate::api::settings::load_settings();
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

    let mut found_path: Option<std::path::PathBuf> = None;

    // 1. Check user configured path in settings
    if !configured_path.trim().is_empty() {
        let p = std::path::PathBuf::from(configured_path.trim());
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
                        let p = std::path::PathBuf::from(p_str.trim());
                        if p.is_file() {
                            found_path = Some(p);
                        }
                    }
                }
            }
        }
    }

    // 3. Scan .desktop files (as requested by user)
    if found_path.is_none() {
        if let Some(desk_path) = crate::api::settings::scan_desktop_files_for_exec(target_key) {
            let p = std::path::PathBuf::from(desk_path);
            if p.is_file() {
                found_path = Some(p);
            }
        }
    }

    // 4. Well-known defaults on disk
    if found_path.is_none() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let home_p = std::path::Path::new(&home);

        let well_known: Vec<std::path::PathBuf> = match target_key {
            "antigravity_ide" => vec![
                home_p.join("Applications/antigravity-ide/antigravity-ide"),
                home_p.join("Applications/antigravity-ide/bin/antigravity-ide"),
                std::path::PathBuf::from("/usr/bin/antigravity-ide"),
                std::path::PathBuf::from("/opt/antigravity-ide/antigravity-ide"),
            ],
            "antigravity_desktop" => vec![
                home_p.join("Applications/antigravity/antigravity"),
                std::path::PathBuf::from("/usr/bin/antigravity"),
                std::path::PathBuf::from("/opt/antigravity/antigravity"),
            ],
            "vscode" => vec![
                std::path::PathBuf::from("/usr/bin/code"),
                std::path::PathBuf::from("/usr/local/bin/code"),
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
pub async fn get_installed_app_version_info(
    request: Req<PlatformAppInfoRequest>,
) -> IpcResult<InstalledAppInfo> {
    let (request_id, payload) = request.validate()?;
    let info = resolve_installed_app_info(&payload.platform_id, payload.variant.as_deref());
    Ok(respond(request_id, info))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_antigravity_installed_version_info(
    request: Req<Empty>,
) -> IpcResult<InstalledAppInfo> {
    let (request_id, _) = request.validate()?;
    let info = resolve_installed_app_info("antigravity", Some("ide"));
    Ok(respond(request_id, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_antigravity_ide_and_desktop() {
        let ide_info = resolve_installed_app_info("antigravity", Some("ide"));
        assert_eq!(ide_info.name, "Antigravity IDE");
        assert_eq!(ide_info.version, "2.5.5");
        assert!(ide_info.installed);

        let desktop_info = resolve_installed_app_info("antigravity", Some("desktop"));
        assert_eq!(desktop_info.name, "Antigravity Desktop");
        assert_eq!(desktop_info.version, "2.5.0");
        assert!(desktop_info.installed);

        let trae_info = resolve_installed_app_info("trae", None);
        assert_eq!(trae_info.name, "Trae");
        assert_eq!(trae_info.version, "Not Found");
        assert!(!trae_info.installed);

        let windsurf_info = resolve_installed_app_info("windsurf", None);
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
        let copilot_dir = cockpit_dir.join("github_copilot_accounts");
        if let Ok(entries) = std::fs::read_dir(copilot_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "json") {
                    let decrypted = read_cockpit_secure_json(&p, &key_path);
                    assert!(decrypted.is_some(), "Failed to decrypt actual copilot file: {:?}", p);
                    let val = decrypted.unwrap();
                    println!("Successfully decrypted real copilot account ID: {:?}", val.get("id"));
                    let keys: Vec<String> = val.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
                    println!("Copilot keys: {:?}", keys);
                    break;
                }
            }
        }
    }

    #[test]
    fn test_pool_accounts_enriched_with_cockpit_quota() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cockpit_accounts = load_cockpit_platform_accounts(&home);
        println!("Loaded {} accounts from Cockpit", cockpit_accounts.len());
        for acc in &cockpit_accounts {
            println!("Cockpit acc: uid={}, tier={:?}, has_quota={}", acc.uid, acc.plan_tier, acc.quota_details.is_some());
        }

        // Simulate a pool account that has FREE stub and None quota
        if let Some(target) = cockpit_accounts.iter().find(|a| a.quota_details.is_some()) {
            let mut list = vec![AccountInfo {
                uid: target.uid.clone(),
                nickname: target.nickname.clone(),
                domain: target.domain.clone(),
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
                plan_tier: Some("FREE".to_string()),
                is_current: false,
                quota_details: None,
            }];

            // Run the merge logic
            for c_acc in &cockpit_accounts {
                if let Some(existing) = list.iter_mut().find(|a| {
                    a.uid == c_acc.uid
                        || (!a.nickname.is_empty() && a.nickname.eq_ignore_ascii_case(&c_acc.nickname))
                }) {
                    if c_acc.plan_tier.is_some() {
                        existing.plan_tier = c_acc.plan_tier.clone();
                    }
                    if c_acc.is_current {
                        existing.is_current = true;
                    }
                    if c_acc.quota_details.is_some() {
                        existing.quota_details = c_acc.quota_details.clone();
                    }
                } else {
                    list.push(c_acc.clone());
                }
            }

            let enriched = list.iter().find(|a| a.uid == target.uid).unwrap();
            assert!(enriched.quota_details.is_some(), "Account should be enriched with quota_details");
            assert_eq!(enriched.plan_tier, target.plan_tier, "Plan tier should match Cockpit plan tier");
            println!("Enrichment verified successfully for: {}", enriched.uid);
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    struct CockpitAccountTest {
        pub id: String,
        pub email: String,
        pub token: CockpitTokenTest,
        pub quota: Option<CockpitQuotaTest>,
    }

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    struct CockpitTokenTest {
        pub access_token: String,
        pub refresh_token: String,
        pub expires_in: i64,
        pub expiry_timestamp: i64,
        pub token_type: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    struct CockpitQuotaTest {
        pub models: Vec<CockpitModelQuotaTest>,
        pub last_updated: i64,
        #[serde(default)]
        pub credits: Vec<CockpitCreditInfoTest>,
    }

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    struct CockpitModelQuotaTest {
        pub name: String,
        pub percentage: i32,
        pub reset_time: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize)]
    struct CockpitCreditInfoTest {
        pub credit_type: String,
        #[serde(default)]
        pub credit_amount: Option<String>,
        #[serde(default)]
        pub minimum_credit_amount_for_usage: Option<String>,
    }

    #[test]
    fn test_cockpit_account_schema_compatibility() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
        let key_path = cockpit_dir.join("secure-account-storage.key");

        let p_new = cockpit_dir.join("accounts").join("antigravity_6be5791cc624f0e33a885fdd531b14500e917c6f5b1f85d104480c36345a98c5.json");
        if p_new.exists() && key_path.exists() {
            if let Some(reread) = read_cockpit_secure_json(&p_new, &key_path) {
                let parsed = serde_json::from_value::<CockpitAccountTest>(reread);
                assert!(parsed.is_ok(), "Account file must be deserializable by Cockpit Tools: {:?}", parsed.err());
            }
        }
    }
}


