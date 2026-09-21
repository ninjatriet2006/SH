/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Tầng API mỏng — nhận request từ Frontend rồi chuyển cho
  `actions::trash_{list,restore,delete}` (Route Local/Remote đã unify). Giữ nguyên tên IPC/api.
- Tương tác: Được gọi từ `services/trashOps.ts` trong frontend.
*/
// Mỗi lệnh đúng 1 pub command fn: validate Req → gọi actions trực tiếp → map backend_error.

use super::envelope::{Empty, IpcResult, Req, backend_error, success, validate};
use crate::actions::list::FileItem;
use crate::actions::trash_list::TrashItemLocal;
use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::actions::{trash_delete, trash_list, trash_restore};
use crate::logic::fastlane::fastlane;

crate::payload!(TrashItemPayload { item_id: String });
crate::payload!(TrashAccountPayload { account: Option<String> });
crate::payload!(RemoteTrashItemPayload { account: Option<String>, path: String });

/// Bóc tên remote từ tham số Frontend gửi xuống, bỏ dấu ':' nếu có.
fn require_remote(account: Option<String>) -> Result<String, String> {
    let name = account.unwrap_or_default().trim().trim_end_matches(':').to_string();
    if name.is_empty() || name == "Local" {
        return Err("Thiếu tên remote (thùng rác đám mây chỉ áp dụng cho ổ Cloud).".to_string());
    }
    Ok(name)
}

#[tauri::command]
pub async fn fs_trash_list_local(request: Req<Empty>) -> IpcResult<Vec<TrashItemLocal>> {
    let request_id = validate(&request)?;
    fastlane(trash_list::list_local)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_restore_local(request: Req<TrashItemPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let item_id = request.payload.item_id;
    fastlane(move || trash_restore::restore_local(&item_id))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_delete_local(request: Req<TrashItemPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let item_id = request.payload.item_id;
    fastlane(move || trash_delete::delete_local(&item_id, DeleteScope::NoTrash))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_empty_local(request: Req<Empty>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    fastlane(move || trash_delete::empty_local(EmptyDirs::Recursive))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_list_remote_terminal(
    request: Req<TrashAccountPayload>,
) -> IpcResult<Vec<FileItem>> {
    let request_id = validate(&request)?;
    let remote = require_remote(request.payload.account).map_err(backend_error)?;
    fastlane(move || trash_list::list_remote(&remote))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(request: Req<RemoteTrashItemPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let remote = require_remote(payload.account).map_err(backend_error)?;
    fastlane(move || trash_restore::restore_remote(&remote, &payload.path))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(request: Req<RemoteTrashItemPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let remote = require_remote(payload.account).map_err(backend_error)?;
    fastlane(move || trash_delete::delete_remote(&remote, &payload.path, DeleteScope::NoTrash))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(request: Req<TrashAccountPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let remote = require_remote(request.payload.account).map_err(backend_error)?;
    fastlane(move || trash_delete::empty_remote(&remote, EmptyDirs::Recursive))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_remote_normalizes_and_validates() {
        assert_eq!(require_remote(Some("GDrive".into())).unwrap(), "GDrive");
        // Frontend có thể gửi kèm dấu ':' — phải bóc ra.
        assert_eq!(require_remote(Some("GDrive:".into())).unwrap(), "GDrive");
        assert_eq!(require_remote(Some("  GDrive  ".into())).unwrap(), "GDrive");

        assert!(require_remote(None).is_err());
        assert!(require_remote(Some("".into())).is_err());
        assert!(require_remote(Some("Local".into())).is_err());
    }
}
