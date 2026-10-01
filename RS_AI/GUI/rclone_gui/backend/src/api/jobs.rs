/*
[INTEGRITY NOTES]
- Mục đích: API hàng đợi job — đường duy nhất cho copy/move/delete/list (Tauri commands).
- Trách nhiệm: Tầng API mỏng — đóng dấu policy từ AppState, gọi `logic::jobs::JobStore`.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::ipc::{
    command_result, deserialize_present_nullable, Empty, IpcErrorCode, IpcResult, Req,
};
use crate::logic::app_state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEnqueueRequest {
    pub kind: String,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub src: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub dst: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub skip_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobIdRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobReorderRequest {
    pub ordered_ids: Vec<String>,
}

/// Job queue: enqueue/list/cancel/reorder.
#[tauri::command]
pub async fn job_enqueue(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    request: Req<JobEnqueueRequest>,
) -> IpcResult<crate::logic::jobs::Job> {
    command_result(request, IpcErrorCode::InvalidArgument, |p| {
        let parsed = crate::logic::jobs::JobKind::parse(&p.kind).ok_or_else(|| {
            "kind must be copy|move|delete|list|manifest|rename|mkdir|touch".to_string()
        })?;
        let policy = state.policy.lock().map(|pol| *pol).unwrap_or_default();
        let job = state.jobs.enqueue_with_policy(
            parsed,
            p.src,
            p.dst,
            policy,
            p.skip_paths.unwrap_or_default(),
        );
        state.jobs.spawn_worker(app_handle);
        Ok(job)
    })
}

#[tauri::command]
pub async fn job_list(
    state: State<'_, AppState>,
    request: Req<Empty>,
) -> IpcResult<Vec<crate::logic::jobs::Job>> {
    command_result(request, IpcErrorCode::Internal, |_| Ok(state.jobs.list()))
}

#[tauri::command]
pub async fn job_cancel(
    state: State<'_, AppState>,
    request: Req<JobIdRequest>,
) -> IpcResult<crate::logic::jobs::Job> {
    command_result(request, IpcErrorCode::NotFound, |p| {
        state.jobs.request_cancel(&p.job_id)
    })
}

#[tauri::command]
pub async fn job_clear_done(
    state: State<'_, AppState>,
    request: Req<Empty>,
) -> IpcResult<usize> {
    command_result(request, IpcErrorCode::Internal, |_| {
        Ok(state.jobs.clear_done())
    })
}


#[tauri::command]
pub fn job_get_queue(state: State<'_, AppState>, request: Req<Empty>) -> IpcResult<Vec<String>> {
    command_result(request, IpcErrorCode::Internal, |_| {
        Ok(state.jobs.get_queue())
    })
}

#[tauri::command]
pub fn job_reorder(state: State<'_, AppState>, request: Req<JobReorderRequest>) -> IpcResult<()> {
    command_result(request, IpcErrorCode::InvalidArgument, |p| {
        state.jobs.reorder_queue(&p.ordered_ids)
    })
}

#[tauri::command]
pub fn job_move_up(state: State<'_, AppState>, request: Req<JobIdRequest>) -> IpcResult<()> {
    command_result(request, IpcErrorCode::NotFound, |p| {
        state.jobs.move_job_up(&p.job_id)
    })
}

#[tauri::command]
pub fn job_move_down(state: State<'_, AppState>, request: Req<JobIdRequest>) -> IpcResult<()> {
    command_result(request, IpcErrorCode::NotFound, |p| {
        state.jobs.move_job_down(&p.job_id)
    })
}

#[tauri::command]
pub fn job_move_to_top(state: State<'_, AppState>, request: Req<JobIdRequest>) -> IpcResult<()> {
    command_result(request, IpcErrorCode::NotFound, |p| {
        state.jobs.move_job_to_top(&p.job_id)
    })
}

#[tauri::command]
pub fn job_get_children(
    state: State<'_, AppState>,
    request: Req<JobIdRequest>,
) -> IpcResult<Vec<crate::logic::queue::QueueItem>> {
    command_result(request, IpcErrorCode::Internal, |p| {
        Ok(state.jobs.children_of(&p.job_id))
    })
}
