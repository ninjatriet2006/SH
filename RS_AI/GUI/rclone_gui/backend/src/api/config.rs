/*
[INTEGRITY NOTES]
Mục đích: Module backend quản lý tệp cấu hình rclone (rclone.conf).
Trách nhiệm: Lấy đường dẫn config, đọc và ghi nội dung tệp.
Các module tương tác: frontend/bridge/config_api.ts
*/

use crate::core::task::blocking;
use std::fs;
use std::process::Command;

/// Lấy đường dẫn tệp rclone.conf
fn get_rclone_config_path() -> Result<String, String> {
    let output = Command::new("rclone")
        .arg("config")
        .arg("file")
        .output()
        .map_err(|e| format!("Lỗi khi chạy lệnh rclone config file: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Lệnh `rclone config file` thất bại ({}): {}",
            output.status, err
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Output mẫu:
    // Configuration file is stored at:
    // /home/user/.config/rclone/rclone.conf
    let path = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .last()
        .unwrap_or("");

    if path.is_empty() {
        Err(format!("Không thể trích xuất đường dẫn từ output: {}", stdout))
    } else {
        Ok(path.to_string())
    }
}

/// Đọc nội dung rclone.conf (đồng bộ, dùng nội bộ).
fn read_config() -> Result<String, String> {
    let path = get_rclone_config_path()?;

    // Đọc nội dung tệp
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Lỗi đọc tệp {}: {}", path, e)),
    }
}

/// Ghi đè nội dung rclone.conf (đồng bộ, dùng nội bộ).
fn write_config(content: String) -> Result<(), String> {
    let path = get_rclone_config_path()?;

    match fs::write(&path, content) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Lỗi ghi tệp {}: {}", path, e)),
    }
}

#[tauri::command]
pub async fn get_config_content() -> Result<String, String> {
    blocking(read_config).await
}

#[tauri::command]
pub async fn set_config_content(content: String) -> Result<(), String> {
    blocking(move || write_config(content)).await
}

#[tauri::command]
pub async fn reorder_config(names: Vec<String>) -> Result<(), String> {
    blocking(move || {
        let content = read_config()?;

        let mut sections = Vec::new();
        let mut current_name: Option<String> = None;
        let mut current_lines = Vec::new();

        for line in content.lines() {
            if line.trim().starts_with('[') && line.trim().ends_with(']') {
                sections.push((current_name.clone(), current_lines.clone()));
                let name = line.trim();
                current_name = Some(name[1..name.len() - 1].trim().to_string());
                current_lines = vec![line.to_string()];
            } else {
                current_lines.push(line.to_string());
            }
        }
        sections.push((current_name, current_lines));

        let mut ordered = Vec::new();

        if let Some(pos) = sections.iter().position(|s| s.0.is_none()) {
            ordered.push(sections.remove(pos));
        }

        for name in names {
            if let Some(pos) = sections.iter().position(|s| s.0.as_deref() == Some(name.as_str())) {
                ordered.push(sections.remove(pos));
            }
        }

        ordered.extend(sections);

        let mut new_content = String::new();
        for (_, lines) in ordered {
            new_content.push_str(&lines.join("\n"));
            new_content.push('\n');
        }

        write_config(new_content)
    })
    .await
}
