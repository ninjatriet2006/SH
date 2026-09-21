/*
[INTEGRITY NOTES]
- Mục đích: API hàng đợi job — đường duy nhất cho copy/move/delete/list (Tauri commands).
- Trách nhiệm: Tầng API mỏng — đóng dấu policy từ AppState, gọi `logic::jobs::JobStore`.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::logic::app_state::AppState;
use tauri::State;

/// Job queue (đường duy nhất cho copy/move/delete/list): enqueue/list/cancel.
#[tauri::command]
pub async fn job_enqueue(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    kind: String,
    src: Option<String>,
    dst: Option<String>,
) -> Result<crate::logic::jobs::Job, String> {
    let parsed =
        crate::logic::jobs::JobKind::parse(&kind)
            .ok_or_else(|| "kind must be copy|move|delete|list".to_string())?;
    // Đóng dấu policy hiện tại (AppState) lên job lúc đặt việc — worker chỉ đọc
    // `job.policy`, không đọc lại state, nên đổi policy giữa chừng không phá việc cũ.
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    let job = state.jobs.enqueue_with_policy(parsed, src, dst, policy);
    state.jobs.spawn_worker(app_handle);
    Ok(job)
}

/// P1: liệt kê snapshot toàn bộ job.
#[tauri::command]
pub async fn job_list(
    state: State<'_, AppState>,
) -> Result<Vec<crate::logic::jobs::Job>, String> {
    Ok(state.jobs.list())
}

/// P1: hủy job (queued → cancelled ngay; running → cờ dừng bước tiếp).
#[tauri::command]
pub async fn job_cancel(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<crate::logic::jobs::Job, String> {
    state.jobs.request_cancel(&job_id)
}
