use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::config::{AppStatus, InstallType};
use crate::tui::app::{App, Screen};
use crate::manager;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Base layout: Header (3), Main (Min 10), Status Message (3), Footer (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main Area
            Constraint::Length(3), // Status Message
            Constraint::Length(1), // Footer Help
        ])
        .split(size);

    // 1. Draw Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Universe Manager ");
    
    let header_text = vec![
        Line::from(vec![
            Span::styled("Hệ thống quản lý ứng dụng đa năng ", Style::default().fg(Color::White)),
            Span::styled(format!("(Tổng số app: {})", app.apps_with_status.len()), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])
    ];
    let header_para = Paragraph::new(header_text)
        .block(header_block)
        .alignment(Alignment::Center);
    f.render_widget(header_para, chunks[0]);

    // 2. Draw Main Area based on Screen State
    match app.current_screen {
        Screen::MainMenu => {
            draw_main_menu(f, app, chunks[1]);
        }
        Screen::AppList => {
            let snapshot = app.get_process_snapshot();
            draw_app_list(f, app, &snapshot, chunks[1]);
        }
        Screen::AppOperations => {
            draw_app_operations(f, app, chunks[1]);
        }
        Screen::UninstallList => {
            draw_uninstall_list(f, app, chunks[1]);
        }
    }

    // 3. Draw Status Bar
    let status_block = Block::default()
        .borders(Borders::ALL)
        .title(" Nhật ký hoạt động ")
        .border_style(Style::default().fg(Color::DarkGray));
    
    let status_str = app.status_message.as_deref().unwrap_or("Đang chờ lệnh từ người dùng...");
    let status_para = Paragraph::new(status_str)
        .style(Style::default().fg(Color::LightGreen))
        .block(status_block);
    f.render_widget(status_para, chunks[2]);

    // 4. Draw Footer Help
    let footer_text = match app.current_screen {
        Screen::MainMenu => Line::from(vec![
            Span::styled(" Phím tắt: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("↑↓/j/k: Chọn chức năng", Style::default().fg(Color::LightBlue)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter: Vào mục đã chọn", Style::default().fg(Color::LightGreen)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("q/Esc: Thoát chương trình", Style::default().fg(Color::LightRed)),
        ]),
        Screen::AppList => Line::from(vec![
            Span::styled(" Phím tắt: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("↑↓/j/k: Duyệt danh sách", Style::default().fg(Color::LightBlue)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space: Tích chọn checkbox [x]", Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Insert: Tiến vào màn hình Thao Tác", Style::default().fg(Color::LightGreen)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Backspace/Esc: Quay lại Menu chính", Style::default().fg(Color::LightRed)),
        ]),
        Screen::AppOperations => Line::from(vec![
            Span::styled(" Phím tắt: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("↑↓/j/k: Chọn hành động", Style::default().fg(Color::LightBlue)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space: Chọn hành động để thực thi [x]", Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Insert/Enter: Thực thi tất cả các hành động đã chọn", Style::default().fg(Color::LightGreen)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Backspace/Esc: Quay lại danh sách app", Style::default().fg(Color::LightRed)),
        ]),
        Screen::UninstallList => Line::from(vec![
            Span::styled(" Phím tắt: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("↑↓/j/k: Duyệt danh sách", Style::default().fg(Color::LightBlue)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space: Tích chọn ứng dụng cần gỡ [x]", Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Insert/Enter: Tiến hành gỡ cài đặt hoàn toàn", Style::default().fg(Color::LightGreen)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("Backspace/Esc: Quay lại Menu chính", Style::default().fg(Color::LightRed)),
        ]),
    };
    let footer_para = Paragraph::new(footer_text).alignment(Alignment::Center);
    f.render_widget(footer_para, chunks[3]);
}

fn draw_main_menu(f: &mut Frame, app: &App, area: Rect) {
    let menu_items = vec![
        " 1. Quản lý ứng dụng (Bật/Tắt/Khởi động lại/Tự khởi động) ",
        " 2. Tích hợp / Chuyển đổi / Cập nhật Portable ",
        " 3. Gỡ cài đặt ứng dụng (Xoá shortcut & dữ liệu) ",
        " 4. Kiểm tra Cập nhật hệ thống (APT, Flatpak, Snap) ",
        " 5. Dọn dẹp Leftovers (Purge APT config & Flatpak unused) ",
        " 6. Thoát chương trình ",
    ];

    let items: Vec<ListItem> = menu_items.iter().enumerate().map(|(i, &item)| {
        let style = if i == app.menu_index {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        ListItem::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(if i == app.menu_index { "  >> " } else { "     " }, Style::default().fg(Color::Cyan)),
                Span::styled(item, style),
            ]),
            Line::from(""),
        ])
    }).collect();

    let menu_block = Block::default()
        .borders(Borders::ALL)
        .title(" MENU CHÍNH ")
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::LightBlue));

    let list = List::new(items)
        .block(menu_block);
    f.render_widget(list, area);
}

