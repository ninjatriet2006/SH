//! Editor Instance Launcher & Process Lifecycle Controls.
//!
//! Khởi chạy IDE với `--user-data-dir` cô lập, quản lý PID và dừng tiến trình.

use std::path::Path;
use std::process::{Child, Command};
use super::discovery::find_executable;

pub struct LaunchOptions<'a> {
    pub platform_id: &'a str,
    pub user_data_dir: &'a Path,
    pub custom_binary_path: Option<&'a Path>,
    pub extra_args: &'a [String],
    pub use_new_window: bool,
}

/// Khởi chạy tiến trình IDE gắn liền với Profile cô lập.
pub fn launch_instance(opts: LaunchOptions) -> Result<u32, String> {
    let bin_path = if let Some(custom) = opts.custom_binary_path {
        custom.to_path_buf()
    } else {
        find_executable(opts.platform_id)
            .ok_or_else(|| format!("Cannot find executable binary for platform '{}'. Please specify executable path in settings.", opts.platform_id))?
    };

    let mut cmd = Command::new(&bin_path);

    // Đối với các IDE gốc VS Code / Electron (Cursor, Windsurf, CodeBuddy, Code, Antigravity)
    if opts.platform_id != "zed" {
        if opts.user_data_dir.as_os_str() != "default" {
            cmd.arg("--user-data-dir").arg(opts.user_data_dir);
        }
        if opts.use_new_window {
            cmd.arg("--new-window");
        }
    }

    for arg in opts.extra_args {
        let trimmed = arg.trim();
        if !trimmed.is_empty() {
            cmd.arg(trimmed);
        }
    }

    let child: Child = cmd.spawn().map_err(|e| format!("Failed to spawn {}: {e}", bin_path.display()))?;
    let pid = child.id();

    Ok(pid)
}

/// Dừng một tiến trình IDE theo PID.
pub fn kill_instance(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::process::Command;
        let status = Command::new("kill")
            .arg("-15") // SIGTERM trước
            .arg(pid.to_string())
            .status()
            .map_err(|e| format!("kill command failed: {e}"))?;

        if status.success() {
            return Ok(());
        }

        // Nếu SIGTERM không được thì SIGKILL
        let _ = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .status();

        Ok(())
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status()
            .map_err(|e| format!("taskkill failed: {e}"))?;
        Ok(())
    }
}
