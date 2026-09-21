/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Tầng API mỏng — nhận request từ Frontend rồi chuyển cho
  `actions::trash_{list,restore,delete}` (Route Local/Remote đã unify). Giữ nguyên tên IPC/api.
- Tương tác: Được gọi từ `services/trashOps.ts` trong frontend.
*/

use crate::actions::{trash_delete, trash_list, trash_restore};
use crate::actions::types::{DeleteScope, EmptyDirs};
use crate::api::files::FileItem;
use crate::logic::fastlane::fastlane;
use serde::Serialize;

// ====================================================================================
// BLOCK: THÙNG RÁC CỤC BỘ (LOCAL)
// ====================================================================================

/// Khai báo dữ liệu trả về cho Thùng rác Local.
/// `id` là tên mục trong `Trash/files/` — dùng để khôi phục / xoá vĩnh viễn.
#[derive(Serialize)]
pub struct TrashItemLocal {
    pub id: String,
    pub name: String,
    pub original_path: String,
    pub time_deleted: String,
}

/// Tên hàm: fs_trash_list_local
/// Mô tả: Lấy danh sách mục trong thùng rác cục bộ, mới xoá xếp trước.
pub async fn fs_trash_list_local() -> Result<Vec<TrashItemLocal>, String> {
    fastlane(trash_list::list_local).await
}

/// Tên hàm: fs_trash_restore_local
/// Mô tả: Khôi phục một mục từ thùng rác cục bộ về vị trí gốc.
pub async fn fs_trash_restore_local(item_id: String) -> Result<(), String> {
    fastlane(move || trash_restore::restore_local(&item_id)).await
}

/// Tên hàm: fs_trash_delete_local
/// Mô tả: Xoá vĩnh viễn một mục khỏi thùng rác cục bộ.
pub async fn fs_trash_delete_local(item_id: String) -> Result<(), String> {
    fastlane(move || trash_delete::delete_local(&item_id, DeleteScope::NoTrash)).await
}

/// Tên hàm: fs_trash_empty_local
/// Mô tả: Xoá vĩnh viễn toàn bộ mục trong thùng rác cục bộ.
pub async fn fs_trash_empty_local() -> Result<(), String> {
    fastlane(move || trash_delete::empty_local(EmptyDirs::Recursive)).await
}

// ====================================================================================
// BLOCK: THÙNG RÁC ĐÁM MÂY (CLOUD/REMOTE)
// ====================================================================================

/// Bóc tên remote từ tham số Frontend gửi xuống, bỏ dấu ':' nếu có.
fn require_remote(account: Option<String>) -> Result<String, String> {
    let name = account.unwrap_or_default().trim().trim_end_matches(':').to_string();
    if name.is_empty() || name == "Local" {
        return Err("Thiếu tên remote (thùng rác đám mây chỉ áp dụng cho ổ Cloud).".to_string());
    }
    Ok(name)
}

/// Tên hàm: fs_trash_list_remote_terminal
/// Mô tả: Liệt kê các mục trong thùng rác của remote (Google Drive, Jottacloud, PikPak).
pub async fn fs_trash_list_remote_terminal(account: Option<String>) -> Result<Vec<FileItem>, String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_list::list_remote(&remote)).await
}

/// Tên hàm: fs_trash_restore_remote_terminal
/// Mô tả: Khôi phục một mục trong thùng rác đám mây về vị trí gốc (chỉ Drive).
pub async fn fs_trash_restore_remote_terminal(account: Option<String>, path: String) -> Result<(), String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_restore::restore_remote(&remote, &path)).await
}

/// Tên hàm: fs_trash_delete_remote_terminal
/// Mô tả: Xoá vĩnh viễn một mục đang ở trong thùng rác đám mây.
pub async fn fs_trash_delete_remote_terminal(account: Option<String>, path: String) -> Result<(), String> {
    let remote = require_remote(account)?;
    fastlane(move || trash_delete::delete_remote(&remote, &path, DeleteScope::NoTrash)).await
}

/// Tên hàm: fs_trash_empty_remote_terminal
/// Mô tả: Dọn sạch toàn bộ thùng rác đám mây (`rclone cleanup`).
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
