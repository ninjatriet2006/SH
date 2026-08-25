/*
[INTEGRITY NOTES]
Mục đích: Module backend quản lý tệp cấu hình rclone (rclone.conf).
Trách nhiệm: Lấy đường dẫn config, đọc và ghi nội dung tệp.
Các module tương tác: frontend/bridge/config_api.ts
*/

use std::process::Command;
use std::fs;

/// Lấy đường dẫn tệp rclone.conf
fn get_rclone_config_path() -> Result<String, String> {
    let output = Command::new("rclone")
        .arg("config")
        .arg("file")
        .output()
        .map_err(|e| format!("Lỗi khi chạy lệnh rclone config file: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Output mẫu:
    // Configuration file is stored at:
    // /home/user/.config/rclone/rclone.conf
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() >= 2 {
        let path = lines[1].trim().to_string();
        Ok(path)
    } else {
        Err(format!("Không thể trích xuất đường dẫn từ output: {}", stdout))
    }
}

#[tauri::command]
pub fn get_config_content() -> Result<String, String> {
    let path = get_rclone_config_path()?;
    
    // Đọc nội dung tệp
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Lỗi đọc tệp {}: {}", path, e)),
    }
}

#[tauri::command]
pub fn set_config_content(content: String) -> Result<(), String> {
    let path = get_rclone_config_path()?;
    
    // Ghi đè nội dung tệp
    match fs::write(&path, content) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Lỗi ghi tệp {}: {}", path, e)),
    }
}
