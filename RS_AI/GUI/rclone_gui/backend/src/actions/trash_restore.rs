/*
[INTEGRITY NOTES]
- Mục đích: Khôi phục mục từ thùng rác Local (`gio`) + Remote (`drive untrash`) (S1 unify).
- Trách nhiệm: Phân tuyến (Route) → chọn nhánh Local/Remote bằng `match` + UNIVERSAL.
- Tương tác: Tầng `api::trash` bọc mỏng qua `logic::fastlane::fastlane`. Không đụng IPC/frontend.
  Giữ nguyên tắc backend-hỗ-trợ: restore remote chỉ `drive` (`backend untrash`).
*/

pub use super::trash_list::Route;
use super::trash_list::{percent_encode, remote_type};
use crate::core::rclone_caller;
use serde_json::Value;

/// Khôi phục một mục local về vị trí gốc. Dùng `gio trash --restore` vì nó tự
/// tạo lại thư mục cha nếu đã bị xoá, và không ghi đè file đang tồn tại.
fn restore_local_inner(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Thiếu định danh mục cần khôi phục.".to_string());
    }

    let uri = format!("trash:///{}", percent_encode(id));
    let output = std::process::Command::new("gio")
        .args(["trash", "--restore", &uri])
        .output()
        .map_err(|e| format!("Lỗi khi gọi gio: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Khôi phục '{}' thất bại: {}", id, output.status)
        } else {
            err
        });
    }
    Ok(())
}

/// Khôi phục một mục remote khỏi thùng rác. Chỉ Google Drive hỗ trợ (`backend untrash`).
fn restore_remote_inner(remote: &str, path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Thiếu đường dẫn mục cần khôi phục.".to_string());
    }

    let backend = remote_type(remote)?;
    if backend != "drive" {
        return Err(format!(
            "rclone không hỗ trợ khôi phục từ thùng rác cho loại '{}'. Hiện chỉ Google Drive làm được (rclone backend untrash).",
            backend
        ));
    }

    let target = format!("{}:{}", remote, path);
    let output = rclone_caller::run_cmd(&["backend", "untrash", &target])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Khôi phục '{}' thất bại: {}", path, output.status)
        } else {
            err
        });
    }

    // `untrash` báo số mục đã xử lý; 0 nghĩa là không tìm thấy gì trong thùng rác.
    if let Ok(res) = serde_json::from_slice::<Value>(&output.stdout) {
        if res.get("Untrashed").and_then(|v| v.as_u64()) == Some(0) {
            return Err(format!("Không tìm thấy '{}' trong thùng rác để khôi phục.", path));
        }
    }
    Ok(())
}

/// Khôi phục một mục local (đồng bộ; tầng api bọc `blocking`).
pub fn restore_local(id: &str) -> Result<(), String> {
    match Route::Local {
        // UNIVERSAL: Local khôi phục qua `gio trash --restore` (tự dựng lại thư mục cha).
        Route::Local => restore_local_inner(id),
        // UNIVERSAL: nhánh Remote không xảy ra ở hàm local — giữ để `match` đủ đầy.
        Route::Remote => Err("Tuyến Remote phải dùng `restore_remote`.".to_string()),
    }
}

/// Khôi phục một mục remote (đồng bộ; tầng api bọc `blocking`).
pub fn restore_remote(remote: &str, path: &str) -> Result<(), String> {
    match Route::classify(remote) {
        // UNIVERSAL: classifier đã loại `"Local"` ở tầng api nên nhánh này là lỗi lập trình.
        Route::Local => Err("Tuyến Local phải dùng `restore_local`.".to_string()),
        // UNIVERSAL: remote chỉ `drive` khôi phục được (`backend untrash`), loại khác báo lỗi rõ.
        Route::Remote => restore_remote_inner(remote, path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_rejects_empty_path() {
        assert!(restore_local("").is_err());
        assert!(restore_remote("GDrive", "").is_err());
    }
}
