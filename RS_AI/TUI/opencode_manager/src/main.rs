mod api;
mod app;
mod ckey;
mod config;
mod ui;

use app::{App, AppMessage, ConfirmAction, Screen};
use config::{AuthEntry, OpencodeConfig};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal()
        && std::env::var("OPENCODE_MANAGER_WRAPPED").is_err()
        && let Ok(exe) = std::env::current_exe()
    {
        #[cfg(target_os = "linux")]
        {
            let terminals = [
                "gnome-terminal",
                "konsole",
                "xfce4-terminal",
                "x-terminal-emulator",
                "xterm",
            ];
            for term in terminals {
                if std::process::Command::new(term)
                    .arg("--")
                    .arg(&exe)
                    .env("OPENCODE_MANAGER_WRAPPED", "1")
                    .spawn()
                    .is_ok()
                    || std::process::Command::new(term)
                        .arg("-e")
                        .arg(&exe)
                        .env("OPENCODE_MANAGER_WRAPPED", "1")
                        .spawn()
                        .is_ok()
                {
                    return Ok(());
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = exe;
        }
    }

    // 1. Setup terminal raw mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Setup panic hook to restore terminal if app crashes
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // 3. Khởi tạo cấu hình và kênh truyền tin
    let opencode_res = OpencodeConfig::load();
    let auth_res = AuthEntry::load_config();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();

    let mut app = match (opencode_res, auth_res) {
        (Ok(cfg), Ok(auth)) => App::new(cfg, auth, tx),
        (Err(e), _) | (_, Err(e)) => {
            // Restore terminal before printing error
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            eprintln!("Lỗi khởi tạo cấu hình: {}", e);
            std::process::exit(1);
        }
    };

    app.log("Ứng dụng khởi động thành công.");

    // Tự động đồng bộ các API key từ auth.json khi khởi chạy
    app.sync_providers_from_auth(true);

    // Tự động check all khi bắt đầu để người dùng thấy trạng thái ngay
    app.check_all_providers();

    // 4. Vòng lặp chính (Main loop)
    let mut draw_needed = true;
    let mut last_tick = std::time::Instant::now();
    let tick_rate = Duration::from_millis(50);

    loop {
        // Vẽ giao diện khi cần
        if draw_needed {
            terminal.draw(|f| ui::draw(f, &mut app))?;
            draw_needed = false;
        }

        // Kiểm tra xem có nhận được thông điệp từ background task không
        while let Ok(msg) = rx.try_recv() {
            app.handle_message(msg);
            draw_needed = true;
        }

        // Poll sự kiện từ Crossterm
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            let ev = event::read()?;
            if let Event::Resize(_, _) = ev {
                draw_needed = true;
            }
            if let Event::Key(key) = ev {
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }

                // Xử lý nút thoát nhanh (Ctrl+C hoặc Q tại màn hình chính)
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                // Xử lý phím tắt Alt+O để khởi chạy OpenCode
                if key.modifiers.contains(KeyModifiers::ALT)
                    && (key.code == KeyCode::Char('o') || key.code == KeyCode::Char('O'))
                {
                    app.launch_opencode();
                    draw_needed = true;
                    continue;
                }

                match app.current_screen {
                    Screen::Main => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Up if app.selected_provider_idx > 0 => {
                            app.selected_provider_idx -= 1;
                            draw_needed = true;
                        }
                        KeyCode::Down | KeyCode::Char('j')
                            if app.selected_provider_idx + 1 < app.providers_keys.len() =>
                        {
                            app.selected_provider_idx += 1;
                            draw_needed = true;
                        }
                        KeyCode::Enter => {
                            // Vừa check trạng thái vừa quét models
                            app.check_selected_provider();
                            app.scan_models_selected();
                            draw_needed = true;
                        }
                        KeyCode::Char(' ') => {
                            app.check_all_providers();
                            draw_needed = true;
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            app.open_add_provider();
                            draw_needed = true;
                        }
                        KeyCode::Char('e') | KeyCode::Char('E') => {
                            app.open_edit_provider();
                            draw_needed = true;
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                            app.open_delete_provider_confirm();
                            draw_needed = true;
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            app.open_quick_clean();
                            draw_needed = true;
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            app.open_auth_keys_manager();
                            draw_needed = true;
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            app.sync_providers_from_auth(false);
                            draw_needed = true;
                        }
                        KeyCode::Char('g') | KeyCode::Char('G') if app.has_ckey_support() => {
                            app.open_ckey_dashboard();
                            draw_needed = true;
                        }
                        KeyCode::Char('k') | KeyCode::Char('K') => {
                            app.open_bulk_add();
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::AddProvider | Screen::EditProvider => {
                        let focus = app.form.focus_index;
                        if app.form.is_editing_field {
                            // Chế độ gõ chữ (Insert Mode)
                            match key.code {
                                KeyCode::Enter | KeyCode::Esc => {
                                    app.form.is_editing_field = false;
                                    app.log("Thoát chế độ sửa ô nhập.");
                                    draw_needed = true;
                                }
                                KeyCode::Backspace => match focus {
                                    1 => {
                                        app.form.name.pop();
                                        draw_needed = true;
                                    }
                                    2 => {
                                        app.form.base_url.pop();
                                        draw_needed = true;
                                    }
                                    3 => {
                                        app.form.api_key.pop();
                                        draw_needed = true;
                                    }
                                    4 => {
                                        app.form.account_key.pop();
                                        draw_needed = true;
                                    }
                                    _ => {}
                                },
                                KeyCode::Char(c) => match focus {
                                    1 => {
                                        app.form.name.push(c);
                                        draw_needed = true;
                                    }
                                    2 => {
                                        app.form.base_url.push(c);
                                        draw_needed = true;
                                    }
                                    3 => {
                                        app.form.api_key.push(c);
                                        draw_needed = true;
                                    }
                                    4 => {
                                        app.form.account_key.push(c);
                                        draw_needed = true;
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        } else {
                            // Chế độ điều hướng (Navigation Mode)
                            match key.code {
                                KeyCode::Esc => {
                                    app.current_screen = Screen::Main;
                                    app.log("Huỷ thao tác nhập form.");
                                    draw_needed = true;
                                }
                                KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                                    let max_focus = 7;
                                    app.form.focus_index = (focus + 1) % (max_focus + 1);
                                    draw_needed = true;
                                }
                                KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
                                    let max_focus = 7;
                                    if focus == 0 {
                                        app.form.focus_index = max_focus;
                                    } else {
                                        app.form.focus_index = focus - 1;
                                    }
                                    draw_needed = true;
                                }
                                KeyCode::Left | KeyCode::Char('h') if focus == 0 => {
                                    app.cycle_form_preset(false);
                                    draw_needed = true;
                                }
                                KeyCode::Right | KeyCode::Char('l') if focus == 0 => {
                                    app.cycle_form_preset(true);
                                    draw_needed = true;
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') if focus >= 5 => {
                                    app.test_form_connection();
                                    draw_needed = true;
                                }
                                KeyCode::Char('s') | KeyCode::Char('S') if focus >= 5 => {
                                    if let Err(e) = app.save_form() {
                                        app.log(format!("Không thể lưu: {}", e));
                                    }
                                    draw_needed = true;
                                }
                                KeyCode::Enter => {
                                    match focus {
                                        0 => {
                                            app.preset_search_query = String::new();
                                            app.selected_preset_search_idx = 0;
                                            app.preset_list_state = ratatui::widgets::ListState::default();
                                            app.current_screen = Screen::SelectPreset;
                                            app.log("Mở màn hình tìm kiếm Preset.");
                                        }
                                        1..=4 => {
                                            app.form.is_editing_field = true;
                                            app.log("Bật chế độ sửa ô nhập. Gõ chữ xong nhấn Enter để hoàn tất.");
                                        }
                                        5 => {
                                            app.test_form_connection();
                                        }
                                        6 => {
                                            if let Err(e) = app.save_form() {
                                                app.log(format!("Không thể lưu: {}", e));
                                            }
                                        }
                                        7 => {
                                            app.current_screen = Screen::Main;
                                            app.log("Huỷ thao tác nhập form.");
                                        }
                                        _ => {}
                                    }
                                    draw_needed = true;
                                }
                                _ => {}
                            }
                        }
                    }

                    Screen::ModelScanResult => match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::Main;
                            draw_needed = true;
                        }
                        KeyCode::Up => {
                            let filtered = app.filtered_scanned_models();
                            if !filtered.is_empty() && app.selected_model_idx > 0 {
                                app.selected_model_idx -= 1;
                                draw_needed = true;
                            }
                        }
                        KeyCode::Down => {
                            let filtered = app.filtered_scanned_models();
                            if !filtered.is_empty() && app.selected_model_idx + 1 < filtered.len() {
                                app.selected_model_idx += 1;
                                draw_needed = true;
                            }
                        }
                        KeyCode::Char(' ') => {
                            let filtered = app.filtered_scanned_models();
                            if !filtered.is_empty() && app.selected_model_idx < filtered.len() {
                                let (original_idx, _, _, _) = filtered[app.selected_model_idx];
                                if let Some(item) = app.scanned_models.get_mut(original_idx) {
                                    item.1 = !item.1;
                                    draw_needed = true;
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.model_search_query.pop();
                            app.selected_model_idx = 0;
                            app.models_list_state = ratatui::widgets::ListState::default();
                            draw_needed = true;
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app.add_scanned_models() {
                                app.log(format!("Lỗi lưu models: {}", e));
                            }
                            draw_needed = true;
                        }
                        KeyCode::Char(c)
                            if !key.modifiers.contains(KeyModifiers::CONTROL)
                                && !key.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            app.model_search_query.push(c);
                            app.selected_model_idx = 0;
                            app.models_list_state = ratatui::widgets::ListState::default();
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::ManageAuthKeys => match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::Main;
                            draw_needed = true;
                        }
                        KeyCode::Up | KeyCode::Char('k') if app.selected_auth_idx > 0 => {
                            app.selected_auth_idx -= 1;
                            draw_needed = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') if app.selected_auth_idx + 1 < app.auth_keys.len() => {
                            app.selected_auth_idx += 1;
                            draw_needed = true;
                        }
                        KeyCode::Delete => {
                            app.open_delete_auth_key_confirm();
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::QuickClean => match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::Main;
                            draw_needed = true;
                        }
                        KeyCode::Up | KeyCode::Char('k') if app.selected_clean_idx > 0 => {
                            app.selected_clean_idx -= 1;
                            draw_needed = true;
                        }
                        KeyCode::Down | KeyCode::Char('j') if app.selected_clean_idx + 1 < app.clean_list.len() => {
                            app.selected_clean_idx += 1;
                            draw_needed = true;
                        }
                        KeyCode::Char(' ') => {
                            if let Some(item) = app.clean_list.get_mut(app.selected_clean_idx) {
                                item.3 = !item.3;
                                draw_needed = true;
                            }
                        }
                        KeyCode::Enter => {
                            // Chuyển sang màn hình xác nhận xoá các API đã chọn
                            app.confirm_action = Some(ConfirmAction::CleanSelected);
                            app.confirm_focus_yes = false;
                            app.current_screen = Screen::Confirmation;
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::Confirmation => match key.code {
                        KeyCode::Esc => {
                            // Quay lại màn hình cũ
                            app.current_screen = match &app.confirm_action {
                                Some(ConfirmAction::DeleteAuthKey(_)) => Screen::ManageAuthKeys,
                                Some(ConfirmAction::CleanSelected) => Screen::QuickClean,
                                Some(ConfirmAction::OverwriteDuplicate { .. }) => {
                                    if app.form.id.is_empty() {
                                        Screen::AddProvider
                                    } else {
                                        Screen::EditProvider
                                    }
                                }
                                _ => Screen::Main,
                            };
                            app.confirm_action = None;
                            draw_needed = true;
                        }
                        KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                            app.confirm_focus_yes = !app.confirm_focus_yes;
                            draw_needed = true;
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.confirm_focus_yes = true;
                            if let Some(action) = app.confirm_action.clone() {
                                match action {
                                    ConfirmAction::DeleteProvider(id) => {
                                        let _ = app.execute_delete_provider(&id);
                                    }
                                    ConfirmAction::DeleteAuthKey(name) => {
                                        let _ = app.execute_delete_auth_key(&name);
                                    }
                                    ConfirmAction::CleanSelected => {
                                        let _ = app.execute_quick_clean();
                                    }
                                    ConfirmAction::OverwriteDuplicate { duplicate_id, .. } => {
                                        if let Err(e) = app.execute_overwrite_duplicate(&duplicate_id) {
                                            app.log(format!("Không thể ghi đè/gộp: {}", e));
                                        }
                                    }
                                }
                                app.confirm_action = None;
                            }
                            draw_needed = true;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            app.confirm_focus_yes = false;
                            app.current_screen = match &app.confirm_action {
                                Some(ConfirmAction::DeleteAuthKey(_)) => Screen::ManageAuthKeys,
                                Some(ConfirmAction::CleanSelected) => Screen::QuickClean,
                                Some(ConfirmAction::OverwriteDuplicate { .. }) => {
                                    if app.form.id.is_empty() {
                                        Screen::AddProvider
                                    } else {
                                        Screen::EditProvider
                                    }
                                }
                                _ => Screen::Main,
                            };
                            app.confirm_action = None;
                            draw_needed = true;
                        }
                        KeyCode::Enter => {
                            if app.confirm_focus_yes {
                                if let Some(action) = app.confirm_action.clone() {
                                    match action {
                                        ConfirmAction::DeleteProvider(id) => {
                                            let _ = app.execute_delete_provider(&id);
                                        }
                                        ConfirmAction::DeleteAuthKey(name) => {
                                            let _ = app.execute_delete_auth_key(&name);
                                        }
                                        ConfirmAction::CleanSelected => {
                                            let _ = app.execute_quick_clean();
                                        }
                                        ConfirmAction::OverwriteDuplicate { duplicate_id, .. } => {
                                            if let Err(e) = app.execute_overwrite_duplicate(&duplicate_id) {
                                                app.log(format!("Không thể ghi đè/gộp: {}", e));
                                            }
                                        }
                                    }
                                }
                            } else {
                                app.current_screen = match &app.confirm_action {
                                    Some(ConfirmAction::DeleteAuthKey(_)) => Screen::ManageAuthKeys,
                                    Some(ConfirmAction::CleanSelected) => Screen::QuickClean,
                                    Some(ConfirmAction::OverwriteDuplicate { .. }) => {
                                        if app.form.id.is_empty() {
                                            Screen::AddProvider
                                        } else {
                                            Screen::EditProvider
                                        }
                                    }
                                    _ => Screen::Main,
                                };
                            }
                            app.confirm_action = None;
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::SelectPreset => match key.code {
                        KeyCode::Esc => {
                            app.current_screen = if app.form.id.is_empty() {
                                Screen::AddProvider
                            } else {
                                Screen::EditProvider
                            };
                            draw_needed = true;
                        }
                        KeyCode::Up => {
                            let filtered = app.filtered_presets();
                            if !filtered.is_empty() && app.selected_preset_search_idx > 0 {
                                app.selected_preset_search_idx -= 1;
                            }
                            draw_needed = true;
                        }
                        KeyCode::Down => {
                            let filtered = app.filtered_presets();
                            if !filtered.is_empty() && app.selected_preset_search_idx + 1 < filtered.len() {
                                app.selected_preset_search_idx += 1;
                            }
                            draw_needed = true;
                        }
                        KeyCode::Backspace => {
                            app.preset_search_query.pop();
                            app.selected_preset_search_idx = 0;
                            app.preset_list_state = ratatui::widgets::ListState::default();
                            draw_needed = true;
                        }
                        KeyCode::Char(c)
                            if !key.modifiers.contains(KeyModifiers::CONTROL)
                                && !key.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            app.preset_search_query.push(c);
                            app.selected_preset_search_idx = 0;
                            app.preset_list_state = ratatui::widgets::ListState::default();
                            draw_needed = true;
                        }
                        KeyCode::Enter => {
                            let filtered = app.filtered_presets();
                            if !filtered.is_empty() && app.selected_preset_search_idx < filtered.len() {
                                let selected = filtered[app.selected_preset_search_idx].clone();
                                app.select_preset(selected);
                            }
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::CKeyDashboard => {
                        if app.ckey_need_key {
                            // Popup chọn/nhập account key
                            match app.ckey_pick_mode {
                                app::CkeyPickMode::Choose => match key.code {
                                    KeyCode::Esc => {
                                        app.current_screen = Screen::Main;
                                        app.ckey_need_key = false;
                                        draw_needed = true;
                                    }
                                    KeyCode::Up if app.ckey_pick_selected_idx > 0 => {
                                        app.ckey_pick_selected_idx -= 1;
                                        draw_needed = true;
                                    }
                                    KeyCode::Down
                                        if app.ckey_pick_selected_idx < app.ckey_account_options.len() =>
                                    {
                                        app.ckey_pick_selected_idx += 1;
                                        draw_needed = true;
                                    }
                                    KeyCode::Tab => {
                                        app.ckey_pick_mode = app::CkeyPickMode::New;
                                        app.ckey_new_key_input.clear();
                                        draw_needed = true;
                                    }
                                    KeyCode::Enter => {
                                        let options = app.ckey_account_options.clone();
                                        if app.ckey_pick_selected_idx < options.len() {
                                            let pid = options[app.ckey_pick_selected_idx].0.clone();
                                            app.ckey_pick_account_key(&pid);
                                        } else {
                                            // Mục cuối: chuyển sang nhập key mới
                                            app.ckey_pick_mode = app::CkeyPickMode::New;
                                            app.ckey_new_key_input.clear();
                                        }
                                        draw_needed = true;
                                    }
                                    _ => {}
                                },
                                app::CkeyPickMode::New => match key.code {
                                    KeyCode::Esc => {
                                        app.current_screen = Screen::Main;
                                        app.ckey_need_key = false;
                                        draw_needed = true;
                                    }
                                    KeyCode::Tab => {
                                        app.ckey_pick_mode = app::CkeyPickMode::Choose;
                                        draw_needed = true;
                                    }
                                    KeyCode::Backspace => {
                                        app.ckey_new_key_input.pop();
                                        draw_needed = true;
                                    }
                                    KeyCode::Enter => {
                                        if let Err(e) = app.ckey_save_new_account_key() {
                                            app.log(format!("Không thể lưu account key: {}", e));
                                        }
                                        draw_needed = true;
                                    }
                                    KeyCode::Char(c)
                                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                                    {
                                        app.ckey_new_key_input.push(c);
                                        draw_needed = true;
                                    }
                                    _ => {}
                                },
                            }
                        } else {
                            // Màn hình dashboard thường: R/I/U/Esc
                            match key.code {
                                KeyCode::Esc => {
                                    app.current_screen = Screen::Main;
                                    draw_needed = true;
                                }
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    app.ckey_fetch_all();
                                    draw_needed = true;
                                }
                                KeyCode::Char('i') | KeyCode::Char('I') => {
                                    app.open_ckey_import();
                                    draw_needed = true;
                                }
                                KeyCode::Char('u') | KeyCode::Char('U') => {
                                    app.open_ckey_usage();
                                    draw_needed = true;
                                }
                                _ => {}
                            }
                        }
                    }

                    Screen::BulkAddProviders => {
                        if app.bulk_is_editing {
                            // Chế độ gõ chữ
                            match key.code {
                                KeyCode::Enter | KeyCode::Esc => {
                                    app.bulk_is_editing = false;
                                    app.log("Thoát chế độ sửa ô nhập.");
                                    draw_needed = true;
                                }
                                KeyCode::Backspace => match app.bulk_focus {
                                    app::BulkFocus::Endpoint => {
                                        app.bulk_endpoint_input.pop();
                                        draw_needed = true;
                                    }
                                    app::BulkFocus::Keys => {
                                        app.bulk_keys_input.pop();
                                        draw_needed = true;
                                    }
                                    app::BulkFocus::Execute => {}
                                },
                                KeyCode::Char(c) => match app.bulk_focus {
                                    app::BulkFocus::Endpoint => {
                                        app.bulk_endpoint_input.push(c);
                                        draw_needed = true;
                                    }
                                    app::BulkFocus::Keys => {
                                        app.bulk_keys_input.push(c);
                                        draw_needed = true;
                                    }
                                    app::BulkFocus::Execute => {}
                                },
                                _ => {}
                            }
                        } else {
                            // Chế độ điều hướng
                            match key.code {
                                KeyCode::Esc => {
                                    app.current_screen = Screen::Main;
                                    draw_needed = true;
                                }
                                KeyCode::Up | KeyCode::Down | KeyCode::Tab
                                | KeyCode::Char('j') | KeyCode::Char('k') => {
                                    app.bulk_focus = match app.bulk_focus {
                                        app::BulkFocus::Endpoint => app::BulkFocus::Keys,
                                        app::BulkFocus::Keys => app::BulkFocus::Execute,
                                        app::BulkFocus::Execute => app::BulkFocus::Endpoint,
                                    };
                                    draw_needed = true;
                                }
                                KeyCode::Enter => match app.bulk_focus {
                                    app::BulkFocus::Endpoint | app::BulkFocus::Keys => {
                                        app.bulk_is_editing = true;
                                        app.log("Bật chế độ sửa ô nhập. Gõ chữ xong nhấn Enter để hoàn tất.");
                                        draw_needed = true;
                                    }
                                    app::BulkFocus::Execute => {
                                        if let Err(e) = app.execute_bulk_add() {
                                            app.log(format!("Không thể thêm nhanh: {}", e));
                                        }
                                        draw_needed = true;
                                    }
                                },
                                _ => {}
                            }
                        }
                    }

                    Screen::CKeyImport => match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::CKeyDashboard;
                            draw_needed = true;
                        }
                        KeyCode::Up => {
                            let filtered = app.filtered_ckey_import();
                            if !filtered.is_empty() && app.ckey_import_idx > 0 {
                                app.ckey_import_idx -= 1;
                                draw_needed = true;
                            }
                        }
                        KeyCode::Down => {
                            let filtered = app.filtered_ckey_import();
                            if !filtered.is_empty() && app.ckey_import_idx + 1 < filtered.len() {
                                app.ckey_import_idx += 1;
                                draw_needed = true;
                            }
                        }
                        KeyCode::Char(' ') => {
                            let filtered = app.filtered_ckey_import();
                            if !filtered.is_empty() && app.ckey_import_idx < filtered.len() {
                                let (original_idx, _, _, _, _, _) = filtered[app.ckey_import_idx];
                                if let Some(item) = app.ckey_import_list.get_mut(original_idx) {
                                    item.1 = !item.1;
                                    draw_needed = true;
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.ckey_import_query.pop();
                            app.ckey_import_idx = 0;
                            app.ckey_import_list_state = ratatui::widgets::ListState::default();
                            draw_needed = true;
                        }
                        KeyCode::Enter => {
                            if let Err(e) = app.execute_ckey_import() {
                                app.log(format!("Lỗi đồng bộ model CKey: {}", e));
                            }
                            draw_needed = true;
                        }
                        KeyCode::Char(c)
                            if !key.modifiers.contains(KeyModifiers::CONTROL)
                                && !key.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            app.ckey_import_query.push(c);
                            app.ckey_import_idx = 0;
                            app.ckey_import_list_state = ratatui::widgets::ListState::default();
                            draw_needed = true;
                        }
                        _ => {}
                    },

                    Screen::CKeyUsage => match key.code {
                        KeyCode::Esc => {
                            app.current_screen = Screen::CKeyDashboard;
                            draw_needed = true;
                        }
                        KeyCode::Up => {
                            if app.ckey_usage_scroll > 0 {
                                app.ckey_usage_scroll -= 1;
                                draw_needed = true;
                            }
                        }
                        KeyCode::Down => {
                            app.ckey_usage_scroll += 1;
                            draw_needed = true;
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                            if app.ckey_usage_page > 1 {
                                app.ckey_fetch_usage_page(app.ckey_usage_page - 1);
                            }
                            draw_needed = true;
                        }
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                            if app.ckey_usage_page < app.ckey_usage_total_pages {
                                app.ckey_fetch_usage_page(app.ckey_usage_page + 1);
                            }
                            draw_needed = true;
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            app.ckey_fetch_usage_page(app.ckey_usage_page);
                            draw_needed = true;
                        }
                        _ => {}
                    },
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = std::time::Instant::now();
        }
    }

    // 5. Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