fn draw_app_list(f: &mut Frame, app: &mut App, process_snapshot: &manager::ProcessSnapshot, area: Rect) {

    // Left: 60%, Right: 40%
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    // Left Pane: Table of Apps with Checkboxes
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(" Danh sách ứng dụng ")
        .border_style(Style::default().fg(Color::DarkGray));

    if app.apps_with_status.is_empty() {
        let empty_msg = Paragraph::new("\n Không có ứng dụng nào.\n Quay lại Menu chọn mục 2 để thêm mới.")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(list_block);
        f.render_widget(empty_msg, main_chunks[0]);
    } else {
        let header = Row::new(vec![
            Cell::from(" Chọn"),
            Cell::from("Tên ứng dụng"),
            Cell::from("Chuyên mục"),
            Cell::from("Nguồn"),
            Cell::from("Trạng thái"),
        ])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = app.apps_with_status.iter().enumerate().map(|(i, (entry, _status))| {
            let is_checked = app.checked_app_ids.contains(&entry.id);
            let check_str = if is_checked { " [x]" } else { " [ ]" };
            let check_style = if is_checked { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) };

            let is_running = process_snapshot.is_running(entry);
            let run_status = if is_running { "RUNNING" } else { "STOPPED" };
            let run_style = if is_running { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };

            let highlight_style = if i == app.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let category_str = entry.category.as_deref().unwrap_or("Other");
            let category_style = if i == app.selected_index {
                Style::default().bg(Color::Blue).fg(Color::Rgb(244, 164, 96)).add_modifier(Modifier::BOLD) // SandyBrown on Blue
            } else {
                Style::default().fg(Color::Rgb(210, 105, 30)) // Chocolate brown
            };

            let source_str = entry.package_type.as_deref().unwrap_or("Local");
            let source_style = if i == app.selected_index {
                match source_str {
                    "APT" => Style::default().bg(Color::Blue).fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    "Flatpak" => Style::default().bg(Color::Blue).fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    "Snap" => Style::default().bg(Color::Blue).fg(Color::LightYellow).add_modifier(Modifier::BOLD),
                    _ => Style::default().bg(Color::Blue).fg(Color::Green).add_modifier(Modifier::BOLD),
                }
            } else {
                match source_str {
                    "APT" => Style::default().fg(Color::Cyan),
                    "Flatpak" => Style::default().fg(Color::Magenta),
                    "Snap" => Style::default().fg(Color::LightYellow),
                    _ => Style::default().fg(Color::Green),
                }
            };

            Row::new(vec![
                Cell::from(check_str).style(check_style),
                Cell::from(entry.name.clone()).style(highlight_style),
                Cell::from(category_str).style(category_style),
                Cell::from(source_str).style(source_style),
                Cell::from(run_status).style(if i == app.selected_index { run_style.bg(Color::Blue).add_modifier(Modifier::BOLD) } else { run_style }),
            ])
        }).collect();

        let widths = [
            Constraint::Length(6),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(9),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(list_block)
            .column_spacing(1);
        app.app_list_state.select(Some(app.selected_index));
        f.render_stateful_widget(table, main_chunks[0], &mut app.app_list_state);
    }

    // Right Pane: Detail View
    let details_block = Block::default()
        .borders(Borders::ALL)
        .title(" Chi tiết ứng dụng ")
        .border_style(Style::default().fg(Color::DarkGray));

    if let Some((entry, status)) = app.apps_with_status.get(app.selected_index) {
        let is_running = process_snapshot.is_running(entry);
        let autostart_enabled = manager::is_autostart_enabled(entry);
        
        let install_type_str = match entry.install_type {
            InstallType::InPlace => "Tại chỗ (In-Place / Giữ nguyên thư mục gốc)",
            InstallType::Moved => "Đã chuyển (Moved / Lưu tại thư mục quản lý tập trung)",
        };

        let mut details_text = vec![
            Line::from(vec![
                Span::styled(" Tên ứng dụng:       ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&entry.name, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" App ID:             ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&entry.id, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" Trạng thái chạy:    ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(if is_running { "ĐANG CHẠY (RUNNING)" } else { "ĐÃ DỪNG (STOPPED)" }, Style::default().fg(if is_running { Color::Green } else { Color::Red }).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Khởi động hệ thống: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(if autostart_enabled { "ĐÃ BẬT (ENABLED)" } else { "ĐÃ TẮT (DISABLED)" }, Style::default().fg(if autostart_enabled { Color::Yellow } else { Color::DarkGray }).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Kiểu cài đặt:       ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(install_type_str, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" File chạy chính:    ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&entry.exec_path, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" Thư mục lưu:        ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&entry.install_path, Style::default().fg(Color::White)),
            ]),
        ];

        if entry.is_custom.unwrap_or(false) {
            details_text.push(Line::from(""));
            details_text.push(Line::from(vec![
                Span::styled(" [Custom Command Settings]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            if let Some(ref start) = entry.start_cmd {
                details_text.push(Line::from(vec![
                    Span::styled("   Lệnh Start:       ", Style::default().fg(Color::Cyan)),
                    Span::styled(start, Style::default().fg(Color::LightYellow)),
                ]));
            }
            if let Some(ref stop) = entry.stop_cmd {
                details_text.push(Line::from(vec![
                    Span::styled("   Lệnh Stop:        ", Style::default().fg(Color::Cyan)),
                    Span::styled(stop, Style::default().fg(Color::LightYellow)),
                ]));
            }
        }

        details_text.push(Line::from(""));
        details_text.push(Line::from("-----------------------------------------------------------------"));
        details_text.push(Line::from(""));

        // Render Configuration & Data Paths
        let paths = manager::get_app_paths(entry);
        details_text.push(Line::from(vec![
            Span::styled(" THƯ MỤC CẤU HÌNH & DỮ LIỆU: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        
        if let Some(ref path) = paths.config_dir {
            details_text.push(Line::from(vec![
                Span::styled("  • Cấu hình (Config): ", Style::default().fg(Color::Cyan)),
                Span::styled(path, Style::default().fg(Color::White)),
            ]));
        }
        if let Some(ref path) = paths.data_dir {
            details_text.push(Line::from(vec![
                Span::styled("  • Dữ liệu (Data):     ", Style::default().fg(Color::Cyan)),
                Span::styled(path, Style::default().fg(Color::White)),
            ]));
        }
        if let Some(ref path) = paths.cache_dir {
            details_text.push(Line::from(vec![
                Span::styled("  • Bộ nhớ đệm (Cache): ", Style::default().fg(Color::Cyan)),
                Span::styled(path, Style::default().fg(Color::White)),
            ]));
        }
        if let Some(ref path) = paths.system_share_dir {
            details_text.push(Line::from(vec![
                Span::styled("  • Tài nguyên (Share): ", Style::default().fg(Color::Cyan)),
                Span::styled(path, Style::default().fg(Color::White)),
            ]));
        }

        details_text.push(Line::from(""));
        details_text.push(Line::from("-----------------------------------------------------------------"));
        details_text.push(Line::from(""));

        // Render Integrity Check Results
        details_text.push(Line::from(vec![
            Span::styled(" TÌNH TRẠNG LIÊN KẾT HỆ THỐNG: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));

        match status {
            AppStatus::Healthy => {
                details_text.push(Line::from(vec![
                    Span::styled("  [OK] Xanh lá - launcher và đường dẫn command line liên kết tốt.", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]));
            }
            AppStatus::Degraded(issues) => {
                details_text.push(Line::from(vec![
                    Span::styled("  [WARNING] Vàng - Lỗi nhẹ hoặc thiếu shortcut:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]));
                for issue in issues {
                    details_text.push(Line::from(vec![
                        Span::styled("   • ", Style::default().fg(Color::Yellow)),
                        Span::styled(issue, Style::default().fg(Color::White)),
                    ]));
                }
            }
            AppStatus::Broken(issues) => {
                details_text.push(Line::from(vec![
                    Span::styled("  [CRITICAL] Đỏ - Thiếu file chạy gốc hoặc thư mục cài đặt:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                ]));
                for issue in issues {
                    details_text.push(Line::from(vec![
                        Span::styled("   ✖ ", Style::default().fg(Color::Red)),
                        Span::styled(issue, Style::default().fg(Color::White)),
                    ]));
                }
            }
        }

        let paragraph = Paragraph::new(details_text)
            .block(details_block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, main_chunks[1]);
    } else {
        let empty_details = Paragraph::new("\n Chọn ứng dụng bên trái để xem chi tiết.")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(details_block);
        f.render_widget(empty_details, main_chunks[1]);
    }
}

fn draw_app_operations(f: &mut Frame, app: &App, area: Rect) {
    // Left: 50% Actions, Right: 50% Selected Apps
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left Pane: Checklist of actions
    let action_block = Block::default()
        .borders(Borders::ALL)
        .title(" Chọn thao tác cần thực thi ")
        .border_style(Style::default().fg(Color::Cyan));

    let action_list = vec![
        " Bắt đầu chạy (Start) ",
        " Dừng chạy (Stop / Kill) ",
        " Khởi động lại (Restart) ",
        " Bật/Tắt khởi động cùng hệ thống (Toggle Autostart) ",
    ];

    let items: Vec<ListItem> = action_list.iter().enumerate().map(|(i, &action)| {
        let is_checked = app.checked_operations.contains(&i);
        let check_icon = if is_checked { "[x] " } else { "[ ] " };
        let check_style = if is_checked { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) };

        let highlight_style = if i == app.operations_index {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        ListItem::new(Line::from(vec![
            Span::styled(check_icon, check_style),
            Span::styled(action, highlight_style),
        ]))
    }).collect();

    let list = List::new(items)
        .block(action_block);
    f.render_widget(list, main_chunks[0]);

    // Right Pane: List of checked apps to apply to
    let app_block = Block::default()
        .borders(Borders::ALL)
        .title(" Áp dụng cho các ứng dụng đã tích ")
        .border_style(Style::default().fg(Color::DarkGray));

    let mut selected_apps_text = vec![
        Line::from(vec![
            Span::styled(" Danh sách ứng dụng sẽ nhận lệnh:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
    ];

    for (entry, _) in &app.apps_with_status {
        if app.checked_app_ids.contains(&entry.id) {
            selected_apps_text.push(Line::from(vec![
                Span::styled("   • ", Style::default().fg(Color::Cyan)),
                Span::styled(&entry.name, Style::default().fg(Color::White)),
                Span::styled(format!(" ({})", entry.id), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    selected_apps_text.push(Line::from(""));
    selected_apps_text.push(Line::from("---------------------------------------------"));
    selected_apps_text.push(Line::from(vec![
        Span::styled(" Nhấn [Space] để tick chọn Hành động cần chạy.", Style::default().fg(Color::Yellow)),
    ]));
    selected_apps_text.push(Line::from(vec![
        Span::styled(" Nhấn [Insert] hoặc [Enter] để thực thi ngay.", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]));
    selected_apps_text.push(Line::from(vec![
        Span::styled(" Nhấn [Backspace] để quay lại.", Style::default().fg(Color::LightRed)),
    ]));

    let paragraph = Paragraph::new(selected_apps_text)
        .block(app_block)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, main_chunks[1]);
}

fn draw_uninstall_list(f: &mut Frame, app: &mut App, area: Rect) {
    // Left: 55% App Table, Right: 45% Safe Warnings
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ])
        .split(area);

    // Left Pane: Table of Apps with Checkboxes
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(" Chọn ứng dụng muốn gỡ bỏ hoàn toàn ")
        .border_style(Style::default().fg(Color::Red));

    if app.apps_with_status.is_empty() {
        let empty_msg = Paragraph::new("\n Không có ứng dụng nào.")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(list_block);
        f.render_widget(empty_msg, main_chunks[0]);
    } else {
        let header = Row::new(vec![
            Cell::from(" Chọn"),
            Cell::from("Tên ứng dụng"),
            Cell::from("Chuyên mục"),
            Cell::from("Nguồn"),
        ])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = app.apps_with_status.iter().enumerate().map(|(i, (entry, _))| {
            let is_checked = app.checked_app_ids.contains(&entry.id);
            let check_str = if is_checked { " [x]" } else { " [ ]" };
            let check_style = if is_checked { Style::default().fg(Color::Red) } else { Style::default().fg(Color::DarkGray) };

            let highlight_style = if i == app.selected_index {
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let category_str = entry.category.as_deref().unwrap_or("Other");
            let category_style = if i == app.selected_index {
                Style::default().bg(Color::Blue).fg(Color::Rgb(244, 164, 96)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(210, 105, 30)) // Chocolate brown
            };

            let source_str = entry.package_type.as_deref().unwrap_or("Local");
            let source_style = if i == app.selected_index {
                match source_str {
                    "APT" => Style::default().bg(Color::Blue).fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    "Flatpak" => Style::default().bg(Color::Blue).fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    "Snap" => Style::default().bg(Color::Blue).fg(Color::LightYellow).add_modifier(Modifier::BOLD),
                    _ => Style::default().bg(Color::Blue).fg(Color::Green).add_modifier(Modifier::BOLD),
                }
            } else {
                match source_str {
                    "APT" => Style::default().fg(Color::Cyan),
                    "Flatpak" => Style::default().fg(Color::Magenta),
                    "Snap" => Style::default().fg(Color::LightYellow),
                    _ => Style::default().fg(Color::Green),
                }
            };

            Row::new(vec![
                Cell::from(check_str).style(check_style),
                Cell::from(entry.name.clone()).style(highlight_style),
                Cell::from(category_str).style(category_style),
                Cell::from(source_str).style(source_style),
            ])
        }).collect();

        let widths = [
            Constraint::Length(6),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(9),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(list_block)
            .column_spacing(1);
        app.uninstall_list_state.select(Some(app.selected_index));
        f.render_stateful_widget(table, main_chunks[0], &mut app.uninstall_list_state);
    }

    // Right Pane: Security Warnings
    let warn_block = Block::default()
        .borders(Borders::ALL)
        .title(" Cảnh báo bảo mật gỡ bỏ ")
        .border_style(Style::default().fg(Color::DarkGray));

    let mut warn_text = vec![
        Line::from(vec![
            Span::styled(" CẢNH BÁO NGUY HIỂM / ĐỒNG BỘ XOÁ FILE", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(" Khi thực hiện gỡ cài đặt trên các app được tích:"),
        Line::from(vec![
            Span::styled("   1. ", Style::default().fg(Color::Red)),
            Span::styled("Xoá toàn bộ launcher (.desktop) ra khỏi hệ thống.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("   2. ", Style::default().fg(Color::Red)),
            Span::styled("Xoá các liên kết command-line symlink tại ~/.local/bin/.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("   3. ", Style::default().fg(Color::Red)),
            Span::styled("Đối với ứng dụng kiểu Moved: Xoá sạch thư mục gốc chứa app.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("   4. ", Style::default().fg(Color::Red)),
            Span::styled("Đối với ứng dụng kiểu In-Place: Chỉ xoá file chạy nhị phân và icon đăng ký, KHÔNG xoá các file khác cùng thư mục để tránh mất mát dữ liệu người dùng.", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from("-----------------------------------------------------------------"),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Các ứng dụng được chọn để gỡ cài đặt:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let mut selected_any = false;
    for (entry, _) in &app.apps_with_status {
        if app.checked_app_ids.contains(&entry.id) {
            selected_any = true;
            warn_text.push(Line::from(vec![
                Span::styled("   ✖ ", Style::default().fg(Color::Red)),
                Span::styled(&entry.name, Style::default().fg(Color::White)),
                Span::styled(format!(" ({})", entry.id), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    if !selected_any {
        warn_text.push(Line::from("   (Chưa chọn ứng dụng nào)"));
    }

    warn_text.push(Line::from(""));
    warn_text.push(Line::from("-----------------------------------------------------------------"));
    warn_text.push(Line::from(vec![
        Span::styled(" Nhấn [Space] bên trái để tích chọn ứng dụng.", Style::default().fg(Color::Yellow)),
    ]));
    warn_text.push(Line::from(vec![
        Span::styled(" Nhấn [Insert] hoặc [Enter] để thực thi gỡ cài đặt hoàn toàn.", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
    ]));
    warn_text.push(Line::from(vec![
        Span::styled(" Nhấn [Backspace] để quay lại.", Style::default().fg(Color::LightRed)),
    ]));

    let paragraph = Paragraph::new(warn_text)
        .block(warn_block)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, main_chunks[1]);
}
