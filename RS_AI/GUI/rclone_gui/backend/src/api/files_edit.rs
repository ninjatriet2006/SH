/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉnh-sửa File (Command Tauri).
- Trách nhiệm: Nhận arg trực tiếp từ Frontend, gọi tầng `logic`/`actions`.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::actions::perm::Policy;
use crate::logic::app_state::AppState;
use tauri::State;



/// S2: chmod tôn trọng policy — policy đọc từ State.
/// GHI CHÚ RUNTIME: Lệnh synchronous chạy trên blocking thread pool của Tauri (`spawn_blocking`).
/// Không bọc async/fastlane do là thao tác biến đổi POSIX nhanh trên Local FS.
#[tauri::command]
pub fn fs_chmod(state: State<'_, AppState>, path: String, mode: u32) -> Result<(), String> {
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    crate::actions::instant::execute_chmod_sync(&path, mode, policy)
}

/// S2: chown tôn trọng policy — policy đọc từ State.
/// GHI CHÚ RUNTIME: Tương tự `fs_chmod`, lệnh synchronous chạy trên blocking thread pool
/// của Tauri, xử lý leo quyền sudo (nếu cần) và gán quyền sở hữu Local FS.
#[tauri::command]
pub fn fs_chown(
    state: State<'_, AppState>,
    path: String,
    uid: u32,
    gid: u32,
) -> Result<(), String> {
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    crate::actions::instant::execute_chown_sync(&path, uid, gid, policy)
}

/// S2: đọc policy hiện tại (`deny`/`ask_once`/`allow_system`).
#[tauri::command]
pub fn get_permission_policy(state: State<'_, AppState>) -> Result<String, String> {
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    Ok(policy.as_str().to_string())
}

/// S2: đặt policy — sai chuỗi trả lỗi String trần.
#[tauri::command]
pub fn set_permission_policy(state: State<'_, AppState>, policy: String) -> Result<String, String> {
    let parsed = Policy::parse(policy.trim())
        .ok_or_else(|| "policy must be deny|ask_once|allow_system".to_string())?;
    *state
        .policy
        .lock()
        .map_err(|_| "policy lock poisoned".to_string())? = parsed;
    Ok(parsed.as_str().to_string())
}
