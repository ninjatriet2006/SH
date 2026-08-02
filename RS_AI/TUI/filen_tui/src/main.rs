use filen_tui::app;

use std::env;
use std::io::{self, IsTerminal};
use std::panic;
use std::process::Command;

fn check_terminal_wrapping() {
    if env::var("FILEN_TUI_WRAPPED").is_ok() {
        return;
    }

    if !io::stdout().is_terminal() {
        #[cfg(target_os = "macos")]
        {
            let current_exe = env::current_exe().unwrap();
            let current_exe_str = current_exe.to_str().unwrap();
            let args: Vec<String> = env::args().skip(1).collect();
            let args_str = if args.is_empty() {
                String::new()
            } else {
                format!(" {}", args.join(" "))
            };
            let script = format!(
                "tell application \"Terminal\" to do script \"export FILEN_TUI_WRAPPED=1 && exec '{}'{}\"",
                current_exe_str, args_str
            );
            let status = Command::new("osascript").args(["-e", &script]).spawn();
            if status.is_ok() {
                std::process::exit(0);
            }
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let terminals = [
                "gnome-terminal",
                "konsole",
                "xfce4-terminal",
                "alacritty",
                "kitty",
                "xterm",
            ];
            for term in terminals {
                if which::which(term).is_ok() {
                    let current_exe = env::current_exe().unwrap();
                    let status = match term {
                        "gnome-terminal" => Command::new(term)
                            .env("FILEN_TUI_WRAPPED", "1")
                            .args(["--", current_exe.to_str().unwrap()])
                            .spawn(),
                        _ => Command::new(term)
                            .env("FILEN_TUI_WRAPPED", "1")
                            .args(["-e", current_exe.to_str().unwrap()])
                            .spawn(),
                    };
                    if status.is_ok() {
                        std::process::exit(0);
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let terminals = ["wt", "cmd"];
            for term in terminals {
                if which::which(term).is_ok() {
                    let current_exe = env::current_exe().unwrap();
                    let status = match term {
                        "wt" => Command::new(term)
                            .env("FILEN_TUI_WRAPPED", "1")
                            .args([current_exe.to_str().unwrap()])
                            .spawn(),
                        _ => Command::new(term)
                            .env("FILEN_TUI_WRAPPED", "1")
                            .args(["/c", "start", "", current_exe.to_str().unwrap()])
                            .spawn(),
                    };
                    if status.is_ok() {
                        std::process::exit(0);
                    }
                }
            }
        }

        eprintln!("Cảnh báo: Không phát hiện TTY và không tìm thấy Terminal Emulator tương thích.");
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Kiểm tra Terminal Wrapping
    check_terminal_wrapping();

    // 2. Thiết lập panic hook khôi phục terminal
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        default_panic(info);
    }));

    // 3. Khởi tạo Terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // 4. Khởi chạy App
    let mut app = app::App::new();
    let res = app.run(&mut terminal).await;

    // 5. Khôi phục Terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Ứng dụng kết thúc với lỗi: {:?}", err);
    }

    Ok(())
}
