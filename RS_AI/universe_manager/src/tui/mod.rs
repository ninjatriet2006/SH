pub mod app;
pub mod ui;

use std::io::{self, Write};
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use app::{App, EnterResult};
use crate::manager;

pub fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Setup panic hook to restore terminal if app crashes
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let res = run_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Lỗi TUI: {:?}", err);
    }

    Ok(())
}

fn run_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut draw_needed = true;
    let mut last_draw = std::time::Instant::now();
    loop {
        let now = std::time::Instant::now();
        if draw_needed || now.duration_since(last_draw).as_secs() >= 2 {
            terminal.draw(|f| ui::draw(f, app))?;
            last_draw = now;
            draw_needed = false;
        }

        // Wait for event up to 100ms
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    // Ignore release events (Unix/Windows compatibility)
                    if key.kind == event::KeyEventKind::Release {
                        continue;
                    }
                    
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.current_screen == app::Screen::MainMenu {
                                break;
                            } else {
                                app.handle_back();
                                draw_needed = true;
                            }
                        }
                        KeyCode::Backspace => {
                            app.handle_back();
                            draw_needed = true;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.previous();
                            draw_needed = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.next();
                            draw_needed = true;
                        }
                        KeyCode::Char(' ') => {
                            app.toggle_checked();
                            draw_needed = true;
                        }
                        KeyCode::Delete => {
                            if app.current_screen == app::Screen::AutostartManager {
                                app.delete_selected_autostart();
                                draw_needed = true;
                            }
                        }
                        KeyCode::Insert => {
                            match app.current_screen {
                                app::Screen::AppList => {
                                    app.go_to_operations();
                                    draw_needed = true;
                                }
                                app::Screen::AppOperations => {
                                    app.execute_operations();
                                    draw_needed = true;
                                }
                                app::Screen::UninstallList => {
                                    let _ = run_uninstalls_outside_raw(terminal, app);
                                    draw_needed = true;
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Enter => {
                            match app.handle_enter() {
                                EnterResult::RunWizard => {
                                    let _ = run_wizard_outside_raw(terminal, app);
                                    draw_needed = true;
                                }
                                EnterResult::RunCheckUpdates => {
                                    let _ = run_check_updates_outside_raw(terminal, app);
                                    draw_needed = true;
                                }
                                EnterResult::RunCleanLeftovers => {
                                    let _ = run_clean_leftovers_outside_raw(terminal, app);
                                    draw_needed = true;
                                }
                                EnterResult::RunUninstalls => {
                                    let _ = run_uninstalls_outside_raw(terminal, app);
                                    draw_needed = true;
                                }
                                EnterResult::Exit => {
                                    break;
                                }
                                EnterResult::None => {
                                    draw_needed = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    draw_needed = true;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn run_uninstalls_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    
    println!("\n=== Bắt đầu Gỡ cài đặt Ứng dụng ===");
    app.execute_uninstalls();
    
    println!("\nNhấn Enter để tiếp tục...");
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    app.reload_apps();
    Ok(())
}

fn run_wizard_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    
    // Clear screen
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    let _ = io::stdout().flush();
    
    println!("=== Bắt đầu Tích hợp Ứng dụng Portable Mới ===\n");
    
    let result = crate::tui::app::run_integration_wizard_inline();
    
    match result {
        Ok(true) => {
            println!("\nTích hợp ứng dụng thành công!");
            println!("Nhấn Enter để quay lại...");
            let mut buf = String::new();
            let _ = io::stdin().read_line(&mut buf);
        }
        Ok(false) => {
            // Cancelled. Immediately return to TUI without waiting.
        }
        Err(e) => {
            println!("\nLỗi tích hợp: {}", e);
            println!("Nhấn Enter để quay lại...");
            let mut buf = String::new();
            let _ = io::stdin().read_line(&mut buf);
        }
    }

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    app.reload_apps();
    Ok(())
}

fn run_check_updates_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    
    println!("\n=== Đang kiểm tra cập nhật hệ thống ===");
    match manager::check_system_updates() {
        Ok(res) => println!("\n{}", res),
        Err(e) => println!("\nLỗi: {}", e),
    }
    println!("Nhấn Enter để quay lại...");
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    app.reload_apps();
    Ok(())
}

fn run_clean_leftovers_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    
    println!("\n=== Đang dọn dẹp leftovers ===");
    match manager::clean_system_leftovers() {
        Ok(res) => println!("\n{}", res),
        Err(e) => println!("\nLỗi: {}", e),
    }
    println!("Nhấn Enter để quay lại...");
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;
    app.reload_apps();
    Ok(())
}
