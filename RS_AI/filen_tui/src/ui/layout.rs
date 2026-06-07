use ratatui::Frame;
use ratatui::layout::{Layout, Constraint, Direction, Rect};
use ratatui::style::{Style, Color, Modifier};
use ratatui::text::{Span, Line};
use ratatui::widgets::{Block, Borders, Paragraph, List, ListItem, Clear, Wrap};

use crate::app::{App, Screen, PopupState, PaneState};

pub fn draw(app: &mut App, f: &mut Frame) {
    let size = f.size();

    // 1. Tạo Layout chính: Header + Body + Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header (Title)
            Constraint::Min(10),   // Body
            Constraint::Length(3), // Footer (Help line)
        ])
        .split(size);

    // 2. Vẽ Header
    draw_header(app, f, chunks[0]);

    // 3. Vẽ Body dựa trên Screen hiện tại
    match app.current_screen {
        Screen::MainMenu => draw_main_menu(app, f, chunks[1]),
        Screen::Explorer => draw_explorer(app, f, chunks[1]),
        Screen::Account => draw_account(app, f, chunks[1]),
        Screen::Servers => draw_servers(app, f, chunks[1]),
    }

    // 4. Vẽ Footer (Help)
    draw_footer(app, f, chunks[2]);

    // 5. Vẽ các Popup Modals đè lên trên nếu có
    draw_popups(app, f);
}

fn draw_header(app: &App, f: &mut Frame, area: Rect) {
    let active_email = operations_whoami_cache(app);
    
    let header_text = vec![
        Line::from(vec![
            Span::styled(" Filen TUI v1.0.0 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("Tài khoản hoạt động: {}", active_email), Style::default().fg(Color::Yellow)),
        ]),
    ];

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
        
    let paragraph = Paragraph::new(header_text).block(header_block);
    f.render_widget(paragraph, area);
}

fn operations_whoami_cache(app: &App) -> String {
    app.active_account.clone().unwrap_or_else(|| "Default Session".to_string())
}

fn parse_storage_pct(used: &str, max: &str) -> f64 {
    fn to_bytes(s: &str) -> f64 {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() { return 0.0; }
        let num: f64 = parts[0].parse().unwrap_or(0.0);
        if parts.len() < 2 { return num; }
        let unit = parts[1].to_uppercase();
        let mult = match unit.as_str() {
            "B" => 1.0,
            "KB" | "KIB" => 1024.0,
            "MB" | "MIB" => 1024.0 * 1024.0,
            "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
            "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            _ => 1.0,
        };
        num * mult
    }
    let u_bytes = to_bytes(used);
    let m_bytes = to_bytes(max);
    if m_bytes <= 0.0 { return 0.0; }
    (u_bytes / m_bytes) * 100.0
}

fn draw_main_menu(app: &App, f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    // 1. Menu bên trái
    let menu_items = vec![
        " 📂 1. Duyệt File (Explorer - Dual Pane Local/Cloud) ",
        " 👤 2. Quản lý Tài khoản (Multi-account & Config) ",
        " 🗑️ 3. Quản lý Thùng rác (Trash Bin) ",
        " 🌐 4. Cấu hình Máy chủ (WebDAV & S3 Server) ",
    ];

    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.main_menu_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(*item).style(style)
        })
        .collect();

    let menu_block = Block::default()
        .title(" DANH SÁCH CHỨC NĂNG ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
        
    let list = List::new(items).block(menu_block);
    f.render_widget(list, chunks[0]);

    // 2. Bảng thông tin hệ thống bên phải
    let active_server_webdav = if app.webdav_server.running { "🟢 Đang chạy" } else { "🔴 Đang tắt" };
    let active_server_s3 = if app.s3_server.running { "🟢 Đang chạy" } else { "🔴 Đang tắt" };
    
    let info_text = vec![
        Line::from(" Trạng thái TUI: Kết nối tốt"),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Máy chủ ngầm WebDAV: "),
            Span::styled(active_server_webdav, Style::default().fg(if app.webdav_server.running { Color::LightGreen } else { Color::Red })),
        ]),
        Line::from(vec![
            Span::raw(" Máy chủ ngầm S3:     "),
            Span::styled(active_server_s3, Style::default().fg(if app.s3_server.running { Color::LightGreen } else { Color::Red })),
        ]),
        Line::from(""),
        Line::from(format!(" Tài khoản đã nạp:   {} account(s)", app.accounts.len())),
        Line::from(format!(" Thư mục cục bộ mặc định: ~/")),
    ];

    let info_block = Block::default()
        .title(" THÔNG TIN HỆ THỐNG ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
        
    let paragraph = Paragraph::new(info_text).block(info_block);
    f.render_widget(paragraph, chunks[1]);
}

