use crate::core::audit::TrafficAuditLog;
use crate::core::runtime::RuntimeState;
use crate::ipc::{respond, Empty, IpcResult, Req};
use tauri::State;

#[tauri::command(rename_all = "snake_case")]
pub fn get_traffic_logs(
    request: Req<Empty>,
    runtime: State<'_, RuntimeState>,
) -> IpcResult<Vec<TrafficAuditLog>> {
    let (request_id, _) = request.validate()?;
    let mut logs = runtime.audit.get_all();
    logs.reverse(); // Most recent first
    Ok(respond(request_id, logs))
}

#[tauri::command(rename_all = "snake_case")]
pub fn clear_traffic_logs(
    request: Req<Empty>,
    runtime: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, _) = request.validate()?;
    runtime.audit.clear();
    Ok(respond(request_id, Empty {}))
}
