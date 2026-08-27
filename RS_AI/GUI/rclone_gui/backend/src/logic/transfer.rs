/*
[INTEGRITY NOTES]
- Mục đích: Quản lý các tiến trình truyền tải dữ liệu ngầm (Copy/Move).
- Trách nhiệm: Chạy lệnh rclone với pipe log JSON, bóc tách tiến độ (progress), phát sự kiện lên Frontend, và quản lý PID để Hủy (Cancel).
- Tương tác: Gọi `core::rclone::build_target`, gọi `app_state.rs`.
*/

use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use tauri::Emitter;
use crate::logic::app_state::AppState;


/// Hàm tiện ích chạy tiến trình copy/move và báo cáo tiến độ về frontend
pub async fn run_transfer_task(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    cmd_name: &str, // "copyto" hoặc "moveto"
    src: String,
    dst: String,
    task_id: Option<u32>,
    excludes: Option<Vec<String>>,
) -> Result<(), String> {
    
    let mut args = vec![
        cmd_name.to_string(),
        src.clone(),
        dst.clone(),
        "--transfers=8".to_string(),
        "--checkers=8".to_string(),
        "--use-json-log".to_string(),
        "--stats".to_string(), "0.5s".to_string(),
        "-v".to_string(),
    ];

    if let Some(excludes_list) = excludes {
        for excl in excludes_list {
            args.push("--exclude".to_string());
            args.push(excl);
        }
    }

    let mut child = Command::new("rclone")
        .args(args)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Lỗi khi khởi chạy tiến trình rclone: {}", e))?;

    let pid = child.id();
    if let Some(id) = task_id {
        if let Ok(mut pids) = state.pids.lock() {
            pids.insert(id, pid);
        }
    }

    let status = tauri::async_runtime::spawn_blocking(move || {
        let mut error_msgs = Vec::new();
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line_str) = line {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line_str) {
                        if let Some(stats) = json.get("stats") {
                            if let Some(id) = task_id {
                                let payload = serde_json::json!({
                                    "id": id,
                                    "stats": stats
                                });
                                let _ = app_handle.emit("transfer_progress", payload);
                            }
                        }
                        if let Some(level) = json.get("level").and_then(|v| v.as_str()) {
                            if level == "error" {
                                if let Some(msg) = json.get("msg").and_then(|v| v.as_str()) {
                                    error_msgs.push(msg.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        let exit_status = child.wait()?;
        Ok::<_, std::io::Error>((exit_status, error_msgs))
    }).await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("Lỗi khi đợi tiến trình rclone kết thúc: {}", e))?;
    
    let (status, error_msgs) = status;
    
    if let Some(id) = task_id {
        if let Ok(mut pids) = state.pids.lock() {
            pids.remove(&id);
        }
    }

    if !status.success() {
        let err_str = if error_msgs.is_empty() {
            format!("Lệnh {} thất bại với mã lỗi: {}", cmd_name, status)
        } else {
            error_msgs.join("\n")
        };
        return Err(err_str);
    }
    Ok(())
}

/// Hàm Hủy tiến trình dựa vào task_id
pub fn cancel_transfer(state: tauri::State<'_, AppState>, task_id: u32) -> Result<(), String> {
    if let Ok(mut pids) = state.pids.lock() {
        if let Some(pid) = pids.remove(&task_id) {
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
            }
        }
    }
    Ok(())
}
