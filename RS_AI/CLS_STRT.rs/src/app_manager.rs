use std::path::PathBuf;
use crate::config::{AppConfig, AppType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppStatus {
    Running,
    Stopped,
}

impl std::fmt::Display for AppStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppStatus::Running => write!(f, "Đang chạy"),
            AppStatus::Stopped => write!(f, "Đang dừng"),
        }
    }
}

fn is_flatpak_running(app_id: &str) -> bool {
    let output = std::process::Command::new("flatpak")
        .args(["ps", "--columns=application"])
        .output();
    
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        stdout.lines().any(|line| line.trim() == app_id)
    } else {
        false
    }
}

fn is_system_running(target: &str) -> bool {
    // Lấy tên file thực thi từ đường dẫn (nếu có)
    let bin_name = PathBuf::from(target)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| target.to_string());

    // Cố gắng tìm bằng pgrep chính xác theo tên tiến trình
    let status = std::process::Command::new("pgrep")
        .arg("-x")
        .arg(&bin_name)
        .status();
    
    if let Ok(s) = status {
        if s.success() {
            return true;
        }
    }

    // Nếu không thấy, thử pgrep -f với chuỗi target đầy đủ
    let status_f = std::process::Command::new("pgrep")
        .arg("-f")
        .arg(target)
        .status();
    
    if let Ok(s) = status_f {
        s.success()
    } else {
        false
    }
}

pub fn check_status(app: &AppConfig) -> AppStatus {
    let is_running = match app.app_type {
        AppType::Flatpak => is_flatpak_running(&app.target),
        AppType::System => is_system_running(&app.target),
    };

    if is_running {
        AppStatus::Running
    } else {
        AppStatus::Stopped
    }
}

pub fn start_app(app: &AppConfig) -> anyhow::Result<()> {
    match app.app_type {
        AppType::Flatpak => {
            let mut cmd = std::process::Command::new("flatpak");
            cmd.arg("run").arg(&app.target);
            
            if let Some(ref start_cmd) = app.start_cmd {
                let args: Vec<&str> = start_cmd.split_whitespace().collect();
                cmd.args(&args);
            } else if app.target == "org.fcitx.Fcitx5" {
                // Giữ nguyên thiết lập mặc định của script cho Fcitx5
                cmd.arg("-rd");
            }
            
            cmd.spawn()?;
        }
        AppType::System => {
            let cmd_str = app.start_cmd.as_ref().unwrap_or(&app.target);
            std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd_str)
                .spawn()?;
        }
    }
    Ok(())
}

pub fn kill_app(app: &AppConfig) -> anyhow::Result<()> {
    match app.app_type {
        AppType::Flatpak => {
            std::process::Command::new("flatpak")
                .arg("kill")
                .arg(&app.target)
                .status()?;
        }
        AppType::System => {
            if let Some(ref kill_cmd) = app.kill_cmd {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(kill_cmd)
                    .status()?;
            } else {
                // Tắt theo tên tiến trình
                let bin_name = PathBuf::from(&app.target)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| app.target.clone());

                std::process::Command::new("pkill")
                    .arg("-x")
                    .arg(&bin_name)
                    .status()?;
            }
        }
    }
    Ok(())
}

pub fn restart_app(app: &AppConfig) -> anyhow::Result<()> {
    let _ = kill_app(app);
    std::thread::sleep(std::time::Duration::from_secs(1));
    start_app(app)?;
    Ok(())
}
