use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::app::{App, Screen, ConfirmAction};
use crate::api::ApiStatus;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Bố cục chính: Header, Body, Logs, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main Body
            Constraint::Length(6), // Logs/Console
            Constraint::Length(1), // Footer/Keys Helper
        ])
        .split(size);

    // 1. Vẽ Header
    draw_header(f, chunks[0]);

    // 2. Vẽ Main Body (Sidebar + Detail)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Sidebar
            Constraint::Percentage(70), // Detail Panel
        ])
        .split(chunks[1]);

    draw_sidebar(f, body_chunks[0], app);
    draw_detail(f, body_chunks[1], app);

    // 3. Vẽ Logs
    draw_logs(f, chunks[2], app);

    // 4. Vẽ Footer
    draw_footer(f, chunks[3], app);

    // 5. Vẽ các Pop-up Modals đè lên tùy màn hình
    let current_screen = app.current_screen.clone();
    match current_screen {
        Screen::AddProvider | Screen::EditProvider => {
            draw_form_modal(f, size, app);
        }
        Screen::ModelScanResult => {
            draw_models_modal(f, size, app);
        }
        Screen::ManageAuthKeys => {
            draw_auth_keys_modal(f, size, app);
        }
        Screen::QuickClean => {
            draw_quick_clean_modal(f, size, app);
        }
        Screen::Confirmation => {
            draw_confirmation_modal(f, size, app);
        }
        Screen::SelectPreset => {
            draw_select_preset_modal(f, size, app);
        }
        Screen::Main => {}
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let logo = " 🌐 OPENCODE API MANAGER & MODEL SYNCR ";
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(10, 20, 30));

    let paragraph = Paragraph::new(logo)
        .block(header_block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    f.render_widget(paragraph, area);
}

fn draw_sidebar(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" 📁 NHÀ CUNG CẤP (PROVIDERS) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(10, 15, 25));

    if app.providers_keys.is_empty() {
        let empty = Paragraph::new("Không có provider nào.\nBấm 'A' để thêm.")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app.providers_keys.iter().map(|key| {
        let provider = app.config.provider.get(key);
        let name = provider.map(|p| p.name.as_str()).unwrap_or("Không tên");
        
        let status_str = match app.api_status_cache.get(key) {
            Some(Some(ApiStatus::Alive)) => ("● Hoạt động", Style::default().fg(Color::Green)),
            Some(Some(ApiStatus::InsufficientCredits(_))) => ("● Hết tiền", Style::default().fg(Color::Yellow)),
            Some(Some(ApiStatus::InvalidKey(_))) => ("● Lỗi Key", Style::default().fg(Color::Red)),
            Some(Some(ApiStatus::Offline(_))) => ("● Ngoại tuyến", Style::default().fg(Color::Magenta)),
            _ => ("○ Chưa check", Style::default().fg(Color::DarkGray)),
        };

        let spans = vec![
            Span::styled(format!(" {:<18}", name), Style::default()),
            Span::raw(" "),
            Span::styled(status_str.0, status_str.1),
        ];

        ListItem::new(Line::from(spans)).style(Style::default().fg(Color::Gray))
    }).collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Rgb(40, 50, 70)).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    app.provider_list_state.select(Some(app.selected_provider_idx));
    f.render_stateful_widget(list, area, &mut app.provider_list_state);
}

