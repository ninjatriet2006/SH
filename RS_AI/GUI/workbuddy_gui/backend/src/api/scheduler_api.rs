use crate::core::runtime::RuntimeState;
use crate::core::upstream::client::Client;
use crate::ipc::{respond, Empty, IpcResult, Req};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub task: String,
    pub success: bool,
    pub message: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_checkin_now(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<TaskResult> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, run_task(&state, "checkin", |client, auth| client.daily_checkin(auth))))
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_travel_now(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<TaskResult> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, run_task(&state, "travel", |client, auth| client.travel_status(auth).map(|_| ()))))
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_activity_now(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<TaskResult> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, run_task(&state, "activity", |client, auth| client.report_chat_activity(auth, &format!("wb2api-{}", chrono::Utc::now().timestamp_millis())))))
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_keepalive_now(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<TaskResult> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, run_task(&state, "keepalive", |client, auth| client.user_resource(auth).map(|_| ()))))
}

fn run_task<F>(state: &RuntimeState, task: &str, operation: F) -> TaskResult
where
    F: Fn(&Client, &crate::core::auth::Auth) -> Result<(), crate::core::upstream::errors::UpstreamError>,
{
    let client = Client::new("");
    let accounts = state.pool.all_accounts();
    if accounts.is_empty() { return TaskResult { task: task.into(), success: false, message: "No accounts available".into() }; }
    let mut failures = 0usize;
    for account in &accounts {
        if operation(&client, &account.auth).is_err() { failures += 1; }
    }
    let success = failures == 0;
    TaskResult { task: task.into(), success, message: if success { format!("{} completed for {} account(s)", task, accounts.len()) } else { format!("{} failed for {failures}/{} account(s)", task, accounts.len()) } }
}
