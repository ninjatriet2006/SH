/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Tầng API mỏng — chuyển cho `actions::trash_{list,restore,delete}`.
  Giữ nguyên tên lệnh. Được gọi từ `services/trashOps.ts` trong frontend.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::actions::list::FileItem;
use crate::actions::trash_list::TrashItemLocal;
use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::actions::{trash_delete, trash_list, trash_restore};
use crate::logic::fastlane::fastlane;

/// Bóc tên remote từ tham số Frontend gửi xuống, bỏ dấu ':' nếu có.
fn require_remote(account: Option<String>) -> Result<String, String> {
    let name = account.unwrap_or_default().trim().trim_end_matches(':').to_string();
    if name.is_empty() || name == "Local" {
        return Err("Thiếu tên remote (thùng rác đám mây chỉ áp dụng cho ổ Cloud).".to_string());
    }
    Ok(name)
}

#[tauri::command]
pub async fn fs_trash_list_local() -> Result<Vec<TrashItemLocal>, String> {
    fastlane(trash_list::list_local).await
}

#[tauri::command]
pub async fn fs_trash_restore_local(item_id: String) -> Result<(), String> {
    fastlane(move || trash_restore::restore_local(&item_id)).await
}

#[tauri::command]
pub async fn fs_trash_delete_local(item_id: String) -> Result<(), String> {
    fastlane(move || trash_delete::delete_local(&item_id, DeleteScope::NoTrash)).await
}

#[tauri::command]
pub async fn fs_trash_empty_local() -> Result<(), String> {
    fastlane(move || trash_delete::empty_local(EmptyDirs::Recursive)).await
}

#[tauri::command]
pub async fn fs_trash_list_remote_terminal(account: Option<String>) -> Result<Vec<FileItem>, String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_list::list_remote(&remote)).await
}

#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(
    account: Option<String>,
    path: String,
) -> Result<(), String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_restore::restore_remote(&remote, &path)).await
}

#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(
    account: Option<String>,
    path: String,
) -> Result<(), String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_delete::delete_remote(&remote, &path, DeleteScope::NoTrash)).await
}

#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(account: Option<String>) -> Result<(), String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_delete::empty_remote(&remote, EmptyDirs::Recursive)).await
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
