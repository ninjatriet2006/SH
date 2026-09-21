/*
[INTEGRITY NOTES]
- Mục đích: Thợ kiểm tra năng lực backend (S2) — logic gọi rclone backend/features + năng lực move/copy.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::actions::move_op::{Cap, SupportCopyAndDelete, SupportMove};
use crate::core::path::cut_remote_path;
use crate::core::rclone_caller;
use serde_json::{Value, json};

/// Features backend của một remote (đồng bộ; tầng api bọc `fastlane`).
pub fn backend_features_inner(remote: &str) -> Result<Value, String> {
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
// UNIVERSAL: move-native 1 bước (`Move`/`DirMove`) khác với copy + purge
// fallback 2 bước (`Copy` + `Purge`); tên nội bộ mới, key JSON giữ nguyên.
pub fn transfer_capability_inner(src: &str, dst: &str) -> Result<Value, String> {
    let (src_remote, _) = cut_remote_path(src);
    let (dst_remote, _) = cut_remote_path(dst);
    let mut support_move = SupportMove(false);
    let mut can_copy = false;
    let mut support_copy_and_delete = SupportCopyAndDelete(false);

    if src_remote == dst_remote && src_remote == "Local" {
        support_move = SupportMove(true);
        can_copy = true;
    } else if src_remote == dst_remote && src_remote != "Local" {
        if let Ok(feats) = backend_features_inner(&src_remote) {
            if let Some(features) = feats.get("Features") {
                let mv = features.get("Move").and_then(|v| v.as_bool()).unwrap_or(false);
                let dir_mv = features.get("DirMove").and_then(|v| v.as_bool()).unwrap_or(false);
                let copy = features.get("Copy").and_then(|v| v.as_bool()).unwrap_or(false);
                let purge = features.get("Purge").and_then(|v| v.as_bool()).unwrap_or(false);
                let cap = Cap::from_backend_features(mv, dir_mv, copy, purge);
                support_move = SupportMove(cap.support_move);
                can_copy = copy;
                support_copy_and_delete = SupportCopyAndDelete(cap.support_copy_and_delete);
            }
        }
    }

    Ok(json!({
        "canMove": support_move.0,
        "canCopy": can_copy,
        "canCopyDelete": support_copy_and_delete.0
    }))
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
        // UNIVERSAL: khác remote/provider thì không move-native lẫn copy-purge.
        let v = transfer_capability_inner("Local::/a", "GDrive::/b").expect("capability");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopyDelete").and_then(|x| x.as_bool()), Some(false));
    }
}
