/*
[INTEGRITY NOTES]
- Mục đích: Thợ đo dung lượng/kích thước remote (S2) — logic gọi rclone about/size.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::core::rclone_caller;
use serde_json::Value;

/// Dung lượng remote `rclone about --json` (đồng bộ; tầng api bọc `fastlane`).
pub fn about_inner(remote: &str) -> Result<Value, String> {
    let output = rclone_caller::run_cmd(&["about", remote, "--json"])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone about: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// Kích thước remote `rclone size --json` (đồng bộ; tầng api bọc `fastlane`).
pub fn size_inner(remote: &str) -> Result<Value, String> {
    let output = rclone_caller::run_cmd(&["size", remote, "--json"])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone size: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}
