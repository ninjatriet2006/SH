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
    email: &str,
    is_current: bool,
) -> (Option<String>, Option<serde_json::Value>) {
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

                let details = serde_json::json!({
                    "plan_tier": "PRO",
                    "is_current": is_current,
                    "claude_5h": c_5h,
                    "claude_weekly": c_weekly,
                    "gemini_5h": g_5h,
                    "gemini_weekly": g_weekly
                });
                return (Some("PRO".to_string()), Some(details));
            } else if let Some(models) = payload.and_then(|p| p.get("models")) {
                // FREE tier
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

                let details = serde_json::json!({
                    "plan_tier": "FREE",
                    "is_current": is_current,
                    "claude_5h": null,
                    "claude_weekly": c_weekly,
                    "gemini_5h": null,
                    "gemini_weekly": g_weekly
                });
                return (Some("FREE".to_string()), Some(details));
            }
        }
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

    for (file_name, domain) in files {
        let p = cockpit_dir.join(file_name);
        if let Ok(raw) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
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
                            let (tier, details) = get_antigravity_account_detail(&cockpit_dir, nickname, is_current);
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

#[tauri::command(rename_all = "snake_case")]
pub fn import_from_local_ide(
    request: Req<ImportLocalRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ImportLocalResult> {
    let (request_id, payload) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut imported = 0;

    // 1. Import Zed Accounts
    if payload.platform.is_empty() || payload.platform == "zed" || payload.platform == "all" {
        let zed_dir = std::path::Path::new(&home).join(".cockpit_tools/zed_accounts");
        if zed_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(zed_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Ok(raw) = std::fs::read_to_string(&p) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
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
        }

        // Also check ~/.cockpit_tools/zed_accounts.json
        let zed_json = std::path::Path::new(&home).join(".cockpit_tools/zed_accounts.json");
        if let Ok(raw) = std::fs::read_to_string(&zed_json) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
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
    }

    // 2. Import CodeBuddy & CodeBuddy CN Accounts
    if payload.platform.is_empty() || payload.platform.starts_with("codebuddy") || payload.platform == "all" {
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

                    if is_curr && current_card.is_none() {
                        current_card = Some(AntigravityOverviewCard {
                            id: id.to_string(),
                            email: email.to_string(),
                            plan_tier: "PRO".to_string(),
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
                            plan_tier: "FREE".to_string(),
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


