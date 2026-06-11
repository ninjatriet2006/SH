use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use crate::app::{App, Screen, PopupState, AppEvent, load_stored_accounts, save_stored_accounts};
use crate::app::operations::Operations;

pub async fn handle_account_key(app: &mut App, key: KeyEvent) {
    // Nếu đang trong popup ViewFile (Xem cấu hình / API Key)
    if let PopupState::ViewFile { name, content, scroll } = app.popup_state.clone() {
        match key.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Up => {
                if scroll > 0 {
                    app.popup_state = PopupState::ViewFile { name, content, scroll: scroll - 1 };
                }
            }
            KeyCode::Down => {
                if scroll + 1 < content.len() {
                    app.popup_state = PopupState::ViewFile { name, content, scroll: scroll + 1 };
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if name.contains("API Key") {
                    for line in &content {
                        if line.contains("API Key for") {
                            if let Some(colon_pos) = line.rfind(':') {
                                let key_str = line[colon_pos + 1..].trim().to_string();
                                let _ = Operations::copy_to_clipboard(&key_str);
                                app.popup_state = PopupState::Message {
                                    title: "Sao chép thành công".to_string(),
                                    message: "Đã sao chép API Key sạch vào clipboard hệ thống!".to_string(),
                                };
                                return;
                            }
                        }
                    }
                } else {
                    let config_lines: Vec<String> = content.iter()
                        .filter(|line| !line.starts_with("✨") && !line.starts_with("⚠️") && !line.starts_with("Bấm [C]"))
                        .cloned()
                        .collect();
                    let config_text = config_lines.join("\n").trim().to_string();
                    let _ = Operations::copy_to_clipboard(&config_text);
                    app.popup_state = PopupState::Message {
                        title: "Sao chép thành công".to_string(),
                        message: "Đã sao chép cấu hình đăng nhập vào clipboard hệ thống!".to_string(),
                    };
                    return;
                }
            }
            _ => {}
        }
        return;
    }

    // Nếu đang trong popup nhập mã 2FA (Bước 2)
    if let PopupState::TwoFAInput { email, password, keep_logged, mut twofa_buffer } = app.popup_state.clone() {
        match key.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Backspace => {
                twofa_buffer.pop();
                app.popup_state = PopupState::TwoFAInput {
                    email,
                    password,
                    keep_logged,
                    twofa_buffer,
                };
            }
            KeyCode::Char(c) => {
                twofa_buffer.push(c);
                app.popup_state = PopupState::TwoFAInput {
                    email,
                    password,
                    keep_logged,
                    twofa_buffer,
                };
            }
            KeyCode::Enter => {
                let code = twofa_buffer.trim().to_string();
                if !code.is_empty() {
                    app.popup_state = PopupState::None;
                    app.is_loading = true;
                    app.login_logs.clear();
                    
                    let tx = app.msg_tx.clone();
                    let email_clone = email.clone();
                    let pass_clone = password.clone();
                    let keep_logged_clone = keep_logged.clone();
                    tokio::spawn(async move {
                        let res = Operations::login_new(&email_clone, &pass_clone, Some(&code), &keep_logged_clone, tx.clone()).await;
                        if let Some(tx) = tx {
                            let _ = tx.send(AppEvent::LoginFinished {
                                email: email_clone,
                                password: pass_clone,
                                keep_logged: keep_logged_clone,
                                twofa_used: true,
                                result: res,
                            });
                        }
                    });
                }
            }
            _ => {}
        }
        return;
    }

    // Nếu đang trong popup QuickLoginSelect
    if let PopupState::QuickLoginSelect { options, selected_idx } = app.popup_state.clone() {
        match key.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Up => {
                let next_idx = if selected_idx == 0 {
                    options.len()
                } else {
                    selected_idx - 1
                };
                app.popup_state = PopupState::QuickLoginSelect {
                    options,
                    selected_idx: next_idx,
                };
            }
            KeyCode::Down => {
                let next_idx = if selected_idx >= options.len() {
                    0
                } else {
                    selected_idx + 1
                };
                app.popup_state = PopupState::QuickLoginSelect {
                    options,
                    selected_idx: next_idx,
                };
            }
            KeyCode::Enter => {
                if selected_idx == 0 {
                    // Đăng nhập thủ công
                    app.popup_state = PopupState::LoginInput {
                        email_buffer: String::new(),
                        pass_buffer: String::new(),
                        keep_logged: "y".to_string(),
                        active_field: 0,
                        error_msg: None,
                    };
                } else {
                    // Đăng nhập nhanh
                    let acc = &options[selected_idx - 1];
                    let email = acc.email.clone();
                    let password = acc.password.clone();
                    
                    app.popup_state = PopupState::None;
                    app.is_loading = true;
                    app.login_logs.clear();
                    
                    let tx = app.msg_tx.clone();
                    let email_clone = email.clone();
                    let pass_clone = password.clone();
                    let keep_clone = "y".to_string();
                    tokio::spawn(async move {
                        let res = Operations::login_new(&email_clone, &pass_clone, None, &keep_clone, tx.clone()).await;
                        if let Some(tx) = tx {
                            let _ = tx.send(AppEvent::LoginFinished {
                                email: email_clone,
                                password: pass_clone,
                                keep_logged: keep_clone,
                                twofa_used: false,
                                result: res,
                            });
                        }
                    });
                }
            }
            _ => {}
        }
        return;
    }

    // Nếu đang trong popup đăng nhập LoginInput (Bước 1: Email + Password)
    if let PopupState::LoginInput { mut email_buffer, mut pass_buffer, mut keep_logged, active_field, error_msg } = app.popup_state.clone() {
        match key.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Tab => {
                // Đổi trường tập trung (0: email, 1: pass, 2: keep_logged)
                let next_field = (active_field + 1) % 3;
                app.popup_state = PopupState::LoginInput {
                    email_buffer,
                    pass_buffer,
                    keep_logged,
                    active_field: next_field,
                    error_msg,
                };
            }
            KeyCode::Backspace => {
                if active_field == 0 {
                    email_buffer.pop();
                } else if active_field == 1 {
                    pass_buffer.pop();
                } else {
                    keep_logged.pop();
                }
                app.popup_state = PopupState::LoginInput {
                    email_buffer,
                    pass_buffer,
                    keep_logged,
                    active_field,
                    error_msg,
                };
            }
            KeyCode::Char(c) => {
                if active_field == 0 {
                    email_buffer.push(c);
                } else if active_field == 1 {
                    pass_buffer.push(c);
                } else {
                    if keep_logged.len() < 5 {
                        keep_logged.push(c);
                    }
                }
                app.popup_state = PopupState::LoginInput {
                    email_buffer,
                    pass_buffer,
                    keep_logged,
                    active_field,
                    error_msg,
                };
            }
            KeyCode::Enter => {
                if active_field == 0 {
                    // Nhấn Enter ở trường Email chuyển xuống Password
                    app.popup_state = PopupState::LoginInput {
                        email_buffer,
                        pass_buffer,
                        keep_logged,
                        active_field: 1,
                        error_msg,
                    };
                } else if active_field == 1 {
                    // Nhấn Enter ở trường Password chuyển xuống KeepLogged
                    app.popup_state = PopupState::LoginInput {
                        email_buffer,
                        pass_buffer,
                        keep_logged,
                        active_field: 2,
                        error_msg,
                    };
                } else {
                    // Thử đăng nhập không có 2FA trước
                    let email = email_buffer.trim().to_string();
                    let pass = pass_buffer.trim().to_string();
                    let keep = keep_logged.trim().to_string();

                    // Xóa credential nếu pass trống nhưng email không trống
                    if pass.is_empty() && !email.is_empty() {
                        let mut stored = load_stored_accounts();
                        if let Some(pos) = stored.iter().position(|acc| acc.email == email) {
                            stored.remove(pos);
                            save_stored_accounts(&stored);
                            app.popup_state = PopupState::Message {
                                title: "Xóa thông tin đăng nhập".to_string(),
                                message: format!("Đã xóa tài khoản {} khỏi danh sách đăng nhập nhanh.", email),
                            };
                        } else {
                            app.popup_state = PopupState::None;
                        }
                        return;
                    }

                    if email.is_empty() && pass.is_empty() {
                        app.popup_state = PopupState::None;
                        return;
                    }

                    if !email.is_empty() && !pass.is_empty() {
                        app.popup_state = PopupState::None;
                        app.is_loading = true;
                        app.login_logs.clear();
                        
                        let tx = app.msg_tx.clone();
                        let email_clone = email.clone();
                        let pass_clone = pass.clone();
                        let keep_clone = keep.clone();
                        tokio::spawn(async move {
                            let res = Operations::login_new(&email_clone, &pass_clone, None, &keep_clone, tx.clone()).await;
                            if let Some(tx) = tx {
                                let _ = tx.send(AppEvent::LoginFinished {
                                    email: email_clone,
                                    password: pass_clone,
                                    keep_logged: keep_clone,
                                    twofa_used: false,
                                    result: res,
                                });
                            }
                        });
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if key.code == KeyCode::Esc {
        app.current_screen = Screen::MainMenu;
        return;
    }

    // Điều hướng danh sách tài khoản
    if !app.accounts.is_empty() {
        match key.code {
            KeyCode::Up => {
                if app.active_account_idx == 0 {
                    app.active_account_idx = app.accounts.len() - 1;
                } else {
                    app.active_account_idx -= 1;
                }
            }
            KeyCode::Down => {
                app.active_account_idx = (app.active_account_idx + 1) % app.accounts.len();
            }
            _ => {}
        }
    }

    // Xử lý các phím Alt+ tổ hợp lệnh tài khoản
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Alt+S: Đổi tài khoản hoạt động
                if app.accounts.is_empty() {
                    return;
                }
                let selected = app.accounts[app.active_account_idx].clone();
                app.active_account = Some(selected.clone());
                app.sync_active_credentials();
                
                app.is_loading = true;
                // Làm mới Explorer & Storage Info
                app.trigger_refresh_storage_info();
                app.refresh_active_pane().await;
                app.active_pane_left = false;
                app.refresh_active_pane().await;
                app.active_pane_left = true;
                app.is_loading = false;

                app.popup_state = PopupState::Message {
                    title: "Chuyển tài khoản".to_string(),
                    message: format!("Đã chuyển sang tài khoản hoạt động: {}", selected),
                };
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Alt+N: Đăng nhập tài khoản mới (Quick Login nếu có sẵn)
                let stored = load_stored_accounts();
                if !stored.is_empty() {
                    app.popup_state = PopupState::QuickLoginSelect {
                        options: stored,
                        selected_idx: 0,
                    };
                } else {
                    app.popup_state = PopupState::LoginInput {
                        email_buffer: String::new(),
                        pass_buffer: String::new(),
                        keep_logged: "y".to_string(),
                        active_field: 0,
                        error_msg: None,
                    };
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Alt+D: Xóa tài khoản khỏi danh sách
                if app.accounts.is_empty() {
                    return;
                }
                let email = app.accounts[app.active_account_idx].clone();
                let is_default = app.default_email.as_ref().map_or(false, |d| *d == email);
                if is_default {
                    app.popup_state = PopupState::Message {
                        title: "Không thể gỡ".to_string(),
                        message: "Tài khoản mặc định không thể bị gỡ bỏ cấu hình riêng. Vui lòng dùng Alt+L để Đăng xuất.".to_string(),
                    };
                } else {
                    if let Some(home) = dirs::home_dir() {
                        let path = home.join(".config/filen-cli/accounts").join(&email);
                        if path.is_dir() {
                            let _ = std::fs::remove_dir_all(path);
                        }
                    }
                    app.refresh_accounts().await;
                    app.popup_state = PopupState::Message {
                        title: "Gỡ tài khoản".to_string(),
                        message: format!("Đã xóa cấu hình tài khoản {} khỏi máy tính.", email),
                    };
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                // Alt+L: Đăng xuất
                if app.active_account.is_none() {
                    return;
                }
                app.is_loading = true;
                match Operations::logout(&app.active_account).await {
                    Ok(_) => {
                        app.popup_state = PopupState::Message {
                            title: "Đăng xuất thành công".to_string(),
                            message: "Đã xóa toàn bộ session credentials của tài khoản hiện tại.".to_string(),
                        };
                        app.trigger_refresh_storage_info();
                        app.refresh_accounts().await;
                    }
                    Err(e) => {
                        app.popup_state = PopupState::Message {
                            title: "Lỗi đăng xuất".to_string(),
                            message: e,
                        };
                    }
                }
                app.is_loading = false;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Alt+C: Xuất Auth Config (xử lý bất đồng bộ để tránh treo UI)
                app.popup_state = PopupState::ViewFile {
                    name: "Cấu hình đăng nhập (Auth Config)".to_string(),
                    content: vec!["Đang truy xuất cấu hình đăng nhập...".to_string(), "Vui lòng đợi giây lát... ⏳".to_string()],
                    scroll: 0,
                };
                let tx = app.msg_tx.clone();
                let active = app.active_account.clone();
                tokio::spawn(async move {
                    let res = Operations::export_auth_config(&active).await;
                    if let Some(ref sender) = tx {
                        let _ = sender.send(AppEvent::ExportFinished {
                            is_api_key: false,
                            result: res,
                        });
                    }
                });
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                // Alt+K: Xuất API Key cho Rclone (xử lý bất đồng bộ để tránh treo UI)
                app.popup_state = PopupState::ViewFile {
                    name: "API Key cho Rclone".to_string(),
                    content: vec!["Đang truy xuất API Key cho Rclone...".to_string(), "Vui lòng đợi giây lát... ⏳".to_string()],
                    scroll: 0,
                };
                let tx = app.msg_tx.clone();
                let active = app.active_account.clone();
                tokio::spawn(async move {
                    let res = Operations::export_api_key(&active).await;
                    if let Some(ref sender) = tx {
                        let _ = sender.send(AppEvent::ExportFinished {
                            is_api_key: true,
                            result: res,
                        });
                    }
                });
            }
            _ => {}
        }
    }
}
