/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉnh-sửa File (Command Tauri).
- Trách nhiệm: Nhận request từ Frontend, gọi tầng `logic`/`actions` để phân tích và thực thi.
- Tương tác: Giao tiếp trực tiếp với Frontend. Không đổi tên lệnh IPC, struct DTO hay JSON.
*/
// UNIVERSAL: S2 chẻ xem/sửa — file này giữ nhánh sửa (mkdir/delete/touch/rename/chmod/chown).
// Mỗi lệnh đúng 1 pub command fn: validate Req → gọi actions trực tiếp → map backend_error.

use super::envelope::{Empty, IpcErrorCode, IpcResult, Req, backend_error, error, success, validate};
use crate::actions::perm::Policy;
use crate::logic::app_state::AppState;
use tauri::State;

crate::payload!(EditPathPayload { path: String });
crate::payload!(RenamePayload { old_path: String, new_path: String });
crate::payload!(ChmodPayload { path: String, mode: u32 });
crate::payload!(ChownPayload { path: String, uid: u32, gid: u32 });
crate::payload!(PermissionPolicyPayload { policy: String });

#[tauri::command]
pub async fn fs_mkdir(request: Req<EditPathPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::actions::create::execute_mkdir(request.payload.path, Policy::AllowSystem)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_touch(request: Req<EditPathPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::actions::create::execute_touch(request.payload.path, Policy::AllowSystem)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_delete(request: Req<EditPathPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::actions::delete_op::execute_delete(
        request.payload.path,
        crate::actions::delete_op::DeleteScope::NoTrash,
    )
    .await
    .map(|data| success(request_id, data))
    .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_rename(request: Req<RenamePayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::actions::rename::execute_rename(payload.old_path, payload.new_path, Policy::AllowSystem)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

/// S2: chmod tôn trọng policy — envelope/payload cũ giữ nguyên, policy đọc từ State.
#[tauri::command]
pub async fn fs_chmod(state: State<'_, AppState>, request: Req<ChmodPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    crate::actions::perm::execute_chmod(payload.path, payload.mode, policy)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

/// S2: chown tôn trọng policy — envelope/payload cũ giữ nguyên, policy đọc từ State.
#[tauri::command]
pub async fn fs_chown(state: State<'_, AppState>, request: Req<ChownPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    crate::actions::perm::execute_chown(payload.path, payload.uid, payload.gid, policy)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

/// S2: đọc policy hiện tại (`deny`/`ask_once`/`allow_system`).
#[tauri::command]
pub fn get_permission_policy(state: State<'_, AppState>, request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    Ok(success(request_id, policy.as_str().to_string()))
}

/// S2: đặt policy — sai chuỗi trả `InvalidArgument`, IPC cũ khác giữ nguyên.
#[tauri::command]
pub fn set_permission_policy(
    state: State<'_, AppState>,
    request: Req<PermissionPolicyPayload>,
) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let policy = Policy::parse(request.payload.policy.trim())
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "policy must be deny|ask_once|allow_system"))?;
    *state
        .policy
        .lock()
        .map_err(|_| error(IpcErrorCode::Internal, "policy lock poisoned"))? = policy;
    Ok(success(request_id, policy.as_str().to_string()))
}
