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
                    // Nhiều CLI (vd adguardvpn-cli) in lý do lỗi ra STDOUT rồi exit != 0,
                    // stderr để trống — chỉ lấy stderr thì error log trống rỗng ("exit status: 11: ").
                    // Lấy cả 2 (cắt 1000 chars đầu mỗi bên để log không phình).
                    fn head(s: &str) -> String {
                        s.trim().chars().take(1000).collect()
                    }
                    let out = head(&String::from_utf8_lossy(&out_buf));
                    let err = head(&String::from_utf8_lossy(&err_buf));
                    let mut msg = if err.is_empty() {
                        format!("Command failed with status {}: {}", status, out)
                    } else if out.is_empty() {
                        format!("Command failed with status {}: {}", status, err)
                    } else {
                        format!(
                            "Command failed with status {}: {}\n[stdout] {}",
                            status, err, out
                        )
                    };
                    // Hint khi CLI đòi login (adguard exit 11 + "Please log in...").
                    let combined = format!("{} {}", out, err).to_lowercase();
                    if combined.contains("not logged in")
                        || combined.contains("log in to")
                        || combined.contains("please log in")
                    {
                        msg.push_str(
                            " → Chưa đăng nhập VPN CLI: chạy lệnh `login` của nó trong terminal trước, rồi Test lại.",
                        );
                    }
                    Err(msg)
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
