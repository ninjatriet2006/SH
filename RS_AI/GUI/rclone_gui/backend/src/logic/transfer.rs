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
) -> Result<(), String> {
    
    let args = vec![
        cmd_name.to_string(),
        src.clone(),
        dst.clone(),
        "--transfers=8".to_string(),
        "--checkers=8".to_string(),
        "--use-json-log".to_string(),
        "--stats".to_string(), "0.5s".to_string(),
        "-v".to_string(),
    ];

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

/// Hàm Hủy tiến trình dựa vào task_id.
///
/// Quan trọng: phải gửi SIGTERM (không phải SIGKILL) để rclone kịp chạy handler
/// dọn dẹp của nó. SIGKILL bỏ lại file rác `<tên>.<hash>.partial` trong thư mục
/// đích; SIGTERM thì rclone tự xoá phần đã tải dở.
///
/// Hàm trả về ngay sau khi gửi SIGTERM. Việc chờ và leo thang sang SIGKILL được
/// thực hiện trên thread nền để không chặn async runtime của Tauri.
pub fn cancel_transfer(state: tauri::State<'_, AppState>, task_id: u32) -> Result<(), String> {
    // Mutex bị poison (một thread khác panic khi đang giữ lock) không nên làm
    // mất khả năng hủy tác vụ — vẫn lấy dữ liệu bên trong ra dùng.
    let pid = match state.pids.lock() {
        Ok(mut pids) => pids.remove(&task_id),
        Err(poisoned) => poisoned.into_inner().remove(&task_id),
    };

    let Some(pid) = pid else { return Ok(()) };

    send_terminate(pid);

    // Leo thang sang SIGKILL ở thread nền nếu tiến trình không tự kết thúc.
    std::thread::spawn(move || {
        if !wait_until_gone(pid, GRACE_PERIOD) {
            send_kill(pid);
        }
    });

    Ok(())
}

/// Thời gian chờ tối đa để rclone tự kết thúc và dọn dẹp sau SIGTERM.
const GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(5);

/// Yêu cầu tiến trình kết thúc một cách có trật tự (cho phép dọn dẹp).
fn send_terminate(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        // Windows không có SIGTERM; taskkill không kèm /F sẽ gửi WM_CLOSE.
        let _ = Command::new("taskkill").args(["/PID", &pid.to_string()]).output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output();
    }
}

/// Kết thúc tiến trình ngay lập tức (phương án cuối, có thể để lại file .partial).
fn send_kill(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill").args(["-KILL", &pid.to_string()]).output();
    }
}

/// Chờ tiến trình `pid` kết thúc, tối đa `timeout`. Trả về true nếu đã kết thúc.
fn wait_until_gone(pid: u32, timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if !process_alive(pid) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    !process_alive(pid)
}

/// Kiểm tra tiến trình còn sống thật sự hay không.
///
/// Trên Linux đọc `/proc/<pid>` thay vì `kill -0`: tiến trình đã chết nhưng chưa
/// được reap (zombie) vẫn phản hồi `kill -0`. Đồng thời đối chiếu `comm` để
/// tránh trường hợp PID đã bị hệ điều hành cấp lại cho tiến trình khác.
#[cfg(target_os = "linux")]
fn process_alive(pid: u32) -> bool {
    let comm = match std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
        Ok(c) => c,
        Err(_) => return false, // Không còn trong /proc → đã kết thúc
    };
    if comm.trim() != "rclone" {
        return false; // PID đã được cấp lại cho tiến trình khác
    }
    // Trạng thái 'Z' = zombie: đã chết, chỉ chờ được reap.
    match std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        Ok(stat) => !stat
            .rsplit(')')
            .next()
            .is_some_and(|rest| rest.split_whitespace().next() == Some("Z")),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "linux"))]
fn process_alive(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn process_alive_false_for_nonexistent_pid() {
        // PID 0 không bao giờ là một tiến trình người dùng hợp lệ.
        assert!(!process_alive(0));
        // PID rất lớn, gần như chắc chắn không tồn tại.
        assert!(!process_alive(4_000_000));
    }

    #[test]
    fn process_alive_false_when_pid_is_not_rclone() {
        // Chính tiến trình test này đang sống, nhưng `comm` không phải "rclone"
        // nên phải bị coi là "không còn tác vụ rclone" (chống PID reuse).
        let me = std::process::id();
        assert!(!process_alive(me));
    }

    #[test]
    fn wait_until_gone_returns_immediately_for_dead_pid() {
        let start = std::time::Instant::now();
        assert!(wait_until_gone(0, std::time::Duration::from_secs(5)));
        // Phải trả về ngay, không chờ hết timeout.
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }
}
