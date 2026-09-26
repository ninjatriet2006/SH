/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉnh-sửa File (Command Tauri).
- Trách nhiệm: Nhận arg trực tiếp từ Frontend, gọi tầng `logic`/`actions`.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::actions::perm::Policy;
use crate::logic::app_state::AppState;
use tauri::State;

/// Legacy API: Tạo thư mục trực tiếp.
/// Khuyến nghị: Frontend nên dùng `job_enqueue` với `kind: "mkdir"` để quản lý tập trung qua hàng đợi.
#[deprecated(note = "Thao tác biến đổi tệp nên dùng job_enqueue để quản lý qua hàng đợi tập trung")]
#[tauri::command]
pub async fn fs_mkdir(path: String) -> Result<(), String> {
    crate::actions::create::execute_mkdir(path, Policy::AllowSystem).await
}

/// Legacy API: Tạo tệp rỗng trực tiếp.
/// Khuyến nghị: Frontend nên dùng `job_enqueue` với `kind: "touch"` để quản lý tập trung qua hàng đợi.
#[deprecated(note = "Thao tác biến đổi tệp nên dùng job_enqueue để quản lý qua hàng đợi tập trung")]
#[tauri::command]
pub async fn fs_touch(path: String) -> Result<(), String> {
    crate::actions::create::execute_touch(path, Policy::AllowSystem).await
}

/// Legacy API: Xóa tệp/thư mục trực tiếp.
/// Khuyến nghị: Frontend nên dùng `job_enqueue` với `kind: "delete"` để quản lý tập trung qua hàng đợi.
#[deprecated(note = "Thao tác biến đổi tệp nên dùng job_enqueue để quản lý qua hàng đợi tập trung")]
#[tauri::command]
pub async fn fs_delete(path: String) -> Result<(), String> {
    crate::actions::delete_op::execute_delete(
        path,
        crate::actions::delete_op::DeleteScope::NoTrash,
    )
    .await
}

/// Legacy API: Đổi tên/di chuyển tệp trực tiếp.
/// Khuyến nghị: Frontend nên dùng `job_enqueue` với `kind: "rename"` để quản lý tập trung qua hàng đợi.
#[deprecated(note = "Thao tác biến đổi tệp nên dùng job_enqueue để quản lý qua hàng đợi tập trung")]
#[tauri::command]
pub async fn fs_rename(old_path: String, new_path: String) -> Result<(), String> {
    crate::actions::rename::execute_rename(old_path, new_path, Policy::AllowSystem).await
}

/// S2: chmod tôn trọng policy — policy đọc từ State.
#[tauri::command]
pub async fn fs_chmod(state: State<'_, AppState>, path: String, mode: u32) -> Result<(), String> {
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    crate::actions::perm::execute_chmod(path, mode, policy).await
}

/// S2: chown tôn trọng policy — policy đọc từ State.
#[tauri::command]
pub async fn fs_chown(
    state: State<'_, AppState>,
    path: String,
    uid: u32,
    gid: u32,
) -> Result<(), String> {
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    crate::actions::perm::execute_chown(path, uid, gid, policy).await
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