fn draw_detail(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" 🔍 THÔNG TIN CHI TIẾT ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(10, 15, 25));

    let selected_id = match app.selected_provider_id() {
        Some(id) => id,
        None => {
            let paragraph = Paragraph::new("Chọn một provider bên trái để xem chi tiết.")
                .block(block)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(paragraph, area);
            return;
        }
    };

    let provider = match app.config.provider.get(selected_id) {
        Some(p) => p,
        None => return,
    };

    // Tạo các chunks bên trong Detail
    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Cấu hình chung
            Constraint::Min(4),    // Danh sách models
        ])
        .split(area.inner(&Margin { vertical: 1, horizontal: 1 }));

    // Vẽ cấu hình chung
    let obfuscated_key = obfuscate_key(&provider.options.api_key);
    let status_desc = match app.api_status_cache.get(selected_id) {
        Some(Some(ApiStatus::Alive)) => "Hoạt động (200 OK)".to_string(),
        Some(Some(ApiStatus::InsufficientCredits(m))) => format!("Hết tiền / Hạn mức (402): {}", m),
        Some(Some(ApiStatus::InvalidKey(m))) => format!("Lỗi API Key (401/403): {}", m),
        Some(Some(ApiStatus::Offline(m))) => format!("Offline / Mất kết nối: {}", m),
        _ => "Chưa kiểm tra (Nhấn Enter để kiểm tra/quét)".to_string(),
    };

    let general_info = format!(
        "🔑 ID Provider:   {}\n\
         🏷️ Tên hiển thị:  {}\n\
         🌐 URL Endpoint:  {}\n\
         🔑 API Key:       {}\n\
         📊 Trạng thái:    {}",
        selected_id,
        provider.name,
        provider.options.base_url,
        obfuscated_key,
        status_desc
    );

    let info_paragraph = Paragraph::new(general_info)
        .block(Block::default().title(" ⚙️ Cấu hình chung ").borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::LightCyan));
    f.render_widget(info_paragraph, detail_chunks[0]);

    // Vẽ danh sách Models đang cấu hình
    let models_title = format!(" 🤖 Các mô hình đã cấu hình ({}) ", provider.models.len());
    let models_block = Block::default()
        .title(models_title)
        .borders(Borders::NONE);

    if provider.models.is_empty() {
        let empty = Paragraph::new("Chưa có model nào được cấu hình.\nBấm [Enter] để quét và thêm models.")
            .block(models_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, detail_chunks[1]);
    } else {
        // Tạo bảng models
        let mut model_keys: Vec<String> = provider.models.keys().cloned().collect();
        model_keys.sort();

        let rows: Vec<Row> = model_keys.iter().map(|key| {
            let model = provider.models.get(key).unwrap();
            let limit_str = if let Some(ref lim) = model.limit {
                format!(
                    "In: {} / Out: {}", 
                    lim.context.map(|c| format_num(c)).unwrap_or_else(|| "-".to_string()),
                    lim.output.map(|o| format_num(o)).unwrap_or_else(|| "-".to_string())
                )
            } else {
                "-".to_string()
            };

            let modalities_str = if let Some(ref modas) = model.modalities {
                format!("In: {:?} | Out: {:?}", modas.input, modas.output)
            } else {
                "-".to_string()
            };

            Row::new(vec![
                Cell::from(key.clone()).style(Style::default().fg(Color::Yellow)),
                Cell::from(limit_str).style(Style::default().fg(Color::Gray)),
                Cell::from(modalities_str).style(Style::default().fg(Color::DarkGray)),
            ])
        }).collect();

        let table = Table::new(rows, [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .header(Row::new(vec![
            Cell::from("Tên Model").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Giới hạn Token").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Phương thức (Modalities)").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]))
        .block(models_block);

        f.render_widget(table, detail_chunks[1]);
    }
}

fn draw_logs(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" 🖥️ NHẬT KÝ HOẠT ĐỘNG / CONSOLE LOGS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .bg(Color::Rgb(5, 5, 10));

    // Hiển thị 4 dòng cuối
    let take_count = area.height.saturating_sub(2) as usize;
    let skip_count = app.logs.len().saturating_sub(take_count);
    let visible_logs: Vec<ListItem> = app.logs.iter()
        .skip(skip_count)
        .map(|log| ListItem::new(Line::from(Span::styled(log, Style::default().fg(Color::LightGreen)))))
        .collect();

    let list = List::new(visible_logs).block(block);
    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, area: Rect, _app: &App) {
    let helper = " [Enter] Check/Scan | [A] Thêm | [E] Sửa | [D] Xoá | [C] Clean | [S] Đồng bộ auth | [M] Keys | [Q] Thoát ";
    let paragraph = Paragraph::new(helper)
        .style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(paragraph, area);
}

