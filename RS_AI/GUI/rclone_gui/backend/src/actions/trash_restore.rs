/*
[INTEGRITY NOTES]
- Mục đích: Khôi phục mục từ thùng rác Local (`gio`) + Remote (`drive untrash`) (S1 unify).
- Trách nhiệm: Phân tuyến (Route) → chọn nhánh Local/Remote bằng `match` + UNIVERSAL.
- Tương tác: Tầng `api::trash_manager` bọc mỏng qua `logic::fastlane::fastlane`. Không đụng IPC/frontend.
  Backend nào có `untrash` thì rclone tự làm, không gate bảng tay.
*/

use super::trash_list::Route;
use super::trash_list::percent_encode;
use crate::core::rclone_caller;
use serde_json::Value;

/// Khôi phục một mục local về vị trí gốc. Dùng `gio trash --restore` vì nó tự
/// tạo lại thư mục cha nếu đã bị xoá, và không ghi đè file đang tồn tại.
/// (đồng bộ; tầng api bọc `blocking`).
pub fn restore_local(id: &str) -> Result<(), String> {
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

/// Khôi phục một mục remote khỏi thùng rác (`backend untrash`).
/// UNIVERSAL không-bịa: KHÔNG gate bằng bảng tay — cứ gọi, backend không có
/// `untrash` thì rclone tự báo lỗi, ta bê nguyên về (mẫu attempt-based của delete).
/// (đồng bộ; tầng api bọc `blocking`).
pub fn restore_remote(remote: &str, path: &str) -> Result<(), String> {
    if Route::classify(remote) == Route::Local {
        return Err("Tuyến Local phải dùng `restore_local`.".to_string());
    }
    if path.is_empty() {
        return Err("Thiếu đường dẫn mục cần khôi phục.".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_rejects_empty_path() {
        assert!(restore_local("").is_err());
        assert!(restore_remote("GDrive", "").is_err());
    }
}
