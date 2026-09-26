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
    skip_paths: Option<Vec<String>>,
) -> Result<crate::logic::jobs::Job, String> {
    let parsed =
        crate::logic::jobs::JobKind::parse(&kind)
            .ok_or_else(|| "kind must be copy|move|delete|list|manifest|rename|mkdir|touch".to_string())?;
    // Đóng dấu policy hiện tại (AppState) lên job lúc đặt việc — worker chỉ đọc
    // `job.policy`, không đọc lại state, nên đổi policy giữa chừng không phá việc cũ.
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    // UNIVERSAL worker-check: danh sách rel bỏ qua (modal thu từ user, rỗng =
    // mặc định Replace); worker check tươi từng vé rồi áp.
    let job = state.jobs.enqueue_with_policy(parsed, src, dst, policy, skip_paths.unwrap_or_default());
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

/// Lấy danh sách ID các job đang chờ trong hàng đợi theo thứ tự thực thi.
#[tauri::command]
pub fn job_get_queue(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    Ok(state.jobs.get_queue())
}

/// Đổi thứ tự toàn bộ hàng đợi theo mảng ID được cấp.
#[tauri::command]
pub fn job_reorder(
    state: State<'_, AppState>,
    ordered_ids: Vec<String>,
) -> Result<(), String> {
    state.jobs.reorder_queue(&ordered_ids)
}

/// Đẩy job lên trước 1 vị trí trong hàng đợi.
#[tauri::command]
pub fn job_move_up(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    state.jobs.move_job_up(&job_id)
}

/// Đẩy job xuống sau 1 vị trí trong hàng đợi.
#[tauri::command]
pub fn job_move_down(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    state.jobs.move_job_down(&job_id)
}

/// Đưa job lên đầu hàng đợi chờ.
#[tauri::command]
pub fn job_move_to_top(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    state.jobs.move_job_to_top(&job_id)
}