// === FORM MODAL (THÊM / SỬA PROVIDER) ===
fn draw_form_modal(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area); // Xoá nền bên dưới

    let preset = app.presets.iter().find(|p| p.id == app.form.preset_id);
    let id_prefix = preset.map(|p| p.id_prefix.as_str()).unwrap_or("custom");
    let preset_name = preset.map(|p| p.name.as_str()).unwrap_or("Custom Provider");

    let title = if app.current_screen == Screen::AddProvider {
        format!(" ➕ THÊM PROVIDER (Mã tự sinh: {}_*) ", id_prefix)
    } else {
        format!(" ✏️ CHỈNH SỬA PROVIDER (ID: {}) ", app.form.id)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    // Layout bên trong form
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Preset (Loại API)
            Constraint::Length(3), // Name
            Constraint::Length(3), // URL
            Constraint::Length(3), // API Key
            Constraint::Length(3), // Buttons
            Constraint::Min(2),    // Test Result
        ])
        .split(popup_area);

    // 0. Preset field
    let preset_style = if app.form.focus_index == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let preset_val = format!("◀  {}  ▶ (Nhấn Enter để Tìm kiếm)", preset_name);
    let preset_input = Paragraph::new(preset_val)
        .alignment(Alignment::Center)
        .block(Block::default().title(" 🏷️ Loại Provider (Trái/Phải để chuyển, Enter để Tìm kiếm) ").borders(Borders::ALL).border_style(preset_style));
    f.render_widget(preset_input, inner_chunks[0]);

    // Helper cho border style
    let get_input_style = |field_idx: usize| -> (Style, String) {
        if app.form.focus_index == field_idx {
            if app.form.is_editing_field {
                (Style::default().fg(Color::Green), " (Đang gõ... Nhấn Enter để hoàn tất) ".to_string())
            } else {
                (Style::default().fg(Color::Yellow), " (Nhấn Enter để chỉnh sửa) ".to_string())
            }
        } else {
            (Style::default().fg(Color::Gray), "".to_string())
        }
    };

    // 1. Name field
    let (name_style, name_hint) = get_input_style(1);
    let name_val = format!("{}{}", app.form.name, if app.form.focus_index == 1 && app.form.is_editing_field { "█" } else { "" });
    let name_input = Paragraph::new(name_val)
        .block(Block::default().title(format!(" Tên hiển thị{}", name_hint)).borders(Borders::ALL).border_style(name_style));
    f.render_widget(name_input, inner_chunks[1]);

    // 2. Base URL field
    let (url_style, url_hint) = get_input_style(2);
    let url_val = format!("{}{}", app.form.base_url, if app.form.focus_index == 2 && app.form.is_editing_field { "█" } else { "" });
    let url_input = Paragraph::new(url_val)
        .block(Block::default().title(format!(" Base URL (Ví dụ: https://.../v1){}", url_hint)).borders(Borders::ALL).border_style(url_style));
    f.render_widget(url_input, inner_chunks[2]);

    // 3. API Key field
    let (key_style, key_hint) = get_input_style(3);
    let key_val = format!("{}{}", app.form.api_key, if app.form.focus_index == 3 && app.form.is_editing_field { "█" } else { "" });
    let key_input = Paragraph::new(key_val)
        .block(Block::default().title(format!(" API Key (Ví dụ: tp-xxxxx){}", key_hint)).borders(Borders::ALL).border_style(key_style));
    f.render_widget(key_input, inner_chunks[3]);

    // Vẽ các nút: Test, Save, Cancel
    let btn_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(inner_chunks[4]);

    let btn_test_style = if app.form.focus_index == 4 { Style::default().bg(Color::Yellow).fg(Color::Black) } else { Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White) };
    let btn_test = Paragraph::new(" [T] KIỂM TRA (TEST) ")
        .block(Block::default().borders(Borders::ALL).border_style(btn_test_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_test, btn_layout[0]);

    let btn_save_style = if app.form.focus_index == 5 { Style::default().bg(Color::Green).fg(Color::Black) } else { Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White) };
    let btn_save = Paragraph::new(" [S] LƯU (SAVE) ")
        .block(Block::default().borders(Borders::ALL).border_style(btn_save_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_save, btn_layout[1]);

    let btn_cancel_style = if app.form.focus_index == 6 { Style::default().bg(Color::Red).fg(Color::Black) } else { Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White) };
    let btn_cancel = Paragraph::new(" [Esc] HUỶ (CANCEL) ")
        .block(Block::default().borders(Borders::ALL).border_style(btn_cancel_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_cancel, btn_layout[2]);

    // Vẽ trạng thái Test connection hoặc Cảnh báo trùng lặp
    let test_msg;
    let test_style;
    if let Some((dup_name, dup_id)) = app.detect_duplicate() {
        test_msg = format!("⚠️ CẢNH BÁO TRÙNG: Trùng API & URL với '{}' ({})!", dup_name, dup_id);
        test_style = Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD);
    } else if app.form.is_testing {
        test_msg = "⏳ Đang kết nối thử...".to_string();
        test_style = Style::default().fg(Color::Yellow);
    } else if let Some(ref status) = app.form.test_status {
        match status {
            ApiStatus::Alive => {
                test_msg = "✅ Kết nối thành công! API hoạt động tốt.".to_string();
                test_style = Style::default().fg(Color::Green);
            }
            ApiStatus::InsufficientCredits(m) => {
                test_msg = format!("⚠️ Hết tiền / Quota (402): {}", m);
                test_style = Style::default().fg(Color::Yellow);
            }
            ApiStatus::InvalidKey(m) => {
                test_msg = format!("❌ Sai Key (401/403): {}", m);
                test_style = Style::default().fg(Color::Red);
            }
            ApiStatus::Offline(m) => {
                test_msg = format!("❌ Ngoại tuyến: {}", m);
                test_style = Style::default().fg(Color::Magenta);
            }
        }
    } else {
        test_msg = "Bấm nút TEST để kiểm thử kết nối trước khi lưu.".to_string();
        test_style = Style::default().fg(Color::DarkGray);
    }

    let test_paragraph = Paragraph::new(test_msg)
        .style(test_style)
        .alignment(Alignment::Center);
    f.render_widget(test_paragraph, inner_chunks[5]);
}

