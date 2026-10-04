//! Zed account operations: list, import, enable, remove, test.

use crate::core::providers::zed;
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct ZedAccountInfo {
    pub id: String,
    pub label: String,
    pub org_id: String,
    pub enabled: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_zed_accounts(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<ZedAccountInfo>> {
    let (request_id, _) = request.validate()?;
    let store = state.storage().map_err(from_string)?;
    let out = store
        .list_zed_accounts()
        .map_err(from_string)?
        .into_iter()
        .map(|a| ZedAccountInfo { id: a.id, label: a.label, org_id: a.org_id, enabled: a.enabled })
        .collect();
    Ok(respond(request_id, out))
}

#[derive(Deserialize)]
pub struct ImportZedRequest {
    /// Absolute path file credential do user chọn qua dialog.
    pub file_path: String,
}

#[derive(Serialize)]
pub struct ImportZedResult {
    pub id: String,
    pub label: String,
    pub org_id: Option<String>,
    pub plan: String,
    pub period: String,
}

/// Import credential file → validate qua users/me → lưu vault.
#[tauri::command(rename_all = "snake_case")]
pub fn import_zed_account(
    request: Req<ImportZedRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ImportZedResult> {
    let (request_id, payload) = request.validate()?;
    if payload.file_path.trim().is_empty() {
        return Err(from_string("credential file path is empty".into()));
    }
    let cred = zed::read_credential_file(payload.file_path.trim()).map_err(from_string)?;
    let profile = zed::fetch_user(&cred.id, &cred.access_token).map_err(from_string)?;
    let label = if profile.name.is_empty() { profile.github_login.clone() } else { profile.name.clone() };
    let store = state.storage().map_err(from_string)?;
    store
        .upsert_zed_account(&profile.user_id, &label, &cred.access_token, profile.org_id.as_deref().unwrap_or(""))
        .map_err(from_string)?;
    state.zed_tokens.invalidate(&zed::account_key(&profile.user_id));
    let period = match (&profile.plan.period_start, &profile.plan.period_end) {
        (Some(s), Some(e)) => format!("{s} → {e}"),
        _ => String::new(),
    };
    Ok(respond(
        request_id,
        ImportZedResult {
            id: profile.user_id,
            label,
            org_id: profile.org_id,
            plan: profile.plan.plan.unwrap_or_default(),
            period,
        },
    ))
}

#[derive(Deserialize)]
pub struct ZedIdRequest {
    pub id: String,
}

#[derive(Deserialize)]
pub struct SetZedEnabledRequest {
    pub id: String,
    pub enabled: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_zed_enabled(
    request: Req<SetZedEnabledRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let store = state.storage().map_err(from_string)?;
    store.set_zed_enabled(&payload.id, payload.enabled).map_err(from_string)?;
    state.zed_tokens.invalidate(&zed::account_key(&payload.id));
    Ok(respond(request_id, Empty {}))
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_zed_account(
    request: Req<ZedIdRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let store = state.storage().map_err(from_string)?;
    store.delete_zed_account(&payload.id).map_err(from_string)?;
    state.zed_tokens.invalidate(&zed::account_key(&payload.id));
    Ok(respond(request_id, Empty {}))
}

#[derive(Serialize)]
pub struct ZedTestResult {
    pub id: String,
    pub ok: bool,
    pub login: String,
    pub plan: String,
    pub quota: String,
    pub message: String,
}

/// Test live credential bằng call GET users/me tới zed.dev API.
#[tauri::command(rename_all = "snake_case")]
pub fn test_zed_account(
    request: Req<ZedIdRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ZedTestResult> {
    let (request_id, payload) = request.validate()?;
    let store = state.storage().map_err(from_string)?;
    let acct = store
        .list_zed_accounts()
        .map_err(from_string)?
        .into_iter()
        .find(|a| a.id == payload.id)
        .ok_or_else(|| from_string(format!("zed account not found: {}", payload.id)))?;
    if !acct.enabled {
        return Ok(respond(
            request_id,
            ZedTestResult {
                id: acct.id,
                ok: false,
                login: String::new(),
                plan: String::new(),
                quota: String::new(),
                message: "account disabled".into(),
            },
        ));
    }
    match zed::fetch_user(&acct.id, &acct.access_token) {
        Ok(p) => {
            let quota = match (p.plan.edit_used, &p.plan.edit_limit) {
                (Some(u), Some(l)) => format!("edits {u}/{l}"),
                _ => "shared pool (no exact remaining)".into(),
            };
            let login = if p.github_login.is_empty() { p.name.clone() } else { format!("{} ({})", p.name, p.github_login) };
            Ok(respond(
                request_id,
                ZedTestResult {
                    id: p.user_id,
                    ok: true,
                    login,
                    plan: p.plan.plan.unwrap_or_default(),
                    quota,
                    message: "OK".into(),
                },
            ))
        }
        Err(e) => Ok(respond(
            request_id,
            ZedTestResult {
                id: acct.id,
                ok: false,
                login: String::new(),
                plan: String::new(),
                quota: String::new(),
                message: e,
            },
        )),
    }
}
