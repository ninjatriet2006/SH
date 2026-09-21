/*
[INTEGRITY NOTES]
- Mục đích: Thợ remote (S2) — logic gọi rclone config (tài khoản).
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::core::rclone_caller;
use serde_json::Value;
use std::collections::HashMap;

// Tương thích: giữ đường dẫn cũ `actions::remotes::{backend_features_inner,
// transfer_capability_inner, about_inner, size_inner}` cho code ngoài còn dùng.
pub use super::checkfeature::{backend_features_inner, transfer_capability_inner};
pub use super::checksize::{about_inner, size_inner};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_remotes_inner() {
        let result = list_remotes_inner();
        println!("Result: {:?}", result);
    }
}
