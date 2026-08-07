use crate::app::operations::Operations;
use crate::app::{App, AppEvent, PopupState, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::{Path, PathBuf};

pub async fn handle_explorer_key(app: &mut App, key: KeyEvent) {
    let popup = app.popup_state.clone();

    // 1. Xử lý các Popup nhập liệu/lựa chọn trước
    match popup {
        PopupState::RenameInput { old_name, buffer } => {
            let mut buf = buffer.clone();
            match key.code {
                KeyCode::Esc => app.popup_state = PopupState::None,
                KeyCode::Backspace => {
                    buf.pop();
                    app.popup_state = PopupState::RenameInput { old_name, buffer: buf };
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                    app.popup_state = PopupState::RenameInput { old_name, buffer: buf };
                }
                KeyCode::Enter => {
                    app.popup_state = PopupState::None;
                    let new_name = buf.trim().to_string();
                    if !new_name.is_empty() {
                        if old_name.starts_with("Nhập ") {
                            // Đây là cấu hình máy chủ trong Server Screen
                            if old_name.contains("WebDAV") {
                                if old_name.contains("Username") {
                                    app.webdav_server.user = new_name;
                                } else if old_name.contains("Password") {
                                    app.webdav_server.pass = new_name;
                                } else if old_name.contains("Port") {
                                    app.webdav_server.port = new_name;
                                }
                            } else {
                                if old_name.contains("Access Key") {
                                    app.s3_server.access_key = new_name;
                                } else if old_name.contains("Secret Key") {
                                    app.s3_server.secret_key = new_name;
                                } else if old_name.contains("Port") {
                                    app.s3_server.port = new_name;
                                }
                            }
                            app.current_screen = Screen::Servers;
                        } else {
                            // Đây là đổi tên file/thư mục thực tế
                            app.is_loading = true;
                            let pane = if app.active_pane_left {
                                &mut app.left_pane
                            } else {
                                &mut app.right_pane
                            };
                            let from_path = Path::new(&pane.path).join(&old_name).to_string_lossy().to_string();
                            let to_path = Path::new(&pane.path).join(&new_name).to_string_lossy().to_string();

                            let res = if pane.is_local {
                                std::fs::rename(&from_path, &to_path).map_err(|e| e.to_string())
                            } else {
                                Operations::mv(&app.active_account, &from_path, &to_path).await
                            };

                            if let Err(e) = res {
                                app.popup_state = PopupState::Message {
                                    title: "Lỗi đổi tên".to_string(),
                                    message: e,
                                };
                            } else {
                                app.refresh_active_pane().await;
                            }
                            app.is_loading = false;
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        PopupState::NewFolderInput { buffer } => {
            let mut buf = buffer.clone();
            match key.code {
                KeyCode::Esc => app.popup_state = PopupState::None,
                KeyCode::Backspace => {
                    buf.pop();
                    app.popup_state = PopupState::NewFolderInput { buffer: buf };
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                    app.popup_state = PopupState::NewFolderInput { buffer: buf };
                }
                KeyCode::Enter => {
                    app.popup_state = PopupState::None;
                    let folder_name = buf.trim().to_string();
                    if !folder_name.is_empty() {
                        app.is_loading = true;
                        let pane = if app.active_pane_left {
                            &mut app.left_pane
                        } else {
                            &mut app.right_pane
                        };
                        let full_path = Path::new(&pane.path).join(&folder_name).to_string_lossy().to_string();

                        let res = if pane.is_local {
                            std::fs::create_dir_all(&full_path).map_err(|e| e.to_string())
                        } else {
                            Operations::mkdir(&app.active_account, &full_path).await
                        };

                        if let Err(e) = res {
                            app.popup_state = PopupState::Message {
                                title: "Lỗi tạo thư mục".to_string(),
                                message: e,
                            };
                        } else {
                            app.refresh_active_pane().await;
                        }
                        app.is_loading = false;
                    }
                }
                _ => {}
            }
            return;
        }
        PopupState::ConfirmDelete { name } => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.popup_state = PopupState::None;
                }
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.popup_state = PopupState::None;
                    app.is_loading = true;

                    let is_left = app.active_pane_left;
                    let pane = if is_left {
                        &mut app.left_pane
                    } else {
                        &mut app.right_pane
                    };
                    let full_path = Path::new(&pane.path).join(&name).to_string_lossy().to_string();

                    let res = if pane.is_local {
                        if pane.items[pane.selected_idx].is_dir {
                            std::fs::remove_dir_all(&full_path).map_err(|e| e.to_string())
                        } else {
                            std::fs::remove_file(&full_path).map_err(|e| e.to_string())
                        }
                    } else {
                        Operations::rm(&app.active_account, &full_path, false).await
                    };

                    if let Err(e) = res {
                        app.popup_state = PopupState::Message {
                            title: "Lỗi xóa tệp".to_string(),
                            message: e,
                        };
                    } else {
                        app.refresh_active_pane().await;
                    }
                    app.is_loading = false;
                }
                _ => {}
            }
            return;
        }
        PopupState::ConfirmEmptyTrash => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    app.popup_state = PopupState::None;
                }
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.popup_state = PopupState::None;
                    app.is_loading = true;
                    match Operations::trash_empty(&app.active_account).await {
                        Ok(_) => {
                            app.popup_state = PopupState::Message {
                                title: "Dọn dẹp".to_string(),
                                message: "Đã dọn sạch thùng rác đám mây thành công!".to_string(),
                            };
                        }
                        Err(e) => {
                            app.popup_state = PopupState::Message {
                                title: "Lỗi dọn dẹp".to_string(),
                                message: e,
                            };
                        }
                    }
                    app.is_loading = false;
                }
                _ => {}
            }
            return;
        }
        PopupState::SpecialActionsMenu { selected_idx } => {
            match key.code {
                KeyCode::Esc => app.popup_state = PopupState::None,
                KeyCode::Up => {
                    let next_idx = if selected_idx == 0 { 5 } else { selected_idx - 1 };
                    app.popup_state = PopupState::SpecialActionsMenu { selected_idx: next_idx };
                }
                KeyCode::Down => {
                    let next_idx = (selected_idx + 1) % 6;
                    app.popup_state = PopupState::SpecialActionsMenu { selected_idx: next_idx };
                }
                KeyCode::Enter => {
                    app.popup_state = PopupState::None;
                    trigger_special_action(app, selected_idx).await;
                }
                _ => {}
            }
            return;
        }
        PopupState::ViewFile { name, content, scroll } => {
            match key.code {
                KeyCode::Esc => app.popup_state = PopupState::None,
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
                    let text_to_copy = content.join("\n");
                    let _ = Operations::copy_to_clipboard(&text_to_copy);
                    app.popup_state = PopupState::Message {
                        title: "Sao chép thành công".to_string(),
                        message: "Đã sao chép nội dung tệp vào clipboard hệ thống!".to_string(),
                    };
                }
                KeyCode::Enter => {
                    // Nếu đây là popup thùng rác (Trash List)
                    if name.contains("Thùng rác") {
                        app.is_loading = true;
                        // Khôi phục mục tương ứng: index 1-based bằng scroll + 1
                        let restore_idx = scroll + 1;
                        match Operations::trash_restore(&app.active_account, restore_idx).await {
                            Ok(_) => {
                                // Tải lại danh sách thùng rác để cập nhật giao diện
                                match Operations::list_trash(&app.active_account).await {
                                    Ok(trash_items) => {
                                        if trash_items.is_empty() {
                                            app.popup_state = PopupState::Message {
                                                title: "Thao tác thành công".to_string(),
                                                message: "Khôi phục mục thành công! Thùng rác hiện đã trống.".to_string(),
                                            };
                                        } else {
                                            let lines: Vec<String> = trash_items.iter().enumerate().map(|(i, item)| {
                                                let type_prefix = if item.is_dir { "[DIR]" } else { "[FILE]" };
                                                format!("({}) {} - {} - {}", i + 1, type_prefix, item.name, item.mod_time)
                                            }).collect();
                                            let new_scroll = if scroll >= lines.len() {
                                                if !lines.is_empty() { lines.len() - 1 } else { 0 }
                                            } else {
                                                scroll
                                            };
                                            app.popup_state = PopupState::ViewFile {
                                                name: "Danh sách Thùng rác (Enter: Khôi phục | Delete: Xóa vĩnh viễn)".to_string(),
                                                content: lines,
                                                scroll: new_scroll,
                                            };
                                        }
                                    }
                                    Err(_) => {
                                        app.popup_state = PopupState::None;
                                    }
                                }
                                app.refresh_active_pane().await;
                            }
                            Err(e) => {
                                app.popup_state = PopupState::Message {
                                    title: "Lỗi khôi phục".to_string(),
                                    message: e,
                                };
                            }
                        }
                        app.is_loading = false;
                    }
                }
                KeyCode::Delete
                    // Nếu đây là popup thùng rác (Trash List)
                    if name.contains("Thùng rác") => {
                        app.is_loading = true;
                        // Xóa vĩnh viễn mục tương ứng: index 1-based bằng scroll + 1
                        let delete_idx = scroll + 1;
                        match Operations::trash_delete(&app.active_account, delete_idx).await {
                            Ok(_) => {
                                // Tải lại danh sách thùng rác để cập nhật giao diện
                                match Operations::list_trash(&app.active_account).await {
                                    Ok(trash_items) => {
                                        if trash_items.is_empty() {
                                            app.popup_state = PopupState::Message {
                                                title: "Thao tác thành công".to_string(),
                                                message: "Đã xóa vĩnh viễn mục được chọn! Thùng rác hiện đã trống.".to_string(),
                                            };
                                        } else {
                                            let lines: Vec<String> = trash_items.iter().enumerate().map(|(i, item)| {
                                                let type_prefix = if item.is_dir { "[DIR]" } else { "[FILE]" };
                                                format!("({}) {} - {} - {}", i + 1, type_prefix, item.name, item.mod_time)
                                            }).collect();
                                            let new_scroll = if scroll >= lines.len() {
                                                if !lines.is_empty() { lines.len() - 1 } else { 0 }
                                            } else {
                                                scroll
                                            };
                                            app.popup_state = PopupState::ViewFile {
                                                name: "Danh sách Thùng rác (Enter: Khôi phục | Delete: Xóa vĩnh viễn)".to_string(),
                                                content: lines,
                                                scroll: new_scroll,
                                            };
                                        }
                                    }
                                    Err(_) => {
                                        app.popup_state = PopupState::None;
                                    }
                                }
                                app.refresh_active_pane().await;
                            }
                            Err(e) => {
                                app.popup_state = PopupState::Message {
                                    title: "Lỗi xóa vĩnh viễn".to_string(),
                                    message: e,
                                };
                            }
                        }
                        app.is_loading = false;
                    }
                _ => {}
            }
            return;
        }
        PopupState::SwitchAccountMenu { selected_idx } => {
            match key.code {
                KeyCode::Esc => app.popup_state = PopupState::None,
                KeyCode::Up => {
                    let next_idx = if selected_idx == 0 {
                        app.accounts.len() - 1
                    } else {
                        selected_idx - 1
                    };
                    app.popup_state = PopupState::SwitchAccountMenu { selected_idx: next_idx };
                }
                KeyCode::Down => {
                    let next_idx = (selected_idx + 1) % app.accounts.len();
                    app.popup_state = PopupState::SwitchAccountMenu { selected_idx: next_idx };
                }
                KeyCode::Enter => {
                    app.popup_state = PopupState::None;
                    if selected_idx < app.accounts.len() {
                        app.active_account_idx = selected_idx;
                        app.active_account = Some(app.accounts[selected_idx].clone());
                    }

                    app.is_loading = true;
                    if !app.left_pane.is_local {
                        app.left_pane.path = "/".to_string();
                        app.left_pane.selected_idx = 0;
                        app.left_pane.scroll_offset = 0;
                        app.left_pane.selected_names.clear();
                    }
                    if !app.right_pane.is_local {
                        app.right_pane.path = "/".to_string();
                        app.right_pane.selected_idx = 0;
                        app.right_pane.scroll_offset = 0;
                        app.right_pane.selected_names.clear();
                    }
                    app.refresh_active_pane().await;
                    app.active_pane_left = !app.active_pane_left;
                    app.refresh_active_pane().await;
                    app.active_pane_left = !app.active_pane_left;
                    app.is_loading = false;
                }
                _ => {}
            }
            return;
        }
        _ => {}
    }

    // 2. Xử lý các phím tắt chính ở màn hình Explorer
    if key.code == KeyCode::Esc {
        app.current_screen = Screen::MainMenu;
        return;
    }

    match key.code {
        KeyCode::Tab => {
            app.active_pane_left = !app.active_pane_left;
        }
        KeyCode::Up => {
            app.select_prev();
        }
        KeyCode::Down => {
            app.select_next();
        }
        KeyCode::Backspace => {
            // Quay lại thư mục cha
            let pane = if app.active_pane_left {
                &mut app.left_pane
            } else {
                &mut app.right_pane
            };
            let current = Path::new(&pane.path);
            if let Some(parent) = current.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if !parent_str.is_empty() {
                    pane.path = parent_str;
                    pane.selected_idx = 0;
                    pane.scroll_offset = 0;
                    pane.selected_names.clear();
                    app.refresh_active_pane().await;
                }
            }
        }
        KeyCode::Enter => {
            // Vào thư mục hoặc mở/xem file
            let is_left = app.active_pane_left;
            let (is_dir, name) = {
                let pane = if is_left { &app.left_pane } else { &app.right_pane };
                if pane.items.is_empty() {
                    (false, String::new())
                } else {
                    let item = &pane.items[pane.selected_idx];
                    (item.is_dir, item.name.clone())
                }
            };

            if !name.is_empty() {
                let pane = if is_left {
                    &mut app.left_pane
                } else {
                    &mut app.right_pane
                };
                if name == ".." {
                    let current = Path::new(&pane.path);
                    if let Some(parent) = current.parent() {
                        let parent_str = parent.to_string_lossy().to_string();
                        if !parent_str.is_empty() {
                            pane.path = parent_str;
                            pane.selected_idx = 0;
                            pane.scroll_offset = 0;
                            pane.selected_names.clear();
                            app.refresh_active_pane().await;
                        }
                    }
                } else if is_dir {
                    pane.path = Path::new(&pane.path).join(&name).to_string_lossy().to_string();
                    pane.selected_idx = 0;
                    pane.scroll_offset = 0;
                    pane.selected_names.clear();
                    app.refresh_active_pane().await;
                } else {
                    // Xem nội dung tệp text
                    app.is_loading = true;
                    let full_path = Path::new(&pane.path).join(&name).to_string_lossy().to_string();
                    let res = if pane.is_local {
                        std::fs::read_to_string(&full_path).map_err(|e| e.to_string())
                    } else {
                        Operations::cat(&app.active_account, &full_path).await
                    };
                    match res {
                        Ok(content) => {
                            app.popup_state = PopupState::ViewFile {
                                name: format!("Xem file: {}", name),
                                content: content.lines().map(|s| s.to_string()).collect(),
                                scroll: 0,
                            };
                        }
                        Err(e) => {
                            app.popup_state = PopupState::Message {
                                title: "Lỗi xem file".to_string(),
                                message: e,
                            };
                        }
                    }
                    app.is_loading = false;
                }
            }
        }
        KeyCode::Delete => {
            // Xóa file/thư mục
            let pane = if app.active_pane_left {
                &app.left_pane
            } else {
                &app.right_pane
            };
            if !pane.items.is_empty() {
                let name = pane.items[pane.selected_idx].name.clone();
                if name != ".." {
                    app.popup_state = PopupState::ConfirmDelete { name };
                }
            }
        }
        KeyCode::Char(' ') => {
            // Dọn sạch selection và clipboard
            app.clipboard.clear();
            let pane = if app.active_pane_left {
                &mut app.left_pane
            } else {
                &mut app.right_pane
            };
            pane.selected_names.clear();
            pane.shift_anchor = None;
            pane.shift_active = false;
        }
        _ => {}
    }

    // Xử lý các tổ hợp phím CTRL+ (Sao chép, dán, di chuyển, chọn tất cả)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('a') | KeyCode::Char('A') => {
                // Ctrl+A: Chọn tất cả (loại trừ "..")
                let pane = if app.active_pane_left { &mut app.left_pane } else { &mut app.right_pane };
                let selectable_count = pane.items.iter().filter(|i| i.name != "..").count();
                if pane.selected_names.len() == selectable_count {
                    pane.selected_names.clear();
                } else {
                    for item in &pane.items {
                        if item.name != ".." {
                            pane.selected_names.insert(item.name.clone());
                        }
                    }
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Ctrl+C: Copy vào clipboard
                let pane = if app.active_pane_left { &app.left_pane } else { &app.right_pane };
                app.clipboard.clear();
                app.clipboard_src_path = pane.path.clone();
                app.clipboard_src_is_local = pane.is_local;
                app.clipboard_src_account = app.active_account.clone();
                app.clipboard_is_cut = false;

                if !pane.selected_names.is_empty() {
                    for item in &pane.items {
                        if pane.selected_names.contains(&item.name) {
                            app.clipboard.push(item.clone());
                        }
                    }
                } else if !pane.items.is_empty() {
                    let name = &pane.items[pane.selected_idx].name;
                    if name != ".." {
                        app.clipboard.push(pane.items[pane.selected_idx].clone());
                    }
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                // Ctrl+X: Cut
                let pane = if app.active_pane_left { &app.left_pane } else { &app.right_pane };
                app.clipboard.clear();
                app.clipboard_src_path = pane.path.clone();
                app.clipboard_src_is_local = pane.is_local;
                app.clipboard_src_account = app.active_account.clone();
                app.clipboard_is_cut = true;

                if !pane.selected_names.is_empty() {
                    for item in &pane.items {
                        if pane.selected_names.contains(&item.name) {
                            app.clipboard.push(item.clone());
                        }
                    }
                } else if !pane.items.is_empty() {
                    let name = &pane.items[pane.selected_idx].name;
                    if name != ".." {
                        app.clipboard.push(pane.items[pane.selected_idx].clone());
                    }
                }
            }
            KeyCode::Char('v') | KeyCode::Char('V')
                // Ctrl+V: Paste (Thực hiện upload/download hoặc sao chép nội bộ)
                if !app.clipboard.is_empty() => {
                    app.is_loading = true;
                    let dest_is_local = if app.active_pane_left { app.left_pane.is_local } else { app.right_pane.is_local };
                    let dest_path = if app.active_pane_left { app.left_pane.path.clone() } else { app.right_pane.path.clone() };

                    let mut error_occurred = false;
                    let mut error_msg = String::new();

                    for item in app.clipboard.clone() {
                        let src_full = Path::new(&app.clipboard_src_path).join(&item.name).to_string_lossy().to_string();
                        let dest_full = Path::new(&dest_path).join(&item.name).to_string_lossy().to_string();

                        let res = if app.clipboard_src_is_local && !dest_is_local {
                            // Local -> Cloud (Upload)
                            Operations::upload(&app.active_account, &src_full, &dest_path).await
                        } else if !app.clipboard_src_is_local && dest_is_local {
                            // Cloud -> Local (Download)
                            Operations::download(&app.clipboard_src_account, &src_full, &dest_full).await
                        } else if !app.clipboard_src_is_local && !dest_is_local {
                            // Cloud -> Cloud (Copy/Move)
                            if app.clipboard_is_cut {
                                Operations::mv(&app.active_account, &src_full, &dest_full).await
                            } else {
                                Operations::cp(&app.active_account, &src_full, &dest_full).await
                            }
                        } else {
                            // Local -> Local
                            if app.clipboard_is_cut {
                                std::fs::rename(&src_full, &dest_full).map_err(|e| e.to_string())
                            } else {
                                std::fs::copy(&src_full, &dest_full).map(|_| ()).map_err(|e| e.to_string())
                            }
                        };

                        if let Err(e) = res {
                            error_occurred = true;
                            error_msg = e;
                            break;
                        }
                    }

                    if error_occurred {
                        app.popup_state = PopupState::Message {
                            title: "Lỗi dán tệp".to_string(),
                            message: error_msg,
                        };
                    } else {
                        app.refresh_active_pane().await;
                        app.active_pane_left = !app.active_pane_left;
                        app.refresh_active_pane().await;
                        app.active_pane_left = !app.active_pane_left;
                    }

                    if app.clipboard_is_cut {
                        app.clipboard.clear();
                    }
                    app.is_loading = false;
                }
            _ => {}
        }
    }

    // Xử lý các tổ hợp phím ALT+
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Alt+R: Đổi nguồn giữa Local/Remote
                let pane = if app.active_pane_left {
                    &mut app.left_pane
                } else {
                    &mut app.right_pane
                };
                pane.is_local = !pane.is_local;
                pane.path = if pane.is_local {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("/"))
                        .to_string_lossy()
                        .to_string()
                } else {
                    "/".to_string()
                };
                pane.selected_idx = 0;
                pane.scroll_offset = 0;
                pane.selected_names.clear();
                app.refresh_active_pane().await;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Alt+N: Tạo thư mục mới
                app.popup_state = PopupState::NewFolderInput { buffer: String::new() };
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Alt+Y: Đổi tên
                let pane = if app.active_pane_left {
                    &app.left_pane
                } else {
                    &app.right_pane
                };
                if !pane.items.is_empty() {
                    let name = pane.items[pane.selected_idx].name.clone();
                    if name != ".." {
                        app.popup_state = PopupState::RenameInput {
                            old_name: name.clone(),
                            buffer: name,
                        };
                    }
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                // Alt+T: Đồng bộ (Sync) hai thư mục hiện tại của Pane Trái và Phải
                trigger_sync(app).await;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                // Alt+V: Chọn mục đơn lẻ bằng cách đánh dấu tích
                let pane = if app.active_pane_left {
                    &mut app.left_pane
                } else {
                    &mut app.right_pane
                };
                if !pane.items.is_empty() {
                    let name = pane.items[pane.selected_idx].name.clone();
                    if name != ".." {
                        if pane.selected_names.contains(&name) {
                            pane.selected_names.remove(&name);
                        } else {
                            pane.selected_names.insert(name);
                        }
                    }
                }
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Alt+O: Mở Menu Đặc biệt
                let pane = if app.active_pane_left {
                    &app.left_pane
                } else {
                    &app.right_pane
                };
                if !pane.items.is_empty() {
                    let name = pane.items[pane.selected_idx].name.clone();
                    if name != ".." {
                        app.popup_state = PopupState::SpecialActionsMenu { selected_idx: 0 };
                    }
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // Alt+S: Đổi tài khoản hoạt động trực tiếp trong Explorer
                app.popup_state = PopupState::SwitchAccountMenu {
                    selected_idx: app.active_account_idx,
                };
            }
            _ => {}
        }
    }

    // Xử lý Shift+V (Bôi đen chọn vùng)
    if key.modifiers.contains(KeyModifiers::SHIFT)
        && !key.modifiers.contains(KeyModifiers::CONTROL)
        && !key.modifiers.contains(KeyModifiers::ALT)
        && (key.code == KeyCode::Char('v') || key.code == KeyCode::Char('V'))
    {
        let pane = if app.active_pane_left {
            &mut app.left_pane
        } else {
            &mut app.right_pane
        };
        if let Some(anchor) = pane.shift_anchor {
            if anchor == pane.selected_idx {
                // Lần nhấn thứ 3 hoặc nhấn tại chỗ: Hủy bỏ neo
                pane.shift_anchor = None;
                pane.shift_active = false;
            } else if !pane.shift_active {
                // Lần nhấn thứ 2: Toggle từ anchor đến vị trí hiện tại
                let start = anchor.min(pane.selected_idx);
                let end = anchor.max(pane.selected_idx);
                for i in start..=end {
                    if i < pane.items.len() && pane.items[i].name != ".." {
                        let name = pane.items[i].name.clone();
                        if pane.selected_names.contains(&name) {
                            pane.selected_names.remove(&name);
                        } else {
                            pane.selected_names.insert(name);
                        }
                    }
                }
                pane.shift_active = true;
            } else {
                // Lần nhấn thứ 3: Hủy bỏ neo
                pane.shift_anchor = None;
                pane.shift_active = false;
            }
        } else {
            // Lần nhấn thứ 1: Đặt anchor
            pane.shift_anchor = Some(pane.selected_idx);
            pane.shift_active = false;
        }
    }
}

