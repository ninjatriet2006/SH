/*
[INTEGRITY NOTES]
- Mục đích: Thợ remote (S2) — logic gọi rclone config/backend/about/size + năng lực move/copy.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::actions::move_op::{Cap, SupportCopyAndDelete, SupportMove};
use crate::core::path::cut_remote_path;
use crate::core::rclone_caller;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Dump thô `rclone config dump` (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: việc hỏi-nhanh config đi thẳng rclone, không qua jobs.
pub fn config_dump() -> Result<Value, String> {
    let output = rclone_caller::run_cmd(&["config", "dump"])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// Danh sách remote đã sort theo tên (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: sort ở thợ để api chỉ còn wrapper 1 dòng.
pub fn list_remotes_inner() -> Result<Vec<Value>, String> {
    let dump = config_dump()?;
    let mut remotes = Vec::new();
    if let Value::Object(map) = dump {
        for (name, config) in map {
            if let Value::Object(mut config_map) = config {
                config_map.insert("name".to_string(), Value::String(name));
                remotes.push(Value::Object(config_map));
            }
        }
    }
    remotes.sort_by(|a, b| {
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        name_a.cmp(name_b)
    });
    Ok(remotes)
}

/// Danh sách provider rclone hỗ trợ (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: việc hỏi-nhanh, trả thô stdout để frontend parse.
pub fn get_providers_inner() -> Result<String, String> {
    let output = rclone_caller::run_cmd(&["config", "providers"])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn collect_option_args(options: &HashMap<String, String>, out: &mut Vec<String>) {
    for (k, v) in options {
        if !v.is_empty() {
            out.push(format!("{}={}", k, v));
        }
    }
}

/// Tạo remote mới (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: bỏ option rỗng như hành vi cũ, giữ nguyên thứ tự args `config create`.
pub fn create_remote_inner(
    name: &str,
    provider: &str,
    options: &HashMap<String, String>,
) -> Result<String, String> {
    let mut args = vec!["config", "create", name, provider];
    let mut option_args = Vec::new();
    collect_option_args(options, &mut option_args);
    for arg in &option_args {
        args.push(arg);
    }
    let output = rclone_caller::run_cmd(&args)?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Xóa remote (đồng bộ; tầng api bọc `fastlane`).
pub fn delete_remote_inner(name: &str) -> Result<String, String> {
    let output = rclone_caller::run_cmd(&["config", "delete", name])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    Ok("Đã xóa remote thành công".to_string())
}

/// Cập nhật remote (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: giữ nguyên cách dựng args `config update` như bản inline cũ.
pub fn update_remote_inner(name: &str, options: &HashMap<String, String>) -> Result<String, String> {
    let mut args = vec!["config", "update", name];
    let mut option_args = Vec::new();
    collect_option_args(options, &mut option_args);
    for arg in &option_args {
        args.push(arg);
    }
    let output = rclone_caller::run_cmd(&args)?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_remotes_inner() {
        let result = list_remotes_inner();
        println!("Result: {:?}", result);
    }

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
