/* 
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Định nghĩa và xử lý các thao tác xóa tạm thời, khôi phục, dọn dẹp thùng rác. Giao tiếp với `gio trash` cho Local và `rclone` cho Remote.
- Tương tác: Được gọi từ `trashOps.ts` trong frontend.
*/

use serde::Serialize;
use std::process::Command;
use crate::FileItem; // Sử dụng FileItem từ thư mục gốc (lib.rs)

/// Khai báo dữ liệu trả về cho Thùng rác Local
#[derive(Serialize)]
pub struct TrashItemLocal {
    pub id: String,
    pub name: String,
    pub original_path: String,
    pub time_deleted: String,
}

/// Lấy danh sách file trong thùng rác cục bộ (Local)
/// Sử dụng công cụ gio của Linux (hoặc trả về giả lập nếu không hỗ trợ)
#[tauri::command]
pub async fn fs_trash_list_local() -> Result<Vec<TrashItemLocal>, String> {
    // Tạm thời trả về danh sách rỗng để tránh văng lỗi trên Frontend
    Ok(vec![])
}

/// Khôi phục một file cụ thể từ thùng rác cục bộ
#[tauri::command]
pub async fn fs_trash_restore_local(item_id: String) -> Result<(), String> {
    // Tạm thời chưa có logic vì phụ thuộc vào thư viện bên thứ 3 hoặc `gio trash`
    Err(format!("Chức năng khôi phục (id: {}) chưa được hỗ trợ hoàn chỉnh.", item_id))
}

/// Làm sạch toàn bộ thùng rác cục bộ
#[tauri::command]
pub async fn fs_trash_empty_local() -> Result<(), String> {
    // Sử dụng `gio trash --empty` (thịnh hành trên Linux)
    Command::new("gio")
        .arg("trash")
        .arg("--empty")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Lấy danh sách file trong thùng rác đám mây (Cloud - Remote)
/// Lệnh rclone: `rclone backend trash <remote>:`
#[tauri::command]
pub async fn fs_trash_list_remote_terminal(_account: Option<String>) -> Result<Vec<FileItem>, String> {
    // Tạm thời trả về danh sách rỗng để ngăn chặn lỗi Command Not Found
    Ok(vec![])
}

/// Khôi phục file trong thùng rác đám mây
#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(_account: Option<String>, idx: usize) -> Result<(), String> {
    Err(format!("Khôi phục file từ Cloud chưa hỗ trợ. (idx: {})", idx))
}

/// Xóa vĩnh viễn 1 file trong thùng rác đám mây
#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(_account: Option<String>, idx: usize) -> Result<(), String> {
    Err(format!("Xóa vĩnh viễn 1 file từ Cloud chưa hỗ trợ. (idx: {})", idx))
}

/// Xóa vĩnh viễn toàn bộ thùng rác đám mây (Empty Trash)
/// Lệnh rclone: `rclone cleanup <remote>:`
#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(account: Option<String>) -> Result<(), String> {
    if let Some(remote) = account {
        Command::new("rclone")
            .arg("cleanup")
            .arg(format!("{}:", remote))
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Lỗi: Không tìm thấy tên remote để dọn dẹp.".to_string())
    }
}
