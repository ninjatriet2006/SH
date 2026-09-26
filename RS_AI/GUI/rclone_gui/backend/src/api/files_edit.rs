/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉnh-sửa File (Command Tauri).
- Trách nhiệm: Nhận Enveloped IPC `Req<T>`, gọi tầng `logic`/`actions`, trả `IpcResult<T>`.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::actions::perm::Policy;
use crate::ipc::{command_result, Empty, IpcErrorCode, IpcResult, Req};
use crate::logic::app_state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsChmodRequest {
    pub path: String,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsChownRequest {
    pub path: String,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPermissionPolicyRequest {
    pub policy: String,
}

#[tauri::command]
pub fn fs_chmod(state: State<'_, AppState>, request: Req<FsChmodRequest>) -> IpcResult<()> {
    command_result(request, IpcErrorCode::Forbidden, |p| {
        let policy = state.policy.lock().map(|pol| *pol).unwrap_or_default();
        crate::actions::instant::execute_chmod_sync(&p.path, p.mode, policy)
    })
}

#[tauri::command]
pub fn fs_chown(state: State<'_, AppState>, request: Req<FsChownRequest>) -> IpcResult<()> {
    command_result(request, IpcErrorCode::Forbidden, |p| {
        let policy = state.policy.lock().map(|pol| *pol).unwrap_or_default();
        crate::actions::instant::execute_chown_sync(&p.path, p.uid, p.gid, policy)
    })
}

#[tauri::command]
pub fn get_permission_policy(state: State<'_, AppState>, request: Req<Empty>) -> IpcResult<String> {
    command_result(request, IpcErrorCode::Internal, |_| {
        let policy = state.policy.lock().map(|pol| *pol).unwrap_or_default();
        Ok(policy.as_str().to_string())
    })
}

#[tauri::command]
pub fn set_permission_policy(
    state: State<'_, AppState>,
    request: Req<SetPermissionPolicyRequest>,
) -> IpcResult<String> {
    command_result(request, IpcErrorCode::InvalidArgument, |p| {
        let parsed = Policy::parse(p.policy.trim())
            .ok_or_else(|| "policy must be deny|ask_once|allow_system".to_string())?;
        *state
            .policy
            .lock()
            .map_err(|_| "policy lock poisoned".to_string())? = parsed;
        Ok(parsed.as_str().to_string())
    })
}
