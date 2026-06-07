use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use crate::app::{App, Screen, PopupState};
use crate::app::operations::Operations;

pub async fn handle_account_key(app: &mut App, key: KeyEvent) {
    // Nếu đang trong popup đăng nhập LoginInput
    if let PopupState::LoginInput { mut email_buffer, mut pass_buffer, active_field } = app.popup_state.clone() {
        match key.code {
            KeyCode::Esc => {
                app.popup_state = PopupState::None;
            }
            KeyCode::Tab => {
                // Đổi trường tập trung (0: email, 1: pass)
                let next_field = (active_field + 1) % 2;
                app.popup_state = PopupState::LoginInput {
                    email_buffer,
                    pass_buffer,
                    active_field: next_field,
                };
            }
            KeyCode::Backspace => {
                if active_field == 0 {
                    email_buffer.pop();
                } else {
                    pass_buffer.pop();
                }
                app.popup_state = PopupState::LoginInput {
                    email_buffer,
                    pass_buffer,
                    active_field,
                };
            }
            KeyCode::Char(c) => {
                if active_field == 0 {
                    email_buffer.push(c);
                } else {
                    pass_buffer.push(c);
                }
                app.popup_state = PopupState::LoginInput {
                    email_buffer,
                    pass_buffer,
                    active_field,
                };
            }
            KeyCode::Enter => {
                if active_field == 0 {
                    // Nhấn Enter ở trường Email chuyển xuống Password
                    app.popup_state = PopupState::LoginInput {
                        email_buffer,
                        pass_buffer,
                        active_field: 1,
                    };
                } else {
                    // Thực hiện Đăng nhập
                    app.popup_state = PopupState::None;
                    app.is_loading = true;
                    let email = email_buffer.trim().to_string();
                    let pass = pass_buffer.trim().to_string();
                    if !email.is_empty() && !pass.is_empty() {
                        match Operations::login_new(&email, &pass).await {
                            Ok(_) => {
                                app.refresh_accounts();
                                app.popup_state = PopupState::Message {
                                    title: "Đăng nhập thành công".to_string(),
                                    message: format!("Tài khoản {} đã được nạp thành công trên TUI.", email),
                                };
                            }
                            Err(e) => {
                                app.popup_state = PopupState::Message {
                                    title: "Lỗi đăng nhập".to_string(),
                                    message: format!("Chi tiết lỗi: {}", e),
                                };
                            }
                        }
                    }
                    app.is_loading = false;
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

    // Xử lý các phím Alt+ tổ hợp lệnh tài khoản
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Alt+S: Đổi tài khoản hoạt động
                let selected = app.accounts[app.active_account_idx].clone();
                if app.active_account_idx == 0 {
                    app.active_account = None;
                } else {
                    app.active_account = Some(selected.clone());
                }
                
                app.is_loading = true;
                // Làm mới Explorer & Storage Info
                app.refresh_storage_info().await;
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
                // Alt+N: Thêm tài khoản mới
                app.popup_state = PopupState::LoginInput {
                    email_buffer: String::new(),
                    pass_buffer: String::new(),
                    active_field: 0,
                };
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // Alt+D: Xóa tài khoản khỏi danh sách
                if app.active_account_idx == 0 {
                    app.popup_state = PopupState::Message {
                        title: "Không thể gỡ".to_string(),
                        message: "Tài khoản mặc định (Default Session) không thể bị gỡ bỏ!".to_string(),
                    };
                } else {
                    let email = app.accounts[app.active_account_idx].clone();
                    if let Some(home) = dirs::home_dir() {
                        let path = home.join(".config/filen-cli/accounts").join(&email);
                        if path.is_dir() {
                            let _ = std::fs::remove_dir_all(path);
                        }
                    }
                    app.refresh_accounts();
                    app.popup_state = PopupState::Message {
                        title: "Gỡ tài khoản".to_string(),
                        message: format!("Đã xóa cấu hình tài khoản {} khỏi máy tính.", email),
                    };
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                // Alt+L: Đăng xuất
                app.is_loading = true;
                match Operations::logout(&app.active_account).await {
                    Ok(_) => {
                        app.popup_state = PopupState::Message {
                            title: "Đăng xuất thành công".to_string(),
                            message: "Đã xóa toàn bộ session credentials của tài khoản hiện tại.".to_string(),
                        };
                        app.refresh_storage_info().await;
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
                // Alt+C: Xuất Auth Config
                app.is_loading = true;
                match Operations::export_auth_config(&app.active_account).await {
                    Ok(text) => {
                        app.popup_state = PopupState::ViewFile {
                            name: "Cấu hình đăng nhập (Auth Config)".to_string(),
                            content: text.lines().map(|s| s.to_string()).collect(),
                            scroll: 0,
                        };
                    }
                    Err(e) => {
                        app.popup_state = PopupState::Message {
                            title: "Lỗi xuất cấu hình".to_string(),
                            message: e,
                        };
                    }
                }
                app.is_loading = false;
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                // Alt+K: Xuất API Key cho Rclone
                app.is_loading = true;
                match Operations::export_api_key(&app.active_account).await {
                    Ok(text) => {
                        app.popup_state = PopupState::ViewFile {
                            name: "API Key cho Rclone".to_string(),
                            content: text.lines().map(|s| s.to_string()).collect(),
                            scroll: 0,
                        };
                    }
                    Err(e) => {
                        app.popup_state = PopupState::Message {
                            title: "Lỗi xuất API Key".to_string(),
                            message: e,
                        };
                    }
                }
                app.is_loading = false;
            }
            _ => {}
        }
    }
}
