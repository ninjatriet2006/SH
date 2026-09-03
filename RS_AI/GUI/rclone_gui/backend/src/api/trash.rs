/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Tầng API mỏng — nhận request từ Frontend rồi chuyển cho
  `logic::trash_local` (chuẩn FreeDesktop) hoặc `logic::trash_remote` (rclone).
- Tương tác: Được gọi từ `services/trashOps.ts` trong frontend.
*/

use crate::api::files::FileItem;
use crate::core::task::blocking;
use crate::logic::{trash_local, trash_remote};
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
#[tauri::command]
pub async fn fs_trash_list_local() -> Result<Vec<TrashItemLocal>, String> {
    blocking(trash_local::list).await
}

/// Tên hàm: fs_trash_restore_local
/// Mô tả: Khôi phục một mục từ thùng rác cục bộ về vị trí gốc.
#[tauri::command]
pub async fn fs_trash_restore_local(item_id: String) -> Result<(), String> {
    blocking(move || trash_local::restore(&item_id)).await
}

/// Tên hàm: fs_trash_delete_local
/// Mô tả: Xoá vĩnh viễn một mục khỏi thùng rác cục bộ.
#[tauri::command]
pub async fn fs_trash_delete_local(item_id: String) -> Result<(), String> {
    blocking(move || trash_local::delete(&item_id)).await
}

/// Tên hàm: fs_trash_empty_local
/// Mô tả: Xoá vĩnh viễn toàn bộ mục trong thùng rác cục bộ.
#[tauri::command]
pub async fn fs_trash_empty_local() -> Result<(), String> {
    blocking(|| {
        let items = trash_local::list()?;
        let mut errors = Vec::new();
        for item in &items {
            if let Err(e) = trash_local::delete(&item.id) {
                errors.push(format!("{}: {}", item.name, e));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("Không xoá được {} mục:\n{}", errors.len(), errors.join("\n")))
        }
    })
    .await
}

// ====================================================================================
// BLOCK: THÙNG RÁC ĐÁM MÂY (CLOUD/REMOTE)
// ====================================================================================

/// Bóc tên remote từ tham số Frontend gửi xuống, bỏ dấu ':' nếu có.
fn require_remote(account: Option<String>) -> Result<String, String> {
    let name = account
        .unwrap_or_default()
        .trim()
        .trim_end_matches(':')
        .to_string();
    if name.is_empty() || name == "Local" {
        return Err("Thiếu tên remote (thùng rác đám mây chỉ áp dụng cho ổ Cloud).".to_string());
    }
    Ok(name)
}

/// Tên hàm: fs_trash_list_remote_terminal
/// Mô tả: Liệt kê các mục trong thùng rác của remote (Google Drive, Jottacloud, PikPak).
#[tauri::command]
pub async fn fs_trash_list_remote_terminal(account: Option<String>) -> Result<Vec<FileItem>, String> {
    let remote = require_remote(account)?;
    blocking(move || trash_remote::list(&remote)).await
}

/// Tên hàm: fs_trash_restore_remote_terminal
/// Mô tả: Khôi phục một mục trong thùng rác đám mây về vị trí gốc.
#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(account: Option<String>, path: String) -> Result<(), String> {
    let remote = require_remote(account)?;
    blocking(move || trash_remote::restore(&remote, &path)).await
}

/// Tên hàm: fs_trash_delete_remote_terminal
/// Mô tả: Xoá vĩnh viễn một mục đang ở trong thùng rác đám mây.
#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(account: Option<String>, path: String) -> Result<(), String> {
    let remote = require_remote(account)?;
    blocking(move || trash_remote::delete(&remote, &path)).await
}

/// Tên hàm: fs_trash_empty_remote_terminal
/// Mô tả: Dọn sạch toàn bộ thùng rác đám mây (`rclone cleanup`).
#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(account: Option<String>) -> Result<(), String> {
    let remote = require_remote(account)?;
    blocking(move || trash_remote::empty(&remote)).await
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