// === MODELS SCAN MODAL ===
// === MODELS SCAN MODAL ===
fn draw_models_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(50, 70, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" 🤖 CHỌN CÁC MODEL ĐỂ ĐỒNG BỘ ({}) ", app.scanned_models.len()))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3), // Footer hướng dẫn
        ])
        .split(popup_area);

    // Tạo danh sách checkbox
    let items: Vec<ListItem> = app.scanned_models.iter().map(|(name, checked)| {
        let prefix = if *checked { "[x] " } else { "[ ] " };
        let style = Style::default().fg(Color::Gray);

        let span_prefix = Span::styled(prefix, if *checked { Style::default().fg(Color::Green) } else { Style::default().fg(Color::DarkGray) });
        let span_name = Span::raw(name);

        ListItem::new(Line::from(vec![span_prefix, span_name])).style(style)
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Rgb(40, 50, 70)).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    app.models_list_state.select(Some(app.selected_model_idx));
    f.render_stateful_widget(list, inner_chunks[0], &mut app.models_list_state);

    let footer_text = " [Space] Chọn/Bỏ chọn | [Enter] Đồng bộ | [Esc] Huỷ ";
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(footer, inner_chunks[1]);
}

// === AUTH KEYS MANAGER MODAL ===
fn draw_auth_keys_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" 🔑 QUẢN LÝ API KEYS TRONG auth.json ({}) ", app.auth_keys.len()))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(popup_area);

    if app.auth_keys.is_empty() {
        let empty = Paragraph::new("Không có API key nào trong auth.json.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, inner_chunks[0]);
    } else {
        let items: Vec<ListItem> = app.auth_keys.iter().map(|(name, val)| {
            let obfuscated = obfuscate_key(val);
            let line = Line::from(vec![
                Span::styled(format!("{:<25}", name), Style::default().fg(Color::Cyan)),
                Span::styled(obfuscated, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line).style(Style::default().fg(Color::Gray))
        }).collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Rgb(40, 50, 70)).fg(Color::White).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        app.auth_keys_list_state.select(Some(app.selected_auth_idx));
        f.render_stateful_widget(list, inner_chunks[0], &mut app.auth_keys_list_state);
    }

    let footer_text = " [Delete] Xoá API Key thừa | [Esc] Đóng ";
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(footer, inner_chunks[1]);
}

// === QUICK CLEAN MODAL ===
fn draw_quick_clean_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 🧹 DỌN DẸP NHANH API HỎNG / HẾT HẠN ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2), // Tiêu đề phụ
            Constraint::Min(5),    // List candidates
            Constraint::Length(3), // Footer buttons
        ])
        .split(popup_area);

    let sub_title = Paragraph::new("Chọn các API hỏng bên dưới để xoá hàng loạt:")
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(sub_title, inner_chunks[0]);

    let items: Vec<ListItem> = app.clean_list.iter().map(|(id, name, status, checked)| {
        let prefix = if *checked { "[x] " } else { "[ ] " };
        let status_desc = match status {
            ApiStatus::InsufficientCredits(_) => "Hết tiền (402)",
            ApiStatus::InvalidKey(_) => "Lỗi Key (401/403)",
            _ => "Lỗi khác",
        };

        let line = Line::from(vec![
            Span::styled(prefix, if *checked { Style::default().fg(Color::Red) } else { Style::default().fg(Color::DarkGray) }),
            Span::styled(format!("{:<15}", name), Style::default().fg(Color::White)),
            Span::styled(format!(" ({})", id), Style::default().fg(Color::DarkGray)),
            Span::raw(" - "),
            Span::styled(status_desc, Style::default().fg(Color::Yellow)),
        ]);

        ListItem::new(line).style(Style::default().fg(Color::Gray))
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Rgb(40, 50, 70)).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    app.clean_list_state.select(Some(app.selected_clean_idx));
    f.render_stateful_widget(list, inner_chunks[1], &mut app.clean_list_state);

    let footer_text = " [Space] Chọn | [Enter] Thực hiện Xoá | [Esc] Huỷ ";
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD));
    f.render_widget(footer, inner_chunks[2]);
}

