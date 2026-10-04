//! Daily auto check-in actions for platform accounts.

use serde::{Deserialize, Serialize};
use crate::core::auto_checkin::{
    execute_checkin_cycle, load_auto_checkin_config, load_auto_checkin_logs,
    save_auto_checkin_config, AutoCheckinConfig, AutoCheckinLogRecord,
};
use crate::ipc::{respond, Empty, IpcError, IpcResult, Req};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCheckinStatusResponse {
    pub config: AutoCheckinConfig,
    pub logs: Vec<AutoCheckinLogRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToggleCheckinPayload {
    pub enabled: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_auto_checkin_status(request: Req<Empty>) -> IpcResult<AutoCheckinStatusResponse> {
    let (request_id, _) = request.validate()?;
    let config = load_auto_checkin_config();
    let logs = load_auto_checkin_logs();
    Ok(respond(request_id, AutoCheckinStatusResponse { config, logs }))
}

#[tauri::command(rename_all = "snake_case")]
pub fn toggle_auto_checkin(request: Req<ToggleCheckinPayload>) -> IpcResult<bool> {
    let (request_id, payload) = request.validate()?;
    let mut config = load_auto_checkin_config();
    config.enabled = payload.enabled;
    match save_auto_checkin_config(&config) {
        Ok(()) => Ok(respond(request_id, true)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_auto_checkin_now(request: Req<Empty>) -> IpcResult<Vec<AutoCheckinLogRecord>> {
    let (request_id, _) = request.validate()?;
    match execute_checkin_cycle() {
        Ok(logs) => Ok(respond(request_id, logs)),
        Err(e) => Err(IpcError::internal(e)),
    }
}
