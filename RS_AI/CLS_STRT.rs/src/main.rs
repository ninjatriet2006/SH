mod config;
mod app_manager;
mod ui;

use std::env;
use std::io::IsTerminal;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    // 1. Kiểm tra xem ứng dụng có đang được chạy trong cửa sổ Terminal thực sự hay không.
    // Nếu chạy từ giao diện đồ họa (GUI), ta sẽ thử tìm và tự mở một terminal tương thích.
    if !std::io::stdout().is_terminal() {
        if let Ok(exe) = env::current_exe() {
            #[cfg(target_os = "linux")]
            {
                let terminals = ["gnome-terminal", "konsole", "xfce4-terminal", "x-terminal-emulator", "xterm"];
                for term in terminals {
                    // Thử khởi chạy terminal mới chạy chính file binary này
                    if Command::new(term).arg("--").arg(&exe).spawn().is_ok() || 
                       Command::new(term).arg("-e").arg(&exe).spawn().is_ok() {
                        return Ok(());
                    }
                }
            }
        }
    }

    // 2. Bắt đầu vòng lặp CLI tương tác
    ui::start_ui_loop()?;

    Ok(())
}