fn draw_explorer(app: &mut App, f: &mut Frame, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(4), // Console status pane
        ])
        .split(area);

    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    // Vẽ bảng trái
    draw_pane(f, &mut app.left_pane, pane_chunks[0], app.active_pane_left);
    // Vẽ bảng phải
    draw_pane(f, &mut app.right_pane, pane_chunks[1], !app.active_pane_left);

    // Vẽ Console Panel ở dưới
    let console_text = if !app.clipboard.is_empty() {
        let action = if app.clipboard_is_cut { "Cắt (Cut)" } else { "Sao chép (Copy)" };
        format!("📋 Đang giữ {} mục trong Clipboard để {}. Bấm Ctrl+V ở pane đích để dán.", app.clipboard.len(), action)
    } else {
        let active_pane = if app.active_pane_left { &app.left_pane } else { &app.right_pane };
        if !active_pane.items.is_empty() {
            let item = &active_pane.items[active_pane.selected_idx];
            let type_str = if item.is_dir { "Thư mục" } else { "Tệp" };
            format!(
                "📁 Đang chọn: {} ({} | Cập nhật: {}) | Số mục đã chọn: {}",
                item.name,
                type_str,
                item.mod_time,
                active_pane.selected_names.len()
            )
        } else {
            "Thư mục trống.".to_string()
        }
    };

    let console_block = Block::default()
        .title(" BẢNG ĐIỀU KHIỂN (CONSOLE STATUS) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
        
    let paragraph = Paragraph::new(console_text).block(console_block);
    f.render_widget(paragraph, main_chunks[1]);
}

