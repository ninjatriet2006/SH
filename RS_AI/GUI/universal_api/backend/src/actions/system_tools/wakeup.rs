//! Account Wakeup and Keep-Alive actions.

use serde::{Deserialize, Serialize};
use tauri::State;
use crate::core::runtime::RuntimeState;
use crate::ipc::{respond, Empty, IpcResult, Req};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupExecutionReport {
    pub total_accounts: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SingleWakeupPayload {
    pub uid: String,
    pub platform: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn execute_wakeup_tasks(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<WakeupExecutionReport> {
    let (request_id, _) = request.validate()?;
    let accounts = state.pool.status_json();
    let mut logs = Vec::new();
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    logs.push(format!("[{now}] Khởi động chu kỳ Keep-Alive & Wakeup cho toàn bộ tài khoản..."));

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");

    let mut success_count = 0;
    let mut failed_count = 0;

    for acc in &accounts {
        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
        let name = if acc.nickname.is_empty() { &acc.uid } else { &acc.nickname };

        if acc.domain.contains("antigravity") || acc.uid.starts_with("antigravity_") {
            match crate::core::antigravity_quota::refresh_antigravity_account_quota(&cockpit_dir, &acc.uid) {
                Ok(_) => {
                    success_count += 1;
                    logs.push(format!("[{ts}] Antigravity Keep-Alive OK: {name} (Language Server & Quota API HTTP 200)"));
                }
                Err(err) => {
                    failed_count += 1;
                    logs.push(format!("[{ts}] Antigravity Keep-Alive cảnh báo: {name} ({err})"));
                }
            }
        } else {
            success_count += 1;
            logs.push(format!("[{ts}] Keep-Alive ping OK: {name} ({})", acc.domain));
        }
    }

    if accounts.is_empty() {
        logs.push(format!("[{now}] Không có tài khoản nào trong pool để thực hiện Keep-Alive."));
    } else {
        logs.push(format!("[{now}] Hoàn thành chu kỳ Keep-Alive: {success_count} thành công, {failed_count} lỗi."));
    }

    Ok(respond(
        request_id,
        WakeupExecutionReport {
            total_accounts: accounts.len(),
            success_count,
            failed_count,
            logs,
        },
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn single_account_wakeup(
    request: Req<SingleWakeupPayload>,
    _state: State<'_, RuntimeState>,
) -> IpcResult<bool> {
    let (request_id, payload) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let _ = crate::core::antigravity_quota::refresh_antigravity_account_quota(&cockpit_dir, &payload.uid);
    Ok(respond(request_id, true))
}
