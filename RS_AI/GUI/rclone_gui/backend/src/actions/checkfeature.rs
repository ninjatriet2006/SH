/*
[INTEGRITY NOTES]
- Mục đích: Thợ kiểm tra năng lực backend (S2) — logic gọi rclone backend/features + năng lực move/copy.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::actions::checkcap::check_cap;
use crate::core::rclone_caller;
use serde_json::{Value, json};

/// Features backend của một remote (đồng bộ; tầng api bọc `fastlane`).
pub fn query_backend_features(remote: &str) -> Result<Value, String> {
    // UNIVERSAL: đuôi ":" báo cho rclone biết đây là một remote.
    let remote_with_colon = format!("{}:", remote);
    let output = rclone_caller::run_cmd(&["backend", "features", &remote_with_colon])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| format!("Lỗi phân tích JSON: {}", e))
}

/// Năng lực move/copy giữa 2 đường dẫn (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: check_cap là não chung (UI hỏi + đường chạy hỏi); đây chỉ dịch
// Cap ra 3 cờ JSON cũ (key giữ nguyên cho frontend/IPC).
pub fn query_transfer_options(src: &str, dst: &str) -> Result<Value, String> {
    let cap = check_cap(src, dst);
    Ok(json!({
        "canMove": cap.support_move,
        "canCopy": cap.support_move || cap.support_copy_and_delete,
        "canCopyDelete": cap.support_copy_and_delete
    }))
}

/// Tên cũ giữ lại cho tương thích (IPC/frontend không đổi).
pub fn transfer_capability_inner(src: &str, dst: &str) -> Result<Value, String> {
    query_transfer_options(src, dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_local_same_remote_allows_move_copy() {
        // UNIVERSAL: cùng Local luôn move-native + copy, không cần hỏi backend.
        let v = transfer_capability_inner("Local::/a", "Local::/b").expect("capability");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(true));
    }

    #[test]
    fn capability_cross_remote_denies_all() {
        // UNIVERSAL: não chung check_cap — DiffCloud khác hãng/tắt cờ thì
        // không move-native lẫn copy-purge (trung chuyển qua local).
        let v = transfer_capability_inner("A::/a", "B::/b").expect("capability");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopyDelete").and_then(|x| x.as_bool()), Some(false));
    }
}