fn draw_pane(f: &mut Frame, pane: &mut PaneState, area: Rect, is_active: bool) {
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };
    let border_style = if is_active { Modifier::BOLD } else { Modifier::empty() };
    
    let path_label = if pane.is_local {
        format!(" LOCAL: {} ", pane.path)
    } else {
        format!(" REMOTE (Filen Cloud): {} ", pane.path)
    };

    let pane_title = if pane.selected_names.is_empty() {
        path_label
    } else {
        format!("{} (Đã chọn: {}) ", path_label, pane.selected_names.len())
    };

    let block = Block::default()
        .title(Span::styled(pane_title, Style::default().fg(border_color).add_modifier(border_style)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let height = area.height.saturating_sub(2) as usize;
    pane.adjust_scroll(height);

    let items: Vec<ListItem> = if pane.loading {
        vec![ListItem::new(" Đang tải dữ liệu Cloud... (Loading)")]
    } else if pane.items.is_empty() {
        vec![ListItem::new(" (Thư mục trống)")]
    } else {
        pane.items
            .iter()
            .enumerate()
            .skip(pane.scroll_offset)
            .take(height)
            .map(|(i, item)| {
                let is_checked = pane.selected_names.contains(&item.name);
                let check_prefix = if is_checked { "✔ " } else { "  " };
                let icon = if item.is_dir { "📁 " } else { "📄 " };
                let display_name = format!("{}{}{}", check_prefix, icon, item.name);

                let style = if i == pane.selected_idx && is_active {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if is_checked {
                    Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
                } else if item.is_dir {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let size_str = if item.is_dir {
                    "---".to_string()
                } else {
                    format_bytes_display(item.size)
                };

                let spans = vec![
                    Span::styled(format!("{:<50}", display_name), style),
                    Span::raw("  "),
                    Span::styled(size_str, Style::default().fg(Color::Magenta)),
                ];
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_account(app: &App, f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Active Account Info
            Constraint::Min(8),    // Accounts List
        ])
        .split(area);

    // 1. Vẽ Thông tin tài khoản hoạt động
    let active_name = operations_whoami_cache(app);
    let pct = parse_storage_pct(&app.storage_used, &app.storage_max);
    let bar_width: usize = 30;
    let filled = (pct * bar_width as f64 / 100.0).round() as usize;
    let empty = bar_width.saturating_sub(filled);
    
    let progress_bar = format!(
        "[{}{}] {:.1}% ({} / {})",
        "█".repeat(filled),
        "░".repeat(empty),
        pct,
        app.storage_used,
        app.storage_max
    );

    let info_text = vec![
        Line::from(format!(" 📧 Địa chỉ Email: {}", active_name)),
        Line::from(format!(" 💾 Dung lượng:    {} đã dùng của {}", app.storage_used, app.storage_max)),
        Line::from(vec![
            Span::raw(" 📊 Bộ nhớ Cloud:  "),
            Span::styled(progress_bar, Style::default().fg(Color::LightGreen)),
        ]),
        Line::from(" 🟢 Trạng thái:    Đang trực tuyến (Online)"),
    ];

    let info_block = Block::default()
        .title(" THÔNG TIN TÀI KHOẢN ĐANG HOẠT ĐỘNG ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
        
    let paragraph = Paragraph::new(info_text).block(info_block);
    f.render_widget(paragraph, chunks[0]);

    // 2. Vẽ Danh sách tài khoản Multi-account
    let items: Vec<ListItem> = app.accounts
        .iter()
        .enumerate()
        .map(|(i, acc)| {
            let is_active = app.active_account.as_ref().map_or(i == 0, |email| email == acc);
            let indicator = if is_active { " ● " } else { "   " };
            let prefix = if i == app.active_account_idx { "▶ " } else { "  " };
            
            let text = format!("{}{}{}", prefix, indicator, acc);
            let style = if i == app.active_account_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(text).style(style)
        })
        .collect();

    let list_block = Block::default()
        .title(" DANH SÁCH TÀI KHOẢN TRÊN TUI (MULTI-ACCOUNT LIST) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
        
    let list = List::new(items).block(list_block);
    f.render_widget(list, chunks[1]);
}

fn draw_servers(app: &App, f: &mut Frame, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab header
            Constraint::Length(8), // Form Fields
            Constraint::Min(4),    // Logs
        ])
        .split(area);

    // 1. Draw Server Tabs
    let webdav_style = if app.active_server_tab == 0 { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let s3_style = if app.active_server_tab == 1 { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    
    let tabs_text = vec![
        Line::from(vec![
            Span::styled(" [ Tab 1: WebDAV Server ] ", webdav_style),
            Span::raw("   "),
            Span::styled(" [ Tab 2: S3 Server ] ", s3_style),
        ]),
    ];
    let tab_block = Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(tabs_text).block(tab_block), main_chunks[0]);

    // 2. Draw config fields
    let active_field = app.server_selected_field;
    let mut fields = Vec::new();
    
    if app.active_server_tab == 0 {
        // WebDAV fields
        fields.push(("Username WebDAV:", app.webdav_server.user.as_str()));
        fields.push(("Password WebDAV:", app.webdav_server.pass.as_str()));
        fields.push(("Port WebDAV:    ", app.webdav_server.port.as_str()));
        let https_str = if app.webdav_server.https { "Có (HTTPS)" } else { "Không (HTTP)" };
        fields.push(("HTTPS (Mã hóa): ", https_str));
        let action_str = if app.webdav_server.running { "▶ [ TẮT SERVER ]" } else { "▶ [ BẬT SERVER ]" };
        fields.push(("Hành động:      ", action_str));
    } else {
        // S3 fields
        fields.push(("Access Key ID:  ", app.s3_server.access_key.as_str()));
        fields.push(("Secret Key:     ", app.s3_server.secret_key.as_str()));
        fields.push(("Port S3 Server: ", app.s3_server.port.as_str()));
        let https_str = if app.s3_server.https { "Có (HTTPS)" } else { "Không (HTTP)" };
        fields.push(("HTTPS (Mã hóa): ", https_str));
        let action_str = if app.s3_server.running { "▶ [ TẮT SERVER ]" } else { "▶ [ BẬT SERVER ]" };
        fields.push(("Hành động:      ", action_str));
    }

    let mut form_lines = Vec::new();
    for (i, (label, val)) in fields.iter().enumerate() {
        let style = if i == active_field {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        form_lines.push(Line::from(vec![
            Span::raw(format!("  {} ", label)),
            Span::styled(format!(" {} ", val), style),
        ]));
    }

    let form_block = Block::default()
        .title(" CẤU HÌNH DỊCH VỤ ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    f.render_widget(Paragraph::new(form_lines).block(form_block), main_chunks[1]);

    // 3. Draw Logs
    let logs = if app.active_server_tab == 0 { &app.webdav_server.logs } else { &app.s3_server.logs };
    let log_block = Block::default()
        .title(" TRÌNH GHI NHẬT KÝ (LOGS) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
        
    let items: Vec<ListItem> = logs.iter().map(|log| ListItem::new(log.as_str())).collect();
    let list = List::new(items).block(log_block);
    f.render_widget(list, main_chunks[2]);
}

fn draw_footer(app: &App, f: &mut Frame, area: Rect) {
    let help_text = match app.current_screen {
        Screen::MainMenu => " ▲/▼: Di chuyển | Enter: Chọn | q: Thoát ứng dụng ",
        Screen::Explorer => " Tab: Đổi bảng | Backspace: Lên thư mục | Alt+R: Đổi Pane Local/Cloud | Alt+N: Tạo thư mục | Alt+Y: Đổi tên | Alt+O: Menu Đặc biệt | Alt+T: Đồng bộ | Shift+V: Chọn vùng | Delete: Xóa | Esc: Về Menu chính ",
        Screen::Account => " ▲/▼: Chọn Account | Alt+S: Đổi Hoạt động | Alt+N: Đăng nhập mới | Alt+D: Gỡ tài khoản | Alt+L: Đăng xuất | Alt+C/K: Xuất Config/API | Esc: Về Menu chính ",
        Screen::Servers => " Tab: Đổi WebDAV/S3 | ▲/▼: Chọn trường | Space/Enter: Sửa/Kích hoạt | Esc: Về Menu chính ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
        
    let paragraph = Paragraph::new(help_text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_popups(app: &App, f: &mut Frame) {
    let size = f.size();
    
    match &app.popup_state {
        PopupState::Message { title, message } => {
            let area = centered_rect(60, 40, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .title(Span::styled(format!(" {} ", title), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let paragraph = Paragraph::new(message.as_str()).block(block).wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
        PopupState::RenameInput { old_name, buffer } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .title(Span::styled(" ĐỔI TÊN / NHẬP LIỆU ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let text = vec![
                Line::from(format!(" Nhập giá trị mới cho: {}", old_name)),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" > ", Style::default().fg(Color::Cyan)),
                    Span::styled(buffer.clone() + "█", Style::default().fg(Color::White)),
                ]),
            ];
            let paragraph = Paragraph::new(text).block(block);
            f.render_widget(paragraph, area);
        }
        PopupState::NewFolderInput { buffer } => {
            let area = centered_rect(50, 25, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .title(Span::styled(" TẠO THƯ MỤC MỚI ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let text = vec![
                Line::from(" Nhập tên thư mục mới cần tạo:"),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" > ", Style::default().fg(Color::Cyan)),
                    Span::styled(buffer.clone() + "█", Style::default().fg(Color::White)),
                ]),
            ];
            let paragraph = Paragraph::new(text).block(block);
            f.render_widget(paragraph, area);
        }
        PopupState::LoginInput { email_buffer, pass_buffer, active_field } => {
            let area = centered_rect(60, 45, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .title(Span::styled(" ĐĂNG NHẬP TÀI KHOẢN MỚI ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
                
            let email_style = if *active_field == 0 { Style::default().fg(Color::Black).bg(Color::Cyan) } else { Style::default().fg(Color::White) };
            let pass_style = if *active_field == 1 { Style::default().fg(Color::Black).bg(Color::Cyan) } else { Style::default().fg(Color::White) };
            
            let masked_pass = "*".repeat(pass_buffer.len());
            
            let text = vec![
                Line::from(" Vui lòng nhập thông tin đăng nhập Cloud Filen:"),
                Line::from(" (Nhấn TAB để di chuyển giữa các trường, Enter để Xác nhận)"),
                Line::from(""),
                Line::from(vec![
                    Span::raw(" 📧 Địa chỉ Email: "),
                    Span::styled(format!(" {} ", email_buffer), email_style),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw(" 🔑 Mật khẩu:      "),
                    Span::styled(format!(" {} ", masked_pass), pass_style),
                ]),
            ];
            let paragraph = Paragraph::new(text).block(block);
            f.render_widget(paragraph, area);
        }
        PopupState::ConfirmDelete { name } => {
            let area = centered_rect(50, 30, size);
            f.render_widget(Clear, area);
            let block = Block::default()
                .title(Span::styled(" XÁC NHẬN XÓA TỆP TIN ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));
                
            let text = vec![
                Line::from(" Bạn có chắc chắn muốn xóa mục này khỏi đĩa cứng / Cloud?"),
                Line::from(format!(" > {}", name)),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [ Có (Y) ] ", Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("   "),
                    Span::styled(" [ Không (N) ] ", Style::default().fg(Color::White)),
                ]),
            ];
            let paragraph = Paragraph::new(text).block(block);
            f.render_widget(paragraph, area);
        }
        PopupState::SpecialActionsMenu { selected_idx } => {
            let area = centered_rect(50, 45, size);
            f.render_widget(Clear, area);
            
            let options = vec![
                "1. 🔗 Tạo Link Tải Công Khai",
                "2. ⭐ Thêm/Bỏ Yêu Thích",
                "3. 🗑️ Khôi Phục từ Thùng Rác",
                "4. 🧹 Dọn Dẹp Thùng Rác",
                "5. 🔍 Xem Siêu Dữ Liệu (Stat)",
                "6. ❌ Đóng Menu",
            ];
            
            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let style = if i == *selected_idx {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(*opt).style(style)
                })
                .collect();
                
            let block = Block::default()
                .title(Span::styled(" MENU HÀNH ĐỘNG ĐẶC BIỆT ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
                
            let list = List::new(items).block(block);
            f.render_widget(list, area);
        }
        PopupState::ViewFile { name, content, scroll } => {
            let area = centered_rect(75, 75, size);
            f.render_widget(Clear, area);
            
            let height = area.height.saturating_sub(4) as usize;
            let visible: Vec<ListItem> = content
                .iter()
                .skip(*scroll)
                .take(height)
                .map(|line| ListItem::new(line.as_str()))
                .collect();
                
            let footer = format!(" [▲/▼] Cuộn | [Esc] Thoát | Dòng {} - {} / {}", scroll + 1, (scroll + height).min(content.len()), content.len());
            let block = Block::default()
                .title(Span::styled(format!(" {} ", name), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
                
            let list = List::new(visible).block(block);
            f.render_widget(list, area);
            
            // Vẽ dòng trạng thái trợ giúp dưới popup
            let sub_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
            f.render_widget(Paragraph::new(footer).style(Style::default().fg(Color::DarkGray)), sub_area);
        }
        PopupState::ConfirmEmptyTrash => {}
        PopupState::SwitchAccountMenu { selected_idx } => {
            let area = centered_rect(50, 45, size);
            f.render_widget(Clear, area);
            
            let items: Vec<ListItem> = app.accounts
                .iter()
                .enumerate()
                .map(|(i, acc)| {
                    let is_active = app.active_account.as_ref().map_or(i == 0, |email| email == acc);
                    let indicator = if is_active { " ● " } else { "   " };
                    let prefix = if i == *selected_idx { "▶ " } else { "  " };
                    
                    let text = format!("{}{}{}", prefix, indicator, acc);
                    let style = if i == *selected_idx {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else if is_active {
                        Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    
                    ListItem::new(text).style(style)
                })
                .collect();
                
            let block = Block::default()
                .title(Span::styled(" CHỌN TÀI KHOẢN HOẠT ĐỘNG ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
                
            let list = List::new(items).block(block);
            f.render_widget(list, area);
        }
        PopupState::None => {}
    }

    // Vẽ Loading overlay nếu đang tải
    if app.is_loading {
        let area = centered_rect(35, 12, size);
        f.render_widget(Clear, area);
        let block = Block::default()
            .title(Span::styled(" THÔNG BÁO ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let paragraph = Paragraph::new("\n   Đang xử lý dữ liệu... ⏳\n   Vui lòng đợi trong giây lát.")
            .block(block)
            .style(Style::default().fg(Color::White));
        f.render_widget(paragraph, area);
    }
}

// Hàm trợ giúp căn giữa khung hình popup
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Trình vẽ byte dung lượng thân thiện
fn format_bytes_display(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let k = 1024.0;
    let sizes = ["B", "KB", "MB", "GB", "TB"];
    let i = (bytes as f64).log(k).floor() as usize;
    if i >= sizes.len() {
        return format!("{} B", bytes);
    }
    let val = bytes as f64 / k.powi(i as i32);
    format!("{:.1} {}", val, sizes[i])
}
