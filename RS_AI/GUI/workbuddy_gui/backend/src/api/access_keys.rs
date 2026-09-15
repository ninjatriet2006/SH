//! Local access-key management (keys handed out to clients like IDEs/Cursor).
//! Keys are generated server-side; plaintext returned exactly once.
use crate::core::runtime::RuntimeState;
use crate::core::storage::AccessKey;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct CreatedAccessKey {
    pub key: AccessKey,
    /// Full plaintext key — shown once, never persisted.
    pub plaintext: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_access_keys(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<Vec<AccessKey>> {
    let (request_id, _) = request.validate()?;
    let keys = state.storage().map_err(from_string)?.list_access_keys().map_err(from_string)?;
    Ok(respond(request_id, keys))
}

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub label: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_access_key(request: Req<CreateKeyRequest>, state: State<'_, RuntimeState>) -> IpcResult<CreatedAccessKey> {
    let (request_id, payload) = request.validate()?;
    if payload.label.trim().is_empty() {
        return Err(from_string("invalid: label must not be empty".into()));
    }
    let (key, plaintext) = state.storage().map_err(from_string)?.create_access_key(payload.label.trim()).map_err(from_string)?;
    Ok(respond(request_id, CreatedAccessKey { key, plaintext }))
}

#[derive(Deserialize)]
pub struct RevokeKeyRequest {
    pub id: i64,
}

#[tauri::command(rename_all = "snake_case")]
pub fn revoke_access_key(request: Req<RevokeKeyRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    state.storage().map_err(from_string)?.revoke_access_key(payload.id).map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}
