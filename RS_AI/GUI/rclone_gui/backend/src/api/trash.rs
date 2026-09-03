/*
[INTEGRITY NOTES]
- Mục đích: Quản lý tính năng Thùng rác (Trash) cho cả Local và Remote (Cloud).
- Trách nhiệm: Định nghĩa và xử lý các thao tác xóa tạm thời, khôi phục, dọn dẹp thùng rác. Giao tiếp với `gio trash` cho Local và `rclone` cho Remote.
- Tương tác: Được gọi từ `trashOps.ts` trong frontend. Dùng `utils` để giao tiếp với rclone.
*/

use crate::api::files::FileItem; // Sử dụng FileItem từ thư mục gốc (lib.rs)
use crate::core::rclone;
use crate::core::task::blocking;
use serde::Serialize;
use std::process::Command;

// ====================================================================================
// BLOCK: THÙNG RÁC CỤC BỘ (LOCAL)
// ====================================================================================

/// Khai báo dữ liệu trả về cho Thùng rác Local
#[derive(Serialize)]
pub struct TrashItemLocal {
    pub id: String,
    pub name: String,
    pub original_path: String,
    pub time_deleted: String,
}

/// Tên hàm: fs_trash_list_local
/// Mô tả: Lấy danh sách file trong thùng rác cục bộ (Local).
#[tauri::command]
pub async fn fs_trash_list_local() -> Result<Vec<TrashItemLocal>, String> {
    // Tạm thời trả về danh sách rỗng để tránh văng lỗi trên Frontend
    Ok(vec![])
}

/// Tên hàm: fs_trash_restore_local
/// Mô tả: Khôi phục một file cụ thể từ thùng rác cục bộ
#[tauri::command]
pub async fn fs_trash_restore_local(item_id: String) -> Result<(), String> {
    Err(format!(
        "Chức năng khôi phục (id: {}) chưa được hỗ trợ hoàn chỉnh.",
        item_id
    ))
}

/// Tên hàm: fs_trash_empty_local
/// Mô tả: Làm sạch toàn bộ thùng rác cục bộ bằng công cụ `gio trash --empty` (Linux)
#[tauri::command]
pub async fn fs_trash_empty_local() -> Result<(), String> {
    blocking(|| {
        // Chạy đồng bộ và kiểm tra exit code — trước đây dùng spawn() nên mọi lỗi
        // (gio không tồn tại, thiếu quyền) đều bị báo là thành công cho Frontend.
        let output = Command::new("gio")
            .arg("trash")
            .arg("--empty")
            .output()
            .map_err(|e| format!("Lỗi khi dọn thùng rác hệ thống: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if err.is_empty() {
                format!("Lệnh `gio trash --empty` thất bại: {}", output.status)
            } else {
                err
            });
        }
        Ok(())
    })
    .await
}

// ====================================================================================
// BLOCK: THÙNG RÁC ĐÁM MÂY (CLOUD/REMOTE)
// ====================================================================================

/// Tên hàm: fs_trash_list_remote_terminal
/// Mô tả: Lấy danh sách file trong thùng rác đám mây (Cloud)
#[tauri::command]
pub async fn fs_trash_list_remote_terminal(_account: Option<String>) -> Result<Vec<FileItem>, String> {
    Ok(vec![])
}

/// Tên hàm: fs_trash_restore_remote_terminal
/// Mô tả: Khôi phục file trong thùng rác đám mây
#[tauri::command]
pub async fn fs_trash_restore_remote_terminal(_account: Option<String>, idx: usize) -> Result<(), String> {
    Err(format!("Khôi phục file từ Cloud chưa hỗ trợ. (idx: {})", idx))
}

/// Tên hàm: fs_trash_delete_remote_terminal
/// Mô tả: Xóa vĩnh viễn 1 file trong thùng rác đám mây
#[tauri::command]
pub async fn fs_trash_delete_remote_terminal(_account: Option<String>, idx: usize) -> Result<(), String> {
    Err(format!("Xóa vĩnh viễn 1 file từ Cloud chưa hỗ trợ. (idx: {})", idx))
}

/// Tên hàm: fs_trash_empty_remote_terminal
/// Mô tả: Xóa vĩnh viễn toàn bộ thùng rác đám mây (Empty Trash) thông qua rclone cleanup
#[tauri::command]
pub async fn fs_trash_empty_remote_terminal(account: Option<String>) -> Result<(), String> {
    blocking(move || {
        if let Some(remote) = account {
            let target = format!("{}:", remote);
            // Tái sử dụng helper spawn_rclone_cmd
            rclone::spawn_cmd(&["cleanup", &target])?;
            Ok(())
        } else {
            Err("Lỗi: Không tìm thấy tên remote để dọn dẹp.".to_string())
        }
    })
    .await
}
