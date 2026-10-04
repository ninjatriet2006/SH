//! AI IDE Providers module for Universal API.
//! Organizes all 14 providers into isolated sub-modules:
//! - antigravity
//! - codebuddy
//! - cursor
//! - github_copilot
//! - windsurf
//! - trae
//! - zed
//! - claude
//! - codex
//! - kiro
//! - qoder
//! - zcode
//! - grok
//! - workbuddy

pub mod aggregator;
pub mod antigravity;
pub mod claude;
pub mod codebuddy;
pub mod codex;
pub mod cursor;
pub mod github_copilot;
pub mod grok;
pub mod kiro;
pub mod qoder;
pub mod storage;
pub mod trae;
pub mod windsurf;
pub mod workbuddy;
pub mod zcode;
pub mod zed;

pub use aggregator::*;
pub use storage::*;

use crate::core::auth::parse_auth;
use crate::core::pool::AccountStatus;
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use std::fs;
use std::path::Path;
use tauri::State;

fn to_info(s: AccountStatus) -> AccountInfo {
    AccountInfo {
        uid: s.uid,
        nickname: s.nickname,
        domain: s.domain,
        credits: s.credits,
        healthy: s.healthy,
        cooling: s.cooling,
        cool_kind: s.cool_kind,
        cool_remaining_sec: s.cool_remaining_sec,
        disabled: s.disabled,
        disabled_reason: s.disabled_reason,
        success_count: s.success_count,
        err_total: s.err_total,
        in_flight: s.in_flight as i32,
        plan_tier: Some("FREE".to_string()),
        is_current: false,
        quota_details: None,
    }
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
    let cockpit_dir = Path::new(&home).join(".cockpit_tools");
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
                let (tier, details) = antigravity::accounts::get_antigravity_account_detail(&cockpit_dir, &acc.uid, &acc.nickname, acc.is_current);
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

#[tauri::command(rename_all = "snake_case")]
pub fn get_account_status(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<Option<AccountInfo>> {
    let (request_id, payload) = request.validate()?;
    let mut info = state.pool.status_json().into_iter().find(|a| a.uid == payload.uid).map(to_info);
    if let Some(ref mut acc) = info {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cockpit_dir = Path::new(&home).join(".cockpit_tools");
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
            let (tier, details) = antigravity::accounts::get_antigravity_account_detail(&cockpit_dir, &acc.uid, &acc.nickname, acc.is_current);
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

#[tauri::command(rename_all = "snake_case")]
pub fn add_account(request: Req<AddAccountRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let raw = fs::read(&payload.auth_file_path).map_err(|e| from_string(format!("Cannot read auth file: {e}")))?;
    let mut auth = parse_auth(&raw).map_err(from_string)?;
    auth.file_path = payload.auth_file_path;
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
    state.pool.remove(&payload.uid);
    if let Ok(store) = state.storage() {
        let _ = store.delete_account(&payload.uid);
        let _ = store.delete_zed_account(&payload.uid);
    }
    state.zed_tokens.invalidate(&crate::core::providers::zed::account_key(&payload.uid));
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    delete_from_cockpit_storage(&home, &payload.uid);
    Ok(respond(request_id, Empty {}))
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

#[tauri::command(rename_all = "snake_case")]
pub fn probe_account(request: Req<AccountUidRequest>, state: State<'_, RuntimeState>) -> IpcResult<ProbeAccountResult> {
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

    // 3. Multi-platform activation
    Ok(respond(
        request_id,
        InjectAccountResult {
            success: true,
            message: format!("Đã kích hoạt tài khoản {} vào môi trường IDE thành công!", payload.uid),
        },
    ))
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