// Xử lý các chức năng phụ trong menu đặc biệt Alt+O
async fn trigger_special_action(app: &mut App, selected_idx: usize) {
    let is_left = app.active_pane_left;
    let pane = if is_left { &app.left_pane } else { &app.right_pane };
    if pane.items.is_empty() {
        return;
    }
    let selected_item_name = pane.items[pane.selected_idx].name.clone();
    let full_path = Path::new(&pane.path)
        .join(&selected_item_name)
        .to_string_lossy()
        .to_string();

    match selected_idx {
        0 => {
            // 🔗 Tạo Link Tải Công Khai
            if pane.is_local {
                app.popup_state = PopupState::Message {
                    title: "Không hỗ trợ".to_string(),
                    message: "Không thể tạo link công khai cho tệp cục bộ (Local).".to_string(),
                };
                return;
            }
            app.is_loading = true;
            match Operations::create_link(&app.active_account, &full_path).await {
                Ok(url) => {
                    app.popup_state = PopupState::Message {
                        title: "Link tải công khai".to_string(),
                        message: format!("Đã tạo thành công:\n{}", url),
                    };
                }
                Err(e) => {
                    app.popup_state = PopupState::Message {
                        title: "Lỗi tạo link".to_string(),
                        message: e,
                    };
                }
            }
            app.is_loading = false;
        }
        1 => {
            // ⭐ Thêm/Bỏ Yêu Thích
            if pane.is_local {
                app.popup_state = PopupState::Message {
                    title: "Không hỗ trợ".to_string(),
                    message: "Không thể yêu thích tệp cục bộ (Local).".to_string(),
                };
                return;
            }
            app.is_loading = true;
            // Ở đây chúng ta chạy yêu thích
            match Operations::favorite(&app.active_account, &full_path).await {
                Ok(_) => {
                    app.popup_state = PopupState::Message {
                        title: "Yêu thích".to_string(),
                        message: format!("Đã yêu thích thành công: {}", selected_item_name),
                    };
                }
                Err(e) => {
                    // Nếu lỗi có thể đã yêu thích trước đó, thử unfavorite
                    if Operations::unfavorite(&app.active_account, &full_path).await.is_ok() {
                        app.popup_state = PopupState::Message {
                            title: "Yêu thích".to_string(),
                            message: format!("Đã bỏ yêu thích thành công: {}", selected_item_name),
                        };
                    } else {
                        app.popup_state = PopupState::Message {
                            title: "Lỗi Yêu thích".to_string(),
                            message: e,
                        };
                    }
                }
            }
            app.is_loading = false;
        }
        2 => {
            // 🗑️ Khôi Phục từ Thùng Rác (Liệt kê danh sách và cho phép chọn)
            if pane.is_local {
                app.popup_state = PopupState::Message {
                    title: "Không hỗ trợ".to_string(),
                    message: "Thùng rác chỉ khả dụng đối với ổ đĩa đám mây.".to_string(),
                };
                return;
            }
            app.is_loading = true;
            match Operations::list_trash(&app.active_account).await {
                Ok(trash_items) => {
                    if trash_items.is_empty() {
                        app.popup_state = PopupState::Message {
                            title: "Thùng rác".to_string(),
                            message: "Thùng rác trống!".to_string(),
                        };
                    } else {
                        let lines = trash_items
                            .iter()
                            .enumerate()
                            .map(|(i, item)| {
                                let type_prefix = if item.is_dir { "[DIR]" } else { "[FILE]" };
                                format!("({}) {} - {} - {}", i + 1, type_prefix, item.name, item.mod_time)
                            })
                            .collect();
                        app.popup_state = PopupState::ViewFile {
                            name: "Danh sách Thùng rác (Enter: Khôi phục | Delete: Xóa vĩnh viễn)".to_string(),
                            content: lines,
                            scroll: 0,
                        };
                    }
                }
                Err(e) => {
                    app.popup_state = PopupState::Message {
                        title: "Lỗi đọc Thùng rác".to_string(),
                        message: e,
                    };
                }
            }
            app.is_loading = false;
        }
        3 => {
            // 🧹 Dọn Dẹp Thùng Rác
            if pane.is_local {
                app.popup_state = PopupState::Message {
                    title: "Không hỗ trợ".to_string(),
                    message: "Thùng rác chỉ khả dụng đối với ổ đĩa đám mây.".to_string(),
                };
                return;
            }
            app.popup_state = PopupState::ConfirmEmptyTrash;
        }
        4 => {
            // 🔍 Xem Siêu Dữ Liệu (Stat)
            app.is_loading = true;
            let res = if pane.is_local {
                match std::fs::metadata(&full_path) {
                    Ok(meta) => {
                        let s = format!(
                            "File: {}\nSize: {} bytes\nType: {}\nModified: {:?}",
                            selected_item_name,
                            meta.len(),
                            if meta.is_dir() { "directory" } else { "file" },
                            meta.modified()
                        );
                        Ok(s)
                    }
                    Err(e) => Err(e.to_string()),
                }
            } else {
                let mut cmd = Operations::get_command(&app.active_account);
                cmd.arg("stat").arg(&full_path);
                let out = cmd.output().await;
                match out {
                    Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).to_string()),
                    Ok(o) => Err(String::from_utf8_lossy(&o.stderr).to_string()),
                    Err(e) => Err(e.to_string()),
                }
            };

            match res {
                Ok(text) => {
                    app.popup_state = PopupState::ViewFile {
                        name: format!("Thông tin Chi tiết: {}", selected_item_name),
                        content: text.lines().map(|s| s.to_string()).collect(),
                        scroll: 0,
                    };
                }
                Err(e) => {
                    app.popup_state = PopupState::Message {
                        title: "Lỗi xem Stat".to_string(),
                        message: e,
                    };
                }
            }
            app.is_loading = false;
        }
        _ => {}
    }
}

pub async fn trigger_sync(app: &mut App) {
    app.is_loading = true;
    let local_path = if app.left_pane.is_local {
        &app.left_pane.path
    } else {
        &app.right_pane.path
    };
    let remote_path = if !app.left_pane.is_local {
        &app.left_pane.path
    } else {
        &app.right_pane.path
    };

    let pair = format!("{}:{}", local_path, remote_path);
    let mut cmd = Operations::get_command(&app.active_account);
    cmd.arg("sync").arg(pair);

    app.popup_state = PopupState::Message {
        title: "Chạy đồng bộ".to_string(),
        message: format!("Đang chạy lệnh đồng bộ ngầm: filen sync {}\nXin hãy đợi...", local_path),
    };

    let tx = app.msg_tx.clone();
    tokio::spawn(async move {
        let out = cmd.output().await;
        if let Some(tx) = tx {
            match out {
                Ok(output) if output.status.success() => {
                    let _ = tx.send(AppEvent::AsyncFinished(Ok(())));
                }
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    let _ = tx.send(AppEvent::AsyncFinished(Err(err)));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::AsyncFinished(Err(e.to_string())));
                }
            }
        }
    });
    app.is_loading = false;
}
