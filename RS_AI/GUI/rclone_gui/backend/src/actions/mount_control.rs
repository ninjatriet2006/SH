//! UNIVERSAL S2 mount micro: start/stop/enable/disable qua systemctl.
//! System level đi qua `pkexec systemctl`; chặn luồng (gọi trong `fastlane`).

use crate::actions::mount_editor::{validate_action, validate_service_name};
use std::process::Command;

/// UNIVERSAL: chạy 1 action systemctl đã allowlist lên service đã validate.
pub fn run_action(service_name: &str, is_user: bool, action: &str) -> Result<String, String> {
    validate_service_name(service_name)?;
    validate_action(action)?;
    let mut cmd = if is_user {
        let mut c = Command::new("systemctl");
        c.arg("--user");
        c
    } else {
        // UNIVERSAL: system level cần pkexec để gọi systemctl.
        Command::new("pkexec")
    };
    if !is_user {
        cmd.arg("systemctl");
    }
    cmd.arg(action);
    cmd.arg(service_name);
    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(format!("Lệnh {action} thành công"))
}

/// UNIVERSAL: nạp lại systemd sau khi thêm/xóa unit (editor gọi ké sau ghi/xóa).
pub fn daemon_reload(is_user: bool) {
    if is_user {
        let _ = Command::new("systemctl").arg("--user").arg("daemon-reload").output();
    } else {
        let _ = Command::new("pkexec").arg("systemctl").arg("daemon-reload").output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_name_and_action_before_spawn() {
        assert!(run_action("../evil", true, "start").is_err());
        assert!(run_action("ok-name", true, "daemon-reload").is_err());
    }
}
