/*
[INTEGRITY NOTES]
- Mục đích: Xử lý các logic backend cho tính năng Quản lý Remote (Thêm, Sửa, Xóa cấu hình đám mây).
- Trách nhiệm: Gọi lệnh rclone config providers, create, update, delete để cấu hình các dịch vụ lưu trữ.
- Tương tác: Gọi bởi lib.rs, cung cấp kết quả cho frontend thông qua Tauri command. Dùng `utils` để gọi rclone.
*/

use serde_json::{Value, json};
use crate::core::rclone; // Sử dụng helper từ thư viện dùng chung
use crate::logic::file_ops::parse_remote_path;

// ====================================================================================
// BLOCK: CÁC HÀM XỬ LÝ CẤU HÌNH REMOTE
// ====================================================================================

/// Tên hàm: list_remotes
/// Mô tả: Trả về danh sách tất cả các remote đã cấu hình từ rclone config dump.
#[tauri::command]
pub async fn list_remotes() -> Result<Vec<Value>, String> {
    let output = rclone::run_cmd(&["config", "dump"])?;
    
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    
    let json_str = String::from_utf8_lossy(&output.stdout);
    let dump: Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    
    let mut remotes = Vec::new();
    if let Value::Object(map) = dump {
        for (name, config) in map {
            if let Value::Object(mut config_map) = config {
                config_map.insert("name".to_string(), Value::String(name));
                remotes.push(Value::Object(config_map));
            }
        }
    }
    
    // Sort remotes alphabetically by name for a better UI experience
    remotes.sort_by(|a, b| {
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        name_a.cmp(name_b)
    });
    
    Ok(remotes)
}

/// Tên hàm: get_providers
/// Mô tả: Trả về danh sách tất cả các loại cloud (Google Drive, Dropbox...) được rclone hỗ trợ dưới dạng JSON.
#[tauri::command]
pub async fn get_providers() -> Result<String, String> {
    let output = rclone::run_cmd(&["config", "providers"])?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Tên hàm: create_remote
/// Mô tả: Tạo mới một cấu hình cloud (remote). Nhận vào tên, loại (provider) và các tùy chọn bổ sung (options).
#[tauri::command]
pub async fn create_remote(
    name: String,
    provider: String,
    options: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let mut args = vec!["config", "create", &name, &provider];
    
    // Thu thập và định dạng các tham số cấu hình
    let mut option_args = Vec::new();
    for (k, v) in &options {
        if !v.is_empty() {
            option_args.push(format!("{}={}", k, v));
        }
    }
    
    // Gộp tất cả arg vào mảng (borrow chuỗi)
    for arg in &option_args {
        args.push(arg);
    }

    let output = rclone::run_cmd(&args)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Tên hàm: delete_remote
/// Mô tả: Xóa một cấu hình cloud (remote) khỏi rclone.
#[tauri::command]
pub async fn delete_remote(name: String) -> Result<String, String> {
    let output = rclone::run_cmd(&["config", "delete", &name])?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }

    Ok("Đã xóa remote thành công".to_string())
}

/// Tên hàm: update_remote
/// Mô tả: Cập nhật các thông số của một cấu hình cloud hiện có.
#[tauri::command]
pub async fn update_remote(
    name: String,
    options: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    let mut args = vec!["config", "update", &name];
    
    // Thu thập và định dạng các tham số cấu hình cần sửa
    let mut option_args = Vec::new();
    for (k, v) in &options {
        if !v.is_empty() {
            option_args.push(format!("{}={}", k, v));
        }
    }
    
    for arg in &option_args {
        args.push(arg);
    }

    let output = rclone::run_cmd(&args)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Tên hàm: get_backend_features
/// Mô tả: Kiểm tra xem ổ đĩa cloud này có hỗ trợ tính năng nào (vd: Thùng rác, copy server-side...).
#[tauri::command]
pub async fn get_backend_features(remote: String) -> Result<Value, String> {
    // Đuôi ":" báo cho rclone biết đây là một remote
    let remote_with_colon = format!("{}:", remote);
    let output = rclone::run_cmd(&["backend", "features", &remote_with_colon])?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Lỗi phân tích JSON: {}", e))?;

    Ok(parsed)
}

/// Tên hàm: check_transfer_capability
/// Mô tả: Đánh giá khả năng copy/move giữa 2 đường dẫn thông qua rclone features.
#[tauri::command]
pub async fn check_transfer_capability(src: String, dst: String) -> Result<Value, String> {
    let (src_remote, _) = parse_remote_path(&src);
    let (dst_remote, _) = parse_remote_path(&dst);

    let mut can_move = false;
    let mut can_copy = false;
    let mut can_copy_delete = false;

    if src_remote == dst_remote && src_remote == "Local" {
        can_move = true;
        can_copy = true;
    } else if src_remote == dst_remote && src_remote != "Local" {
        // Hỏi features từ backend
        if let Ok(feats) = get_backend_features(src_remote).await {
            if let Some(features) = feats.get("Features") {
                if let Some(mv) = features.get("Move").and_then(|v| v.as_bool()) {
                    if mv { can_move = true; }
                }
                if let Some(dir_mv) = features.get("DirMove").and_then(|v| v.as_bool()) {
                    if dir_mv { can_move = true; }
                }

                let copy = features.get("Copy").and_then(|v| v.as_bool()).unwrap_or(false);
                if copy { can_copy = true; }
                
                let purge = features.get("Purge").and_then(|v| v.as_bool()).unwrap_or(false);
                can_copy_delete = copy && purge;
            }
        }
    }

    Ok(json!({
        "canMove": can_move,
        "canCopy": can_copy,
        "canCopyDelete": can_copy_delete
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_remotes() {
        let result = list_remotes().await;
        println!("Result: {:?}", result);
    }
}


/// Tên hàm: rclone_about
/// Mô tả: Lấy thông tin dung lượng của một remote (total, used, free, trashed, other).
#[tauri::command]
pub async fn rclone_about(remote: String) -> Result<Value, String> {
    let output = rclone::run_cmd(&["about", &remote, "--json"])?;
    
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone about: {}", err_msg));
    }
    
    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    
    Ok(parsed)
}

/// Tên hàm: rclone_size
/// Mô tả: Lấy thông tin kích thước và số lượng tệp của một remote hoặc thư mục.
#[tauri::command]
pub async fn rclone_size(remote: String) -> Result<Value, String> {
    let output = rclone::run_cmd(&["size", &remote, "--json"])?;
    
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone size: {}", err_msg));
    }
    
    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    
    Ok(parsed)
}
