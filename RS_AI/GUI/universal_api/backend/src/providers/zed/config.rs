//! Zed configuration and model catalog actions.

use crate::core::providers::zed;
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ZedConfigView {
    pub enabled: bool,
    pub system_id: String,
}

/// Đọc Configuration khối Zed.
#[tauri::command(rename_all = "snake_case")]
pub fn get_zed_config(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ZedConfigView> {
    let (request_id, _) = request.validate()?;
    let guard = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?;
    Ok(respond(
        request_id,
        ZedConfigView { enabled: guard.zed.enabled, system_id: guard.zed.system_id.clone() },
    ))
}

#[derive(Deserialize)]
pub struct SaveZedConfigRequest {
    pub enabled: bool,
    pub system_id: String,
}

/// Lưu Configuration khối Zed. Có hiệu lực ngay cho request sau (đọc live từ config).
#[tauri::command(rename_all = "snake_case")]
pub fn save_zed_config(
    request: Req<SaveZedConfigRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    if payload.system_id.len() > 256 {
        return Err(from_string("system_id too long (max 256)".into()));
    }
    if !payload.system_id.trim().is_empty()
        && !payload
            .system_id
            .trim()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
    {
        return Err(from_string("system_id: only A-Za-z0-9._:- allowed".into()));
    }
    {
        let mut guard = state
            .config
            .lock()
            .map_err(|_| from_string("Config lock poisoned".into()))?;
        guard.zed.enabled = payload.enabled;
        guard.zed.system_id = payload.system_id.trim().to_string();
        crate::core::config::save_config(&guard).map_err(from_string)?;
    }
    Ok(respond(request_id, Empty {}))
}

/// Refresh models từ mọi account Zed enabled → cache routing + merge /v1/models.
#[tauri::command(rename_all = "snake_case")]
pub fn refresh_zed_models(
    request: Req<Empty>,
    gateway: State<'_, crate::core::gateway_state::GatewayState>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<zed::ZedModelInfo>> {
    let (request_id, _) = request.validate()?;
    let handle = gateway
        .zed_models
        .lock()
        .map_err(|_| from_string("State lock poisoned".into()))?
        .clone()
        .ok_or_else(|| from_string("gateway not running (start it first)".into()))?;
    let store = state.storage().map_err(from_string)?;
    let accounts = store.list_zed_accounts().map_err(from_string)?;
    let sys = state
        .config
        .lock()
        .map(|c| zed::resolve_system_id(&c.zed.system_id))
        .unwrap_or_else(|_| zed::system_id_env());
    let mut merged: std::collections::HashMap<String, zed::ZedModelInfo> = std::collections::HashMap::new();
    let mut errors = Vec::new();
    for a in accounts.iter().filter(|a| a.enabled) {
        let key = zed::account_key(&a.id);
        match zed::list_models(&state.zed_tokens, &key, &a.id, &a.access_token, empty_org(&a.org_id), sys.as_deref()) {
            Ok(models) => {
                for m in models {
                    merged.entry(m.id.clone()).or_insert(m);
                }
            }
            Err(e) => errors.push(format!("{}: {e}", a.id)),
        }
    }
    if merged.is_empty() {
        let detail = if errors.is_empty() { "no enabled zed account".to_string() } else { errors.join("; ") };
        return Err(from_string(detail));
    }
    *handle.write() = merged.clone();
    let mut out: Vec<zed::ZedModelInfo> = merged.into_values().collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(respond(request_id, out))
}

fn empty_org(s: &str) -> Option<&str> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}
