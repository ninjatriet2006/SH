/*
[INTEGRITY NOTES]
- Mục đích: Điểm vào GUI BUILDER — TUI chọn và build project trong workspace RS_AI.
- Trách nhiệm: Bọc terminal emulator khi thiếu TTY (mở từ file manager), bật/tắt
  raw mode an toàn, cài panic hook, chạy event loop.
- Tương tác: `discovery` (tìm project), `app` (trạng thái), `ui` (vẽ), `builder` (build).

Quy chuẩn terminal đã áp dụng:
  * Panic hook khôi phục terminal — nếu không, crash sẽ để terminal kẹt raw mode.
  * Tự bọc terminal emulator khi stdout không phải TTY, kèm biến môi trường chống
    lặp vô hạn.
*/

mod app;
mod builder;
mod discovery;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use app::{App, Screen};

/// Biến môi trường chống bọc terminal lặp vô hạn.
const WRAPPED_ENV: &str = "GUI_BUILDER_WRAPPED";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Mở từ trình quản lý file → không có TTY → tự mở trong terminal emulator.
    if wrap_in_terminal_if_needed() {
        return Ok(());
    }

    let root = match discovery::workspace_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Không xác định được workspace root: {e}");
            std::process::exit(1);
        }
    };

    let projects = match discovery::discover(&root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Không quét được danh sách project: {e}");
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Panic hook: khôi phục terminal trước khi in lỗi.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut app = App::new(root, projects);
    let res = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // In tổng kết ra terminal thật để người dùng còn thấy sau khi thoát.
    if !app.results.is_empty() {
        println!("── Kết quả build ──");
        for r in &app.results {
            println!("{} {}", if r.ok { "✔" } else { "✖" }, r.message);
        }
    }

    res.map_err(Into::into)
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll ngắn để vừa nhận phím vừa cập nhật tiến trình build mượt.
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Bỏ qua sự kiện nhả phím trên Windows/một số terminal.
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Ctrl+C luôn thoát.
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c'))
                {
                    break;
                }

                match app.screen {
                    Screen::Select => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up => app.move_cursor(-1),
                        KeyCode::Down => app.move_cursor(1),
                        KeyCode::Char(' ') => app.toggle_current(),
                        KeyCode::Char('a') => app.select_all(),
                        KeyCode::Enter => app.start_build(),
                        _ => {}
                    },
                    Screen::Building => match key.code {
                        // Đang build thì chặn thoát mềm để không bỏ dở giữa chừng.
                        KeyCode::Char('q') if !app.is_running() => break,
                        KeyCode::Esc => app.back_to_select(),
                        _ => {}
                    },
                }
            }
        }

        app.drain_events();

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// Nếu stdout không phải TTY, mở lại chính binary này trong một terminal emulator.
/// Trả `true` nếu đã spawn thành công (caller nên thoát ngay).
fn wrap_in_terminal_if_needed() -> bool {
    use std::io::IsTerminal;

    if io::stdout().is_terminal() || std::env::var(WRAPPED_ENV).is_ok() {
        return false;
    }

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };

    #[cfg(target_os = "linux")]
    {
        // `--` dùng cho gnome-terminal/x-terminal-emulator, `-e` cho xterm và tương tự.
        let terminals = [
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "x-terminal-emulator",
            "alacritty",
            "kitty",
            "xterm",
        ];
        for term in terminals {
            let spawned = std::process::Command::new(term)
                .arg("--")
                .arg(&exe)
                .env(WRAPPED_ENV, "1")
                .spawn()
                .is_ok()
                || std::process::Command::new(term)
                    .arg("-e")
                    .arg(&exe)
                    .env(WRAPPED_ENV, "1")
                    .spawn()
                    .is_ok();
            if spawned {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    /// Kiểm chứng luồng build + xuất release thật cho một project Cargo nhẹ,
    /// không cần TTY. Bỏ qua nếu không tìm được workspace.
    #[test]
    #[ignore = "chạy build thật, tốn thời gian — dùng `cargo test -- --ignored`"]
    fn build_pipeline_exports_binary() {
        use crate::builder::{BuildEvent, build_project};
        use crate::discovery;
        use std::sync::mpsc::channel;

        let Ok(root) = discovery::workspace_root() else { return };
        let Ok(projects) = discovery::discover(&root) else { return };
        let Some(project) = projects.iter().find(|p| p.package == "img_splt") else {
            return;
        };

        let (tx, rx) = channel();
        let p = project.clone();
        let r = root.clone();
        std::thread::spawn(move || build_project(r, p, tx));

        let mut saw_progress = false;
        let mut finished_ok = None;
        for ev in rx {
            match ev {
                BuildEvent::Progress { done, total, .. } => {
                    assert!(total > 0, "total phải > 0 để vẽ được thanh tiến trình");
                    assert!(done <= total || total == 0);
                    saw_progress = true;
                }
                BuildEvent::Finished { ok, .. } => {
                    finished_ok = Some(ok);
                    break;
                }
                _ => {}
            }
        }

        assert_eq!(finished_ok, Some(true), "build phải thành công");
        assert!(saw_progress, "phải nhận được sự kiện tiến trình từ cargo");
        let exported = root.join("release").join(&project.bin_name).join(&project.bin_name);
        assert!(exported.is_file(), "binary phải được xuất vào release/");
    }
}
