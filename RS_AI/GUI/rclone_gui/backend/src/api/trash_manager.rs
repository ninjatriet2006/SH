/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Tầng API mỏng — chuyển cho `actions::trash_{list,restore,delete}`.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::actions::list::FileItem;
use crate::actions::trash_list::TrashItemLocal;
use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::actions::{trash_delete, trash_list, trash_restore};
use crate::ipc::{
    async_command_result, deserialize_present_nullable, Empty, IpcErrorCode, IpcResult, Req,
};
use crate::logic::fastlane::fastlane;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashIdRequest {
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTrashListRequest {
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTrashItemRequest {
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub account: Option<String>,
    pub path: String,
}

/// Bóc tên remote từ tham số Frontend gửi xuống, bỏ dấu ':' nếu có.
pub(crate) fn require_remote(account: Option<String>) -> Result<String, String> {
    let name = account.unwrap_or_default().trim().trim_end_matches(':').to_string();
    if name.is_empty() || name == "Local" {
        return Err("Thiếu tên remote (thùng rác đám mây chỉ áp dụng cho ổ Cloud).".to_string());
    }
    Ok(name)
}

#[tauri::command]
pub async fn fs_trash_list_local(request: Req<Empty>) -> IpcResult<Vec<TrashItemLocal>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(trash_list::list_local).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_restore_local(request: Req<TrashIdRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        fastlane(move || trash_restore::restore_local(&p.item_id)).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_delete_local(request: Req<TrashIdRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        fastlane(move || trash_delete::delete_local(&p.item_id, DeleteScope::NoTrash)).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_empty_local(request: Req<Empty>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(move || trash_delete::empty_local(EmptyDirs::Recursive)).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_list_remote_terminal(
    request: Req<RemoteTrashListRequest>,
) -> IpcResult<Vec<FileItem>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        let remote = require_remote(p.account)?;
        fastlane(move || trash_list::list_remote(&remote)).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(
    request: Req<RemoteTrashItemRequest>,
) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        let remote = require_remote(p.account)?;
        fastlane(move || trash_restore::restore_remote(&remote, &p.path)).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(
    request: Req<RemoteTrashItemRequest>,
) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        let remote = require_remote(p.account)?;
        fastlane(move || trash_delete::delete_remote(&remote, &p.path, DeleteScope::NoTrash)).await
    })
    .await
}

#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(
    request: Req<RemoteTrashListRequest>,
) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        let remote = require_remote(p.account)?;
        fastlane(move || trash_delete::empty_remote(&remote, EmptyDirs::Recursive)).await
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_remote_normalizes_and_validates() {
        assert_eq!(require_remote(Some("GDrive".into())).unwrap(), "GDrive");
        assert_eq!(require_remote(Some("GDrive:".into())).unwrap(), "GDrive");
        assert!(require_remote(None).is_err());
        assert!(require_remote(Some("Local".into())).is_err());
        assert!(require_remote(Some("".into())).is_err());
    }
}
