use crate::core::auth::parse_auth;
use crate::core::pool::FingerprintConfig;
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
    pub proxy_url: Option<String>,
    pub fingerprint_profile: Option<FingerprintConfig>,
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

#[derive(Deserialize)]
pub struct UpdateAccountRoutingRequest {
    pub uid: String,
    pub proxy_url: Option<String>,
    pub user_agent: Option<String>,
    pub custom_headers: Option<std::collections::HashMap<String, String>>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_account_routing(
    request: Req<UpdateAccountRoutingRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let fp = if payload.user_agent.is_some() || payload.custom_headers.is_some() {
        Some(FingerprintConfig {
            user_agent: payload.user_agent.filter(|s| !s.trim().is_empty()),
            headers: payload.custom_headers.unwrap_or_default(),
        })
    } else {
        None
    };
    let proxy = payload.proxy_url.and_then(|p| {
        let trimmed = p.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    });
    if !state.pool.update_routing(&payload.uid, proxy, fp) {
        return Err(from_string(format!("Account not found: {}", payload.uid)));
    }
    Ok(respond(request_id, Empty {}))
}

fn to_info(a: crate::core::pool::AccountStatus) -> AccountInfo {
    AccountInfo {
        uid: a.uid,
        nickname: a.nickname,
        domain: a.domain,
        proxy_url: a.proxy_url,
        fingerprint_profile: a.fingerprint_profile,
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
