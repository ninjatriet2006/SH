pub mod app;
pub mod ui;

use app::{App, EnterResult};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::{self, Write};
use std::time::Duration;

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
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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

        // If the app needs an initial/forced scan, do it NOW after drawing the loading screen
        if app.needs_initial_scan {
            app.reload_apps();
            app.needs_initial_scan = false;
            draw_needed = true;
            continue; // redraw immediately without waiting for events
        }

        if app.needs_update_scan {
            if let Ok(updates) = crate::maintenance::check_system_updates() {
                app.update_entries = updates;
            } else {
                app.update_entries = Vec::new();
            }
            app.needs_update_scan = false;
            draw_needed = true;
            continue;
        }

        // Wait for event up to 100ms
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    // Ignore release events (Unix/Windows compatibility)
                    if key.kind == event::KeyEventKind::Release {
                        continue;
                    }

                    if app.is_searching {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                app.is_searching = false;
                                draw_needed = true;
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.update_filter();
                                draw_needed = true;
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.update_filter();
                                draw_needed = true;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if app.is_install_searching {
                        match key.code {
                            KeyCode::Esc => {
                                app.is_install_searching = false;
                                draw_needed = true;
                            }
                            KeyCode::Enter => {
                                app.is_install_searching = false;
                                app.status_message = Some("Đang tìm kiếm phần mềm...".to_string());
                                terminal.draw(|f| ui::draw(f, app))?;

                                app.install_search_results = crate::installer::search_apps(&app.install_search_query);
                                if app.install_search_results.is_empty() {
                                    app.status_message = Some("Không tìm thấy kết quả nào!".to_string());
                                } else {
                                    app.status_message =
                                        Some(format!("Tìm thấy {} kết quả.", app.install_search_results.len()));
                                }
                                app.install_selected_index = 0;
                                draw_needed = true;
                            }
                            KeyCode::Backspace => {
                                app.install_search_query.pop();
                                draw_needed = true;
                            }
                            KeyCode::Char(c) => {
                                app.install_search_query.push(c);
                                draw_needed = true;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('/') | KeyCode::Char('f')
                            if app.current_screen == crate::tui::app::Screen::AppList =>
                        {
                            app.is_searching = true;
                            draw_needed = true;
                        }
                        KeyCode::Char('/') if app.current_screen == crate::tui::app::Screen::AppInstaller => {
                            app.is_install_searching = true;
                            draw_needed = true;
                        }
                        KeyCode::Char('r') if key.modifiers.contains(event::KeyModifiers::ALT) => {
                            app.needs_initial_scan = true;
                            draw_needed = true;
                        }
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
                        KeyCode::Char('i') | KeyCode::Insert => match app.current_screen {
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
                            app::Screen::UpdateManager => {
                                let _ = run_update_selected_outside_raw(terminal, app);
                                draw_needed = true;
                            }
                            _ => {}
                        },
                        KeyCode::Enter => {
                            if app.current_screen == app::Screen::UpdateManager {
                                let _ = run_update_selected_outside_raw(terminal, app);
                                draw_needed = true;
                                continue;
                            }
                            if app.current_screen == app::Screen::PackageManagerInstaller {
                                let _ = run_pm_install_outside_raw(terminal, app);
                                draw_needed = true;
                                continue;
                            }
                            if app.current_screen == app::Screen::AppInstaller {
                                let _ = run_app_install_outside_raw(terminal, app);
                                draw_needed = true;
                                continue;
                            }
                            match app.handle_enter() {
                                EnterResult::RunWizard => {
                                    let _ = run_wizard_outside_raw(terminal, app);
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

fn run_update_selected_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    if app.checked_updates.is_empty() {
        return Ok(());
    }
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    println!("\n=== Bắt đầu Cập nhật Phần mềm ===");
    let mut selected_entries = Vec::new();
    for &idx in &app.checked_updates {
        if let Some(entry) = app.update_entries.get(idx) {
            selected_entries.push(entry);
        }
    }
    match crate::maintenance::execute_updates(selected_entries) {
        Ok(res) => println!("{}", res),
        Err(e) => println!("Lỗi cập nhật: {}", e),
    }

    println!("\nNhấn Enter để tiếp tục...");
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;

    // Rescan after update
    app.needs_update_scan = true;
    app.checked_updates.clear();

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

fn run_clean_leftovers_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    println!("\n=== Đang dọn dẹp leftovers ===");
    match crate::maintenance::clean_system_leftovers() {
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

fn run_pm_install_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    if app.checked_pms.is_empty() {
        return Ok(());
    }
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    println!("\n=== Bắt đầu Cài đặt Công cụ Nền tảng ===");
    for &idx in &app.checked_pms {
        if let Some((name, _)) = app.pm_entries.get(idx) {
            println!("Đang cài đặt {}...", name);
            match crate::installer::install_package_manager(name) {
                Ok(msg) => println!("{}", msg),
                Err(e) => println!("Lỗi: {}", e),
            }
        }
    }

    println!("\nNhấn Enter để tiếp tục...");
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.clear()?;

    // Rescan after update
    app.pm_entries = crate::installer::check_package_managers();
    app.checked_pms.clear();

    Ok(())
}

fn run_app_install_outside_raw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    if app.install_search_results.is_empty() {
        return Ok(());
    }
    let selected = app.install_search_results.get(app.install_selected_index).cloned();
    if let Some(entry) = selected {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        println!("\n=== Bắt đầu Cài đặt Phần mềm ===");
        println!("Đang cài đặt {} (ID: {}) qua {}...", entry.name, entry.id, entry.source);
        match crate::installer::install_app(&entry.id, &entry.source) {
            Ok(msg) => println!("{}", msg),
            Err(e) => println!("Lỗi: {}", e),
        }

        println!("\nNhấn Enter để tiếp tục...");
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        terminal.clear()?;
    }

    Ok(())
}
