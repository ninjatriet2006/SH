/*
[INTEGRITY NOTES]
- Mục đích: API hàng đợi job — đường duy nhất cho copy/move/delete/list (Tauri commands).
- Trách nhiệm: Tầng API mỏng — validate Req, đóng dấu policy từ AppState, gọi `logic::jobs::JobStore`.
- Tương tác: Giữ nguyên tên lệnh Tauri + payload JSON + IpcError (dời từ `ipc.rs`).
*/

use super::envelope::{Empty, IpcErrorCode, IpcResult, Req, backend_error, error, success, validate};
use crate::logic::app_state::AppState;
use tauri::State;

crate::payload!(JobEnqueuePayload { kind: String, src: Option<String>, dst: Option<String> });
crate::payload!(JobIdPayload { job_id: String });

/// Job queue (đường duy nhất cho copy/move/delete/list): enqueue/list/cancel.
#[tauri::command]
pub async fn job_enqueue(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    request: Req<JobEnqueuePayload>,
) -> IpcResult<crate::logic::jobs::Job> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let kind = crate::logic::jobs::JobKind::parse(&payload.kind)
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "kind must be copy|move|delete|list"))?;
    // Đóng dấu policy hiện tại (AppState) lên job lúc đặt việc — worker chỉ đọc
    // `job.policy`, không đọc lại state, nên đổi policy giữa chừng không phá việc cũ.
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    let job = state.jobs.enqueue_with_policy(kind, payload.src, payload.dst, policy);
    state.jobs.spawn_worker(app_handle);
    Ok(success(request_id, job))
}

/// P1: liệt kê snapshot toàn bộ job.
#[tauri::command]
pub async fn job_list(
    state: State<'_, AppState>,
    request: Req<Empty>,
) -> IpcResult<Vec<crate::logic::jobs::Job>> {
    let request_id = validate(&request)?;
    Ok(success(request_id, state.jobs.list()))
}

/// P1: hủy job (queued → cancelled ngay; running → cờ dừng bước tiếp).
#[tauri::command]
pub async fn job_cancel(
    state: State<'_, AppState>,
    request: Req<JobIdPayload>,
) -> IpcResult<crate::logic::jobs::Job> {
    let request_id = validate(&request)?;
    state
        .jobs
        .request_cancel(&request.payload.job_id)
        .map(|job| success(request_id, job))
        .map_err(backend_error)
}
