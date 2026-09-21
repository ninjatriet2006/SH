/*
[INTEGRITY NOTES]
- Mục đích: Thợ ĐỤNG remote (S2 ghi) — tạo/sửa/xóa tài khoản rclone.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::core::rclone_caller;
use std::collections::HashMap;

// UNIVERSAL: gom option k=v, bỏ value rỗng như hành vi cũ.
pub fn collect_option_args(options: &HashMap<String, String>, out: &mut Vec<String>) {
    for (k, v) in options {
        if !v.is_empty() {
            out.push(format!("{}={}", k, v));
        }
    }
}

/// Tạo remote mới (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: giữ nguyên thứ tự args `config create` như bản inline cũ.
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

    #[test]
    fn collect_option_args_skips_empty() {
        // UNIVERSAL: kiểm tra thuần, không gọi rclone thật.
        let mut opts = HashMap::new();
        opts.insert("a".to_string(), "1".to_string());
        opts.insert("b".to_string(), String::new());
        let mut out = Vec::new();
        collect_option_args(&opts, &mut out);
        assert_eq!(out, vec!["a=1".to_string()]);
    }
}
