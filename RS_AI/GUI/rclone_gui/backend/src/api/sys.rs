/*
[INTEGRITY NOTES]
- Mục đích: API hệ điều hành / ứng dụng ngoài (Open With, clipboard, custom actions).
- Trách nhiệm: Tầng API mỏng — validate Req rồi gọi `actions::view` + `logic::{clipboard,custom_action}`.
- Tương tác: Giữ nguyên tên lệnh Tauri + payload JSON + IpcError (dời từ `ipc.rs`).
*/
// Mỗi lệnh đúng 1 pub command fn: validate Req → gọi actions/logic trực tiếp → map backend_error.

use super::envelope::{Empty, IpcResult, Req, backend_error, success, validate};

crate::payload!(OpenWithPayload { path: String, exec_cmd: Option<String>, app: Option<String> });
crate::payload!(ClipboardSetPayload { items: Vec<crate::logic::clipboard::OSClipboardItem>, is_cut: bool });
crate::payload!(FilesPayload { files: Vec<crate::logic::custom_action::SimpleFileItem> });
crate::payload!(CustomActionPayload { exec_template: String, base_path: String, file_names: Vec<String> });

#[tauri::command]
pub async fn sys_open_with(request: Req<OpenWithPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::actions::view::sys_open_with(payload.path, payload.exec_cmd, payload.app)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn sys_list_apps(
    request: Req<Empty>,
) -> IpcResult<Vec<crate::actions::view::DesktopApp>> {
    let request_id = validate(&request)?;
    crate::actions::view::sys_list_apps()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn os_clipboard_set(request: Req<ClipboardSetPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::logic::clipboard::os_clipboard_set(payload.items, payload.is_cut)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn os_clipboard_get(
    request: Req<Empty>,
) -> IpcResult<Option<crate::logic::clipboard::OSClipboardData>> {
    let request_id = validate(&request)?;
    crate::logic::clipboard::os_clipboard_get()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn sys_get_custom_actions(
    request: Req<Empty>,
) -> IpcResult<Vec<crate::logic::custom_action::CustomAction>> {
    let request_id = validate(&request)?;
    crate::logic::custom_action::sys_get_custom_actions()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn sys_get_valid_actions(
    request: Req<FilesPayload>,
) -> IpcResult<Vec<crate::logic::custom_action::CustomAction>> {
    let request_id = validate(&request)?;
    crate::logic::custom_action::sys_get_valid_actions(request.payload.files)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn sys_execute_custom_action(request: Req<CustomActionPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::logic::custom_action::sys_execute_custom_action(
        payload.exec_template,
        payload.base_path,
        payload.file_names,
    )
    .await
    .map(|data| success(request_id, data))
    .map_err(backend_error)
}
