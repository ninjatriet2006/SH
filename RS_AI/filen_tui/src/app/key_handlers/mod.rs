pub mod explorer;
pub mod account;

use crossterm::event::{KeyEvent, KeyCode};
use crate::app::{App, Screen, PopupState};

pub async fn handle_key(app: &mut App, key: KeyEvent, quit: &mut bool) {
    // Nếu có popup tin nhắn (Message) đang hiện, bất kỳ phím nào (Enter/Esc) sẽ đóng popup
    if let PopupState::Message { .. } = app.popup_state {
        if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
            app.popup_state = PopupState::None;
        }
        return;
    }

    match app.current_screen {
        Screen::MainMenu => {
            handle_menu_key(app, key, quit).await;
        }
        Screen::Explorer => {
            explorer::handle_explorer_key(app, key).await;
        }
        Screen::Account => {
            account::handle_account_key(app, key).await;
        }
        Screen::Servers => {
            handle_servers_key(app, key).await;
        }
    }
}

async fn handle_menu_key(app: &mut App, key: KeyEvent, quit: &mut bool) {
    match key.code {
        KeyCode::Up => {
            if app.main_menu_selected == 0 {
                app.main_menu_selected = 3;
            } else {
                app.main_menu_selected -= 1;
            }
        }
        KeyCode::Down => {
            app.main_menu_selected = (app.main_menu_selected + 1) % 4;
        }
        KeyCode::Enter => {
            match app.main_menu_selected {
                0 => app.current_screen = Screen::Explorer,
                1 => {
                    app.current_screen = Screen::Account;
                    app.trigger_refresh_storage_info();
                }
                2 => {
                    // Mở Thùng rác: Phím Alt+O -> mục 3/4 trong Explorer
                    app.current_screen = Screen::Explorer;
                    app.popup_state = PopupState::SpecialActionsMenu { selected_idx: 2 };
                }
                3 => app.current_screen = Screen::Servers,
                _ => {}
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            *quit = true;
        }
        _ => {}
    }
}

async fn handle_servers_key(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Esc {
        app.current_screen = Screen::MainMenu;
        return;
    }

    match key.code {
        KeyCode::Tab => {
            app.active_server_tab = (app.active_server_tab + 1) % 2;
            app.server_selected_field = 0;
        }
        KeyCode::Up => {
            if app.server_selected_field == 0 {
                app.server_selected_field = 4; // 5 trường (0: User/Access Key, 1: Pass/Secret Key, 2: Port, 3: HTTPS, 4: Bật/Tắt)
            } else {
                app.server_selected_field -= 1;
            }
        }
        KeyCode::Down => {
            app.server_selected_field = (app.server_selected_field + 1) % 5;
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if app.server_selected_field == 4 {
                // Kích hoạt Bật/Tắt server
                toggle_server(app).await;
            } else if app.server_selected_field == 3 {
                // Toggle HTTPS bool
                if app.active_server_tab == 0 {
                    app.webdav_server.https = !app.webdav_server.https;
                } else {
                    app.s3_server.https = !app.s3_server.https;
                }
            } else {
                // Nhập text cho các trường còn lại
                // Ở đây chúng ta có thể tạo popup nhập văn bản, để đơn giản hóa,
                // chúng ta sẽ có cơ chế popup sửa cấu hình.
                let (title, buffer) = if app.active_server_tab == 0 {
                    match app.server_selected_field {
                        0 => ("Nhập Username WebDAV".to_string(), app.webdav_server.user.clone()),
                        1 => ("Nhập Password WebDAV".to_string(), app.webdav_server.pass.clone()),
                        2 => ("Nhập Port WebDAV".to_string(), app.webdav_server.port.clone()),
                        _ => (String::new(), String::new()),
                    }
                } else {
                    match app.server_selected_field {
                        0 => ("Nhập Access Key ID S3".to_string(), app.s3_server.access_key.clone()),
                        1 => ("Nhập Secret Access Key S3".to_string(), app.s3_server.secret_key.clone()),
                        2 => ("Nhập Port S3".to_string(), app.s3_server.port.clone()),
                        _ => (String::new(), String::new()),
                    }
                };
                if !title.is_empty() {
                    app.popup_state = PopupState::RenameInput {
                        old_name: title,
                        buffer,
                    };
                    app.edit_cursor_idx = 0;
                }
            }
        }
        _ => {}
    }
}

async fn toggle_server(app: &mut App) {
    if app.active_server_tab == 0 {
        // WebDAV
        if app.webdav_server.running {
            if let Some(mut child) = app.webdav_server.child.take() {
                let _ = child.kill().await;
            }
            app.webdav_server.running = false;
            app.webdav_server.logs.push("Máy chủ WebDAV đã dừng.".to_string());
        } else {
            // Chạy lệnh CLI: filen webdav --w-user ... --w-password ... --w-port ...
            let mut cmd = crate::app::operations::Operations::get_command(&app.active_account);
            cmd.arg("webdav")
               .arg("--w-user").arg(&app.webdav_server.user)
               .arg("--w-password").arg(&app.webdav_server.pass)
               .arg("--w-port").arg(&app.webdav_server.port);
            if app.webdav_server.https {
                cmd.arg("--w-https");
            }
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            
            // Import stdio
            use std::process::Stdio;
            match cmd.spawn() {
                Ok(child) => {
                    app.webdav_server.child = Some(child);
                    app.webdav_server.running = true;
                    app.webdav_server.logs.push(format!("Đã khởi chạy WebDAV trên cổng {}.", app.webdav_server.port));
                }
                Err(e) => {
                    app.webdav_server.logs.push(format!("Lỗi khi bật WebDAV: {}", e));
                }
            }
        }
    } else {
        // S3
        if app.s3_server.running {
            if let Some(mut child) = app.s3_server.child.take() {
                let _ = child.kill().await;
            }
            app.s3_server.running = false;
            app.s3_server.logs.push("Máy chủ S3 đã dừng.".to_string());
        } else {
            // Chạy lệnh CLI: filen s3 --s3-access-key-id ... --s3-secret-access-key ... --s3-port ...
            let mut cmd = crate::app::operations::Operations::get_command(&app.active_account);
            cmd.arg("s3")
               .arg("--s3-access-key-id").arg(&app.s3_server.access_key)
               .arg("--s3-secret-access-key").arg(&app.s3_server.secret_key)
               .arg("--s3-port").arg(&app.s3_server.port);
            if app.s3_server.https {
                cmd.arg("--s3-https");
            }
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            
            use std::process::Stdio;
            match cmd.spawn() {
                Ok(child) => {
                    app.s3_server.child = Some(child);
                    app.s3_server.running = true;
                    app.s3_server.logs.push(format!("Đã khởi chạy S3 trên cổng {}.", app.s3_server.port));
                }
                Err(e) => {
                    app.s3_server.logs.push(format!("Lỗi khi bật S3: {}", e));
                }
            }
        }
    }
}