// === CONFIRMATION MODAL ===
fn draw_confirmation_modal(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(40, 30, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ❓ XÁC NHẬN ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .bg(Color::Rgb(25, 10, 10));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(popup_area);

    let prompt = match &app.confirm_action {
        Some(ConfirmAction::DeleteProvider(id)) => format!("Bạn có chắc chắn muốn xoá\nProvider '{}' khỏi opencode.json?", id),
        Some(ConfirmAction::DeleteAuthKey(name)) => format!("Bạn có chắc chắn muốn xoá\nAPI Key '{}' khỏi auth.json?", name),
        Some(ConfirmAction::CleanSelected) => "Bạn có chắc chắn muốn xoá tất cả\ncác API lỗi đã chọn?".to_string(),
        Some(ConfirmAction::OverwriteDuplicate { duplicate_id, duplicate_name }) => {
            format!(
                "⚠️ CẢNH BÁO TRÙNG LẶP!\n\nAPI và Base URL này trùng khớp với\nprovider '{}' (ID: {}).\n\nBạn có muốn LỌC TRÙNG (ghi đè & gộp)\nlên provider đã tồn tại đó không?",
                duplicate_name, duplicate_id
            )
        }
        None => "Bạn có chắc chắn muốn thực hiện hành động này?".to_string(),
    };

    let prompt_p = Paragraph::new(prompt)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    f.render_widget(prompt_p, inner_chunks[0]);

    // Vẽ 2 nút Yes và No
    let btn_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner_chunks[1]);

    let yes_style = if app.confirm_focus_yes {
        Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::Rgb(50, 40, 40)).fg(Color::Gray)
    };
    let btn_yes = Paragraph::new(" CÓ (YES) ")
        .block(Block::default().borders(Borders::ALL).border_style(yes_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_yes, btn_layout[0]);

    let no_style = if !app.confirm_focus_yes {
        Style::default().bg(Color::Gray).fg(Color::Black).add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::Gray)
    };
    let btn_no = Paragraph::new(" KHÔNG (NO) ")
        .block(Block::default().borders(Borders::ALL).border_style(no_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_no, btn_layout[1]);
}

