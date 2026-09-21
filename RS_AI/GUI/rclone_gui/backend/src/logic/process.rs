//! UNIVERSAL: primitive tiến trình rclone dùng chung (tách từ `transfer.rs`).
//! Giữ 1 nguồn sự thật cho hủy êm: SIGTERM → chờ `GRACE_PERIOD` → SIGKILL,
//! để rclone kịp dọn `.partial`. `transfer.rs` + `jobs.rs` đều re-dùng từ đây,
//! không `child.kill()` thẳng ở nơi khác.

use std::process::Command;

/// UNIVERSAL: thời gian chờ tối đa để rclone tự kết thúc và dọn dẹp sau SIGTERM.
pub(crate) const GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(5);

/// UNIVERSAL: yêu cầu tiến trình kết thúc có trật tự (cho phép dọn dẹp).
pub(crate) fn send_terminate(pid: u32) {
    // An toàn: `kill 0`/`kill 1` nhắm cả process group/init — không bao giờ gửi.
    if pid <= 1 {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        // UNIVERSAL: Windows không có SIGTERM; taskkill không kèm /F gửi WM_CLOSE.
        let _ = Command::new("taskkill").args(["/PID", &pid.to_string()]).output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output();
    }
}

/// UNIVERSAL: kết thúc ngay (phương án cuối, có thể để lại file .partial).
pub(crate) fn send_kill(pid: u32) {
    // An toàn: `kill 0`/`kill 1` nhắm cả process group/init — không bao giờ gửi.
    if pid <= 1 {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill").args(["-KILL", &pid.to_string()]).output();
    }
}

/// UNIVERSAL: hủy êm theo pid — SIGTERM → chờ `GRACE_PERIOD` → SIGKILL nếu còn sống.
///
/// An toàn: pid 0 (và pid nhỏ hệ thống) trong Unix nghĩa là "cả process group" —
/// `kill 0` bắn tín hiệu vào chính tiến trình gọi. Bỏ qua để không tự sát.
pub(crate) fn terminate_gracefully(pid: u32) {
    if pid <= 1 {
        return;
    }
    send_terminate(pid);
    if !wait_until_gone(pid, GRACE_PERIOD) {
        send_kill(pid);
    }
}

/// UNIVERSAL: chờ tiến trình `pid` kết thúc, tối đa `timeout`.
pub(crate) fn wait_until_gone(pid: u32, timeout: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if !process_alive(pid) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    !process_alive(pid)
}

/// UNIVERSAL: kiểm tra tiến trình còn sống thật sự hay không.
///
/// Trên Linux đọc `/proc/<pid>` thay vì `kill -0`: tiến trình đã chết nhưng chưa
/// được reap (zombie) vẫn phản hồi `kill -0`. Đồng thời đối chiếu `comm` để
/// tránh trường hợp PID đã bị hệ điều hành cấp lại cho tiến trình khác.
#[cfg(target_os = "linux")]
pub(crate) fn process_alive(pid: u32) -> bool {
    let comm = match std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
        Ok(c) => c,
        Err(_) => return false, // UNIVERSAL: không còn trong /proc → đã kết thúc
    };
    if comm.trim() != "rclone" {
        return false; // UNIVERSAL: PID đã được cấp lại cho tiến trình khác
    }
    // UNIVERSAL: trạng thái 'Z' = zombie: đã chết, chỉ chờ được reap.
    match std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        Ok(stat) => !stat
            .rsplit(')')
            .next()
            .is_some_and(|rest| rest.split_whitespace().next() == Some("Z")),
        Err(_) => false,
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn process_alive(pid: u32) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn process_alive_false_for_nonexistent_pid() {
        // UNIVERSAL: PID 0 không bao giờ là tiến trình người dùng hợp lệ.
        assert!(!process_alive(0));
        // UNIVERSAL: PID rất lớn, gần như chắc chắn không tồn tại.
        assert!(!process_alive(4_000_000));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn process_alive_false_when_pid_is_not_rclone() {
        // UNIVERSAL: chính tiến trình test đang sống nhưng `comm` không phải
        // "rclone" nên coi như không còn tác vụ rclone (chống PID reuse).
        let me = std::process::id();
        assert!(!process_alive(me));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn wait_until_gone_returns_immediately_for_dead_pid() {
        // UNIVERSAL: pid chết phải trả về ngay, không chờ hết timeout.
        let start = std::time::Instant::now();
        assert!(wait_until_gone(0, std::time::Duration::from_secs(5)));
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn terminate_gracefully_noop_for_dead_pid() {
        // UNIVERSAL: pid không tồn tại → TERM+chờ+KILL đều no-op, không panic.
        terminate_gracefully(0);
    }
}
