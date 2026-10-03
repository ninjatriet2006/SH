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
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_accounts(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<Vec<AccountInfo>> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, state.pool.status_json().into_iter().map(to_info).collect()))
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
    if let Ok(store) = state.storage() { let _ = store.delete_account(&payload.uid); }
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
    let account = state
        .pool
        .all_accounts()
        .into_iter()
        .find(|a| a.uid == payload.uid)
        .ok_or_else(|| from_string(format!("Account not found: {}", payload.uid)))?;

    state.pool.enable(&payload.uid);

    Ok(respond(
        request_id,
        InjectAccountResult {
            success: true,
            message: format!("Đã kích hoạt phiên làm việc cho {}", account.auth.nickname),
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
    let (request_id, _payload) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut imported = 0;

    // 1. Quét từ Cockpit directory (~/.cockpit_tools/codebuddy_accounts/)
    let cockpit_cb_dir = std::path::Path::new(&home).join(".cockpit_tools/codebuddy_accounts");
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

    // 2. Quét từ Cockpit Backups nếu folder riêng chưa có
    if imported == 0 {
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
    }

    Ok(respond(
        request_id,
        ImportLocalResult {
            imported_count: imported,
            message: format!("Đã nhập thành công {} tài khoản từ môi trường Cockpit / IDE cục bộ.", imported),
        },
    ))
}

