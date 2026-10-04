//! Antigravity Quota calculation and cache synchronization.

use crate::ipc::{from_string, respond, IpcResult, Req};
use super::super::storage::AccountUidRequest;

pub use crate::core::antigravity_quota::*;

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
