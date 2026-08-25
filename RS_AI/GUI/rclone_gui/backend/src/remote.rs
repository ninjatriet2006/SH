/*
[INTEGRITY NOTES]
Mục đích: Xử lý các logic backend cho tính năng Quản lý Remote.
Trách nhiệm: Gọi lệnh rclone config providers, create, update, delete.
Các module tương tác: lib.rs, frontend (qua Tauri command).
*/

use std::process::Command;
use serde_json::Value;

/// Gọi `rclone config providers` để lấy JSON danh sách các schema cấu hình remote
#[tauri::command]
pub async fn get_providers() -> Result<String, String> {
    let output = Command::new("rclone")
        .arg("config")
        .arg("providers")
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone error: {}", err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(json_str)
}

/// Tạo một remote mới bằng lệnh `rclone config create`
#[tauri::command]
pub async fn create_remote(
    name: String,
    provider: String,
    options: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let mut cmd = Command::new("rclone");
    cmd.arg("config")
        .arg("create")
        .arg(&name)
        .arg(&provider);

    for (k, v) in options {
        if !v.is_empty() {
            cmd.arg(format!("{}={}", k, v));
        }
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone error: {}", err_msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Xóa một remote bằng lệnh `rclone config delete`
#[tauri::command]
pub async fn delete_remote(name: String) -> Result<String, String> {
    let output = Command::new("rclone")
        .arg("config")
        .arg("delete")
        .arg(&name)
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone error: {}", err_msg));
    }

    Ok("Đã xóa remote thành công".to_string())
}

/// Sửa một remote bằng lệnh `rclone config update`
#[tauri::command]
pub async fn update_remote(
    name: String,
    options: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let mut cmd = Command::new("rclone");
    cmd.arg("config")
        .arg("update")
        .arg(&name);

    for (k, v) in options {
        if !v.is_empty() {
            cmd.arg(format!("{}={}", k, v));
        }
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone error: {}", err_msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Lấy các tính năng backend của một remote bằng lệnh `rclone backend features <remote>:`
#[tauri::command]
pub async fn get_backend_features(remote: String) -> Result<Value, String> {
    let remote_with_colon = format!("{}:", remote);
    let output = Command::new("rclone")
        .arg("backend")
        .arg("features")
        .arg(&remote_with_colon)
        .output()
        .map_err(|e| format!("Failed to execute rclone: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("rclone error: {}", err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(parsed)
}
