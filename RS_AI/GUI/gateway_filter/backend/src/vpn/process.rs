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

/// Lấy danh sách các vị trí thực tế của AdGuard VPN CLI:
/// 1. Thử `adguardvpn-cli list-locations 100` để lấy danh sách có ping khi đã login.
/// 2. Fallback: `adguardvpn-cli list-locations --bash-completion ""` hoạt động 100% kể cả chưa login.
pub async fn fetch_adguard_locations() -> Vec<super::types::AdguardLocationItem> {
    let mut items = Vec::new();

    // 1. Thử chạy list-locations có ping
    if let Ok(output) = run_tunnel_command_timeout("adguardvpn-cli list-locations 100", 5).await {
        if !output.contains("Please log in") && !output.contains("not logged in") {
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty()
                    || trimmed.starts_with('#')
                    || trimmed.to_lowercase().starts_with("city")
                    || trimmed.to_lowercase().starts_with("location")
                {
                    continue;
                }
                let ping = if let Some(idx) = trimmed.rfind("ms") {
                    trimmed[..idx]
                        .split_whitespace()
                        .last()
                        .and_then(|s| s.parse::<u64>().ok())
                } else {
                    None
                };
                items.push(super::types::AdguardLocationItem {
                    id: trimmed.to_string(),
                    name: trimmed.to_string(),
                    ping_ms: ping,
                });
            }
        }
    }

    // 2. Fallback tuyệt đối với bash-completion
    if items.is_empty() {
        if let Ok(output) = run_tunnel_command_timeout("adguardvpn-cli list-locations --bash-completion \"\"", 5).await {
            let mut isos = Vec::new();
            let mut others = Vec::new();

            for line in output.lines() {
                let l = line.trim();
                if l.is_empty() {
                    continue;
                }
                // Các mã quốc gia ISO 2 chữ cái (US, JP, SG, DE, GB...)
                if l.len() == 2 && l.chars().all(|c| c.is_ascii_uppercase()) {
                    isos.push(l.to_string());
                } else {
                    others.push(l.to_string());
                }
            }

            for iso in isos {
                items.push(super::types::AdguardLocationItem {
                    id: iso.clone(),
                    name: format!("Country [{}]", iso),
                    ping_ms: None,
                });
            }
            for other in others {
                items.push(super::types::AdguardLocationItem {
                    id: other.clone(),
                    name: other,
                    ping_ms: None,
                });
            }
        }
    }

    items
}

/// Chọn ngẫu nhiên 1 location hợp lệ từ danh sách thực tế của AdGuard VPN CLI
pub async fn pick_random_adguard_location() -> Option<String> {
    let list = fetch_adguard_locations().await;
    if list.is_empty() {
        return None;
    }

    // Ưu tiên chọn từ các mã ISO 2 chữ cái (US, JP, SG, DE...) vì kết nối ổn định nhất
    let iso_candidates: Vec<String> = list
        .iter()
        .filter(|i| i.id.len() == 2 && i.id.chars().all(|c| c.is_ascii_uppercase()))
        .map(|i| i.id.clone())
        .collect();

    if !iso_candidates.is_empty() {
        let random_idx = (uuid::Uuid::new_v4().as_u128() as usize) % iso_candidates.len();
        return Some(iso_candidates[random_idx].clone());
    }

    let random_idx = (uuid::Uuid::new_v4().as_u128() as usize) % list.len();
    Some(list[random_idx].id.clone())
}

