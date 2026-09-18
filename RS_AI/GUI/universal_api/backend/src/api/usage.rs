//! Usage analytics & raw trace inspection backed by SQLite.
use crate::core::runtime::RuntimeState;
use crate::core::storage::{UsageLog, UsageSummary};
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct UsageLogsRequest {
    pub limit: Option<i64>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_usage_logs(request: Req<UsageLogsRequest>, state: State<'_, RuntimeState>) -> IpcResult<Vec<UsageLog>> {
    let (request_id, payload) = request.validate()?;
    let limit = payload.limit.unwrap_or(100).clamp(1, 1000);
    let logs = state.storage().map_err(from_string)?.list_usage_logs(limit).map_err(from_string)?;
    Ok(respond(request_id, logs))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_usage_summary(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<Vec<UsageSummary>> {
    let (request_id, _) = request.validate()?;
    let summary = state.storage().map_err(from_string)?.usage_summary().map_err(from_string)?;
    Ok(respond(request_id, summary))
}
