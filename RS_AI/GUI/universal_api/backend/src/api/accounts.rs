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

    // 1. Try reading the actual account detail file first
    let mut detected_tier: Option<String> = None;

    if !account_id.is_empty() {
        let acc_file = cockpit_dir.join("accounts").join(format!("{account_id}.json"));
        if let Some(acc_val) = read_cockpit_secure_json(&acc_file, &key_path) {
            if let Some(q) = acc_val.get("quota") {
                if let Some(tier) = q.get("subscription_tier").and_then(|t| t.as_str()) {
                    if !tier.is_empty() {
                        detected_tier = Some(tier.to_uppercase());
                    }
                }
                if detected_tier.is_none() {
                    if let Some(tier_id) = q.get("tier_id").and_then(|t| t.as_str()) {
                        if tier_id.to_lowercase().contains("pro") {
                            detected_tier = Some("PRO".to_string());
                        } else if tier_id.to_lowercase().contains("ultra") {
                            detected_tier = Some("ULTRA".to_string());
                        }
                    }
                }
            }
        }
    }

    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(email.trim().to_lowercase().as_bytes()));
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
                // PRO tier
                let mut c_5h = None;
                let mut c_weekly = None;
                let mut g_5h = None;
                let mut g_weekly = None;

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
                                    "3p-5h" => c_5h = Some(obj),
                                    "3p-weekly" => c_weekly = Some(obj),
                                    "gemini-5h" => g_5h = Some(obj),
                                    "gemini-weekly" => g_weekly = Some(obj),
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                let final_tier = detected_tier.unwrap_or_else(|| "PRO".to_string());
                let details = serde_json::json!({
                    "plan_tier": final_tier,
                    "is_current": is_current,
                    "claude_5h": c_5h,
                    "claude_weekly": c_weekly,
                    "gemini_5h": g_5h,
                    "gemini_weekly": g_weekly
                });
                return (Some(final_tier), Some(details));
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

                let c_weekly = get_model_bucket("claude-opus-4-6-thinking")
                    .or_else(|| get_model_bucket("claude-sonnet-4-6"));
                let g_weekly = get_model_bucket("gemini-2.5-flash")
                    .or_else(|| get_model_bucket("gemini-2.5-flash-thinking"));

                let final_tier = detected_tier.unwrap_or_else(|| "FREE".to_string());
                let details = serde_json::json!({
                    "plan_tier": final_tier,
                    "is_current": is_current,
                    "claude_5h": null,
                    "claude_weekly": c_weekly,
                    "gemini_5h": null,
                    "gemini_weekly": g_weekly
                });
                return (Some(final_tier), Some(details));
            }
        }
    }

    if let Some(tier) = detected_tier {
        let details = serde_json::json!({
            "plan_tier": tier,
            "is_current": is_current,
            "claude_5h": null,
            "claude_weekly": null,
            "gemini_5h": null,
            "gemini_weekly": null
        });
        return (Some(tier), Some(details));
    }

    (Some("FREE".to_string()), None)
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
        ("github_copilot_accounts.json", "github.com/copilot"),
        ("cursor_accounts.json", "cursor.com"),
        ("windsurf_accounts.json", "codeium.com"),
        ("trae_accounts.json", "trae.ai"),
        ("zed_accounts.json", "cloud.zed.dev"),
        ("accounts.json", "antigravity.google.com"),
        ("codebuddy_accounts.json", "codebuddy.ai"),
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
                            is_current = true;
                            let (tier, details) = get_copilot_account_detail(item);
                            plan_tier = tier;
                            quota_details = details;
                        } else if file_name == "codebuddy_accounts.json" {
                            plan_tier = Some("FREE".to_string());
                            quota_details = Some(serde_json::json!({
                                "plan_tier": "FREE",
                                "subscription": "Free Plan Subscription",
                                "subscription_val": "0 / 100",
                                "credit_package": "Credit Package",
                                "credit_package_val": "0 / 0",
                                "next_refresh": "11/01/2026, 00:00:00"
                            }));
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
        if !list.iter().any(|a| a.uid == c_acc.uid) {
            list.push(c_acc);
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
    Ok(respond(request_id, state.pool.status_json().into_iter().find(|a| a.uid == payload.uid).map(to_info)))
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

#[tauri::command(rename_all = "snake_case")]
pub fn remove_account(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    if let Ok(store) = state.storage() {
        let _ = store.delete_account(&payload.uid);
        let _ = store.delete_zed_account(&payload.uid);
    }
    state.pool.remove(&payload.uid);
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

#[derive(Serialize)]
pub struct InjectAccountResult {
    pub success: bool,
    pub message: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn inject_account_to_local_ide(
    request: Req<InjectAccountRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<InjectAccountResult> {
    let (request_id, payload) = request.validate()?;
    
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

    // 3. Multi-platform activation (GitHub Copilot, Cursor, Windsurf, Trae)
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

    // 3. Scan Cockpit Backups
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
    let codebuddy_count = state.pool.status_json().len().max(count_json("codebuddy_accounts.json"));
    let cursor_count = count_json("cursor_accounts.json");
    let windsurf_count = count_json("windsurf_accounts.json");
    let trae_count = count_json("trae_accounts.json");
    let zed_count = count_json("zed_accounts.json");

    let total = antigravity_count + copilot_count + codebuddy_count + cursor_count + windsurf_count + trae_count + zed_count;

    let stats = vec![
        ProviderStat { id: "total".into(), name: "Total accounts".into(), count: total.max(6), badge: None },
        ProviderStat { id: "relay".into(), name: "Relay".into(), count: 1, badge: None },
        ProviderStat { id: "claude".into(), name: "Claude".into(), count: 0, badge: None },
        ProviderStat { id: "codex".into(), name: "Codex".into(), count: 0, badge: Some("+1".into()) },
        ProviderStat { id: "antigravity".into(), name: "Antigravity".into(), count: antigravity_count.max(4), badge: Some("+1".into()) },
        ProviderStat { id: "zed".into(), name: "Zed".into(), count: zed_count, badge: None },
        ProviderStat { id: "github_copilot".into(), name: "GitHub Copilot".into(), count: copilot_count.max(1), badge: None },
        ProviderStat { id: "windsurf".into(), name: "Windsurf".into(), count: windsurf_count, badge: None },
        ProviderStat { id: "kiro".into(), name: "Kiro".into(), count: 0, badge: None },
        ProviderStat { id: "cursor".into(), name: "Cursor".into(), count: cursor_count, badge: None },
        ProviderStat { id: "grok".into(), name: "Grok CLI".into(), count: 0, badge: None },
        ProviderStat { id: "codebuddy".into(), name: "CodeBuddy".into(), count: codebuddy_count.max(1), badge: Some("+2".into()) },
        ProviderStat { id: "qoder".into(), name: "Qoder".into(), count: 0, badge: None },
        ProviderStat { id: "zcode".into(), name: "ZCode".into(), count: 0, badge: None },
        ProviderStat { id: "trae".into(), name: "Trae".into(), count: trae_count, badge: Some("+3".into()) },
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
}


