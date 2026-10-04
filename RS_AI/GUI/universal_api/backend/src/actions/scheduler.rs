use crate::core::config::ScheduleConfig;
use crate::core::runtime::RuntimeState;
use crate::core::upstream::client::Client;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub task: String,
    pub success: bool,
    pub message: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_keepalive_now(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<TaskResult> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, run_task(&state, "keepalive", |client, auth| client.user_resource(auth).map(|_| ()))))
}

/// Đọc lịch hiện tại từ config (giờ + cờ bật/tắt từng task).
#[tauri::command(rename_all = "snake_case")]
pub fn get_schedule(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<ScheduleConfig> {
    let (request_id, _) = request.validate()?;
    let schedule = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .schedule
        .clone();
    Ok(respond(request_id, schedule))
}

#[derive(Deserialize)]
pub struct SaveScheduleRequest {
    pub schedule: ScheduleConfig,
}

/// Lưu lịch (validate giờ 0..23). Có hiệu lực sau khi restart gateway vì
/// Scheduler nền snapshot config lúc start.
#[tauri::command(rename_all = "snake_case")]
pub fn save_schedule(request: Req<SaveScheduleRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    validate_schedule(&payload.schedule).map_err(from_string)?;
    {
        let mut guard = state
            .config
            .lock()
            .map_err(|_| from_string("Config lock poisoned".into()))?;
        guard.schedule = payload.schedule.clone();
        crate::core::config::save_config(&guard).map_err(from_string)?;
    }
    Ok(respond(request_id, Empty {}))
}

fn validate_schedule(s: &ScheduleConfig) -> Result<(), String> {
    // Chỉ còn keepalive (bản intl không có checkin/travel/activity).
    // Field cũ trong file vẫn parse (serde bỏ qua) — không cần migrate.
    let hours = &s.keepalive_hours;
    if hours.len() > 24 {
        return Err("keepalive_hours has too many entries (max 24)".into());
    }
    for h in hours {
        if *h < 0 || *h > 23 {
            return Err(format!("keepalive_hours contains invalid hour {h} (must be 0..23)"));
        }
    }
    Ok(())
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