// === UTILS ===
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

fn obfuscate_key(key: &str) -> String {
    if key.is_empty() {
        return "Trống".to_string();
    }
    if key.len() <= 12 {
        return "...".to_string();
    }
    format!("{}...{}", &key[..6], &key[key.len() - 6..])
}

fn format_num(val: u64) -> String {
    if val >= 1_048_576 {
        format!("{}M", val / 1_048_576)
    } else if val >= 1024 {
        format!("{}k", val / 1024)
    } else {
        val.to_string()
    }
}

fn draw_select_preset_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(70, 70, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 🏷️ CHỌN LOẠI PROVIDER PRESET ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::LightCyan))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(5),    // List of presets
            Constraint::Length(1), // Help bar
        ])
        .split(popup_area);

    // 1. Search Bar
    let search_style = Style::default().fg(Color::Yellow);
    let search_text = format!(" {}", app.preset_search_query);
    let search_paragraph = Paragraph::new(search_text)
        .block(Block::default()
            .title(" 🔍 Nhập từ khoá tìm kiếm (Ví dụ: mimo, openai...) ")
            .borders(Borders::ALL)
            .border_style(search_style)
        );
    f.render_widget(search_paragraph, inner_chunks[0]);
    
    // Đặt con trỏ nhấp nháy trong ô tìm kiếm để tránh cảm giác bị "đơ" (frozen input)
    let cursor_x = (inner_chunks[0].x + 2 + app.preset_search_query.len() as u16)
        .min(inner_chunks[0].x + inner_chunks[0].width - 2);
    let cursor_y = inner_chunks[0].y + 1;
    f.set_cursor(cursor_x, cursor_y);

    // 2. Filtered Presets List
    let filtered = app.filtered_presets();
    let mut list_items = Vec::new();

    for preset in &filtered {
        let mut line_content = format!("{:<25}", preset.name);
        if !preset.base_url.is_empty() {
            line_content.push_str(&format!(" ({})", preset.base_url));
        }
        list_items.push(ListItem::new(line_content).style(Style::default().fg(Color::White)));
    }

    if list_items.is_empty() {
        let empty_msg = Paragraph::new("❌ Không tìm thấy preset nào phù hợp. Nhập chữ khác hoặc dùng Esc để huỷ.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        f.render_widget(empty_msg, inner_chunks[1]);
    } else {
        let list_widget = List::new(list_items)
            .block(Block::default()
                .title(format!(" Danh sách preset khớp ({}) ", filtered.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray))
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
            .highlight_symbol("● ");

        app.preset_list_state.select(Some(app.selected_preset_search_idx));
        f.render_stateful_widget(list_widget, inner_chunks[1], &mut app.preset_list_state);
    }

    // 3. Help Bar
    let help_text = " [Gõ chữ] Tìm kiếm | [↑/↓] Di chuyển | [Enter] Chọn | [Esc] Quay lại ";
    let help_paragraph = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help_paragraph, inner_chunks[2]);
}
