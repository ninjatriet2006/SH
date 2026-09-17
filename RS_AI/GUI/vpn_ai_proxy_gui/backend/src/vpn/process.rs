/// Chạy lệnh CLI hệ thống với timeout — chống deadlock khi tiến trình con bị kẹt
/// (VD: VPN prompt hỏi Y/N làm treo thread vô thời hạn).
/// Hết timeout sẽ kill tiến trình con để tránh orphan process giữ cổng.
pub async fn run_tunnel_command_timeout(cmd_str: &str, timeout_secs: u64) -> Result<String, String> {
    let trimmed = cmd_str.trim();
    if trimmed.is_empty() {
        return Err("Command is empty".to_string());
    }

    #[cfg(target_os = "windows")]
    let mut child = tokio::process::Command::new("cmd")
        .args(["/C", trimmed])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let mut child = tokio::process::Command::new("sh")
        .args(["-c", trimmed])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Tách pipe ra trước để đọc sau khi tiến trình thoát (tránh move child vào wait_with_output)
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();

    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait(),
    )
    .await
    {
        Ok(Ok(status)) => {
            // Tiến trình đã thoát — đọc nốt stdout/stderr còn lại trong pipe
            use tokio::io::AsyncReadExt;
            let mut out_buf = Vec::new();
            let mut err_buf = Vec::new();
            if let Some(mut out) = stdout.take() {
                let _ = out.read_to_end(&mut out_buf).await;
            }
            if let Some(mut err_pipe) = stderr.take() {
                let _ = err_pipe.read_to_end(&mut err_buf).await;
            }
            if status.success() {
                Ok(String::from_utf8_lossy(&out_buf).to_string())
            } else {
                Err(format!(
                    "Command failed with status {}: {}",
                    status,
                    String::from_utf8_lossy(&err_buf)
                ))
            }
        }
        Ok(Err(e)) => Err(format!("Failed to wait for command: {}", e)),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(format!(
                "Command timed out after {}s and was killed: {}",
                timeout_secs, trimmed
            ))
        }
    }
}
