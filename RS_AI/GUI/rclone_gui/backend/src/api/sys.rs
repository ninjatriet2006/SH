/*
[INTEGRITY NOTES]
- Mục đích: API hệ điều hành / ứng dụng ngoài (Open With, clipboard, custom actions).
- Trách nhiệm: Tầng API mỏng — gọi `actions::view` + `logic::{clipboard,custom_action}`.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::ipc::{
    async_command_result, deserialize_present_nullable, Empty, IpcErrorCode, IpcResult, Req,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysOpenWithRequest {
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub exec_cmd: Option<String>,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub app: Option<String>,
}

#[derive(Deserialize)]
pub struct OsClipboardSetRequest {
    pub items: Vec<crate::logic::clipboard::OSClipboardItem>,
    pub is_cut: bool,
}

#[derive(Deserialize)]
pub struct SysGetValidActionsRequest {
    pub files: Vec<crate::logic::custom_action::SimpleFileItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysExecuteCustomActionRequest {
    pub exec_template: String,
    pub base_path: String,
    pub file_names: Vec<String>,
}

#[tauri::command]
pub async fn sys_open_with(request: Req<SysOpenWithRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::view::sys_open_with(p.path, p.exec_cmd, p.app).await
    })
    .await
}

#[tauri::command]
pub async fn sys_list_apps(
    request: Req<Empty>,
) -> IpcResult<Vec<crate::actions::system::DesktopApp>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        crate::actions::system::sys_list_apps().await
    })
    .await
}

#[tauri::command]
pub async fn os_clipboard_set(request: Req<OsClipboardSetRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Internal, |p| async move {
        crate::logic::clipboard::os_clipboard_set(p.items, p.is_cut).await
    })
    .await
}

#[tauri::command]
pub async fn os_clipboard_get(
    request: Req<Empty>,
) -> IpcResult<Option<crate::logic::clipboard::OSClipboardData>> {
    async_command_result(request, IpcErrorCode::Internal, |_| async move {
        crate::logic::clipboard::os_clipboard_get().await
    })
    .await
}

#[tauri::command]
pub async fn sys_get_custom_actions(
    request: Req<Empty>,
) -> IpcResult<Vec<crate::logic::custom_action::CustomAction>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        crate::logic::custom_action::sys_get_custom_actions().await
    })
    .await
}

#[tauri::command]
pub async fn sys_get_valid_actions(
    request: Req<SysGetValidActionsRequest>,
) -> IpcResult<Vec<crate::logic::custom_action::CustomAction>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::logic::custom_action::sys_get_valid_actions(p.files).await
    })
    .await
}

#[tauri::command]
pub async fn sys_execute_custom_action(
    request: Req<SysExecuteCustomActionRequest>,
) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Internal, |p| async move {
        crate::logic::custom_action::sys_execute_custom_action(
            p.exec_template,
            p.base_path,
            p.file_names,
        )
        .await
    })
    .await
}
