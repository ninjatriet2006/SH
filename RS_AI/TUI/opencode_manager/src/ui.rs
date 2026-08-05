use crate::api::ApiStatus;
use crate::app::{App, BulkFocus, CkeyPickMode, ConfirmAction, Screen};
use crate::config::normalize_base_url;
use ratatui::{prelude::*, widgets::*};

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
        Screen::CKeyDashboard => {
            draw_ckey_dashboard_modal(f, size, app);
        }
        Screen::BulkAddProviders => {
            draw_bulk_add_modal(f, size, app);
        }
        Screen::CKeyImport => {
            draw_ckey_import_modal(f, size, app);
        }
        Screen::CKeyUsage => {
            draw_ckey_usage_modal(f, size, app);
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

    let items: Vec<ListItem> = app
        .providers_keys
        .iter()
        .map(|key| {
            let provider = app.config.provider.get(key);
            let name = provider
                .map(|p| {
                    if p.name.is_empty() {
                        key.as_str()
                    } else {
                        p.name.as_str()
                    }
                })
                .unwrap_or("Không tên");

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
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 50, 70))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
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
        .split(area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }));

    // Vẽ cấu hình chung
    let obfuscated_key = obfuscate_key(&provider.options.api_key);
    let status_desc = match app.api_status_cache.get(selected_id) {
        Some(Some(ApiStatus::Alive)) => "Hoạt động (200 OK)".to_string(),
        Some(Some(ApiStatus::InsufficientCredits(m))) => format!("Hết tiền / Hạn mức (402): {}", m),
        Some(Some(ApiStatus::InvalidKey(m))) => format!("Lỗi API Key (401/403): {}", m),
        Some(Some(ApiStatus::Offline(m))) => format!("Offline / Mất kết nối: {}", m),
        _ => "Chưa kiểm tra (Nhấn Enter để kiểm tra/quét)".to_string(),
    };

    let name_display = if provider.name.is_empty() {
        "Không tên (Chưa thiết lập)"
    } else {
        &provider.name
    };
    let general_info = format!(
        "🔑 ID Provider:   {}\n\
         🏷️ Tên hiển thị:  {}\n\
         🌐 URL Endpoint:  {}\n\
         🔑 API Key:       {}\n\
         📊 Trạng thái:    {}",
        selected_id, name_display, provider.options.base_url, obfuscated_key, status_desc
    );

    let info_paragraph = Paragraph::new(general_info)
        .block(
            Block::default()
                .title(" ⚙️ Cấu hình chung ")
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::LightCyan));
    f.render_widget(info_paragraph, detail_chunks[0]);

    // Vẽ danh sách Models đang cấu hình
    let models_title = format!(" 🤖 Các mô hình đã cấu hình ({}) ", provider.models.len());
    let models_block = Block::default().title(models_title).borders(Borders::NONE);

    if provider.models.is_empty() {
        let empty = Paragraph::new("Chưa có model nào được cấu hình.\nBấm [Enter] để quét và thêm models.")
            .block(models_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, detail_chunks[1]);
    } else {
        // Tạo bảng models
        let mut model_keys: Vec<String> = provider.models.keys().cloned().collect();
        model_keys.sort();

        let rows: Vec<Row> = model_keys
            .iter()
            .map(|key| {
                let model = provider.models.get(key).unwrap();
                let limit_str = if let Some(ref lim) = model.limit {
                    format!(
                        "In: {} / Out: {}",
                        lim.context.map(format_num).unwrap_or_else(|| "-".to_string()),
                        lim.output.map(format_num).unwrap_or_else(|| "-".to_string())
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
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ],
        )
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
    let visible_logs: Vec<ListItem> = app
        .logs
        .iter()
        .skip(skip_count)
        .map(|log| ListItem::new(Line::from(Span::styled(log, Style::default().fg(Color::LightGreen)))))
        .collect();

    let list = List::new(visible_logs).block(block);
    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    // [G] hiển thị khi provider ĐANG CHỌN là CKey; [K] Thêm nhanh luôn hiển thị.
    let g_part = if app.has_ckey_support() {
        " | [G] Kiểm tra TK"
    } else {
        ""
    };
    let k_part = " | [K] Thêm nhanh";
    let helper = format!(
        " [Enter] Check/Scan | [A] Thêm | [E] Sửa | [D] Xoá | [C] Clean | [S] Đồng bộ auth | [M] Keys{}{} | [Alt+O] OpenCode | [Q] Thoát ",
        g_part, k_part
    );
    let paragraph = Paragraph::new(helper)
        .style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
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

    // Ô Account key chỉ hiển thị khi endpoint là CKey → bố cục động.
    let show_account_key = normalize_base_url(&app.form.base_url)
        == normalize_base_url(crate::ckey::CKEY_LLM_BASE_URL);

    let mut form_constraints = vec![
        Constraint::Length(3), // Preset (Loại API)
        Constraint::Length(3), // Name
        Constraint::Length(3), // URL
        Constraint::Length(3), // API Key
    ];
    if show_account_key {
        form_constraints.push(Constraint::Length(3)); // Account key
    }
    form_constraints.push(Constraint::Length(3)); // Buttons
    form_constraints.push(Constraint::Min(2));    // Test Result

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(form_constraints)
        .split(popup_area);

    // Vị trí của khối Buttons / Test result phụ thuộc có ô Account key hay không.
    let btn_idx = if show_account_key { 5 } else { 4 };
    let test_idx = btn_idx + 1;

    // 0. Preset field
    let preset_style = if app.form.focus_index == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };
    let preset_val = format!("◀  {}  ▶ (Nhấn Enter để Tìm kiếm)", preset_name);
    let preset_input = Paragraph::new(preset_val).alignment(Alignment::Center).block(
        Block::default()
            .title(" 🏷️ Loại Provider (Trái/Phải để chuyển, Enter để Tìm kiếm) ")
            .borders(Borders::ALL)
            .border_style(preset_style),
    );
    f.render_widget(preset_input, inner_chunks[0]);

    // Helper cho border style
    let get_input_style = |field_idx: usize| -> (Style, String) {
        if app.form.focus_index == field_idx {
            if app.form.is_editing_field {
                (
                    Style::default().fg(Color::Green),
                    " (Đang gõ... Nhấn Enter để hoàn tất) ".to_string(),
                )
            } else {
                (
                    Style::default().fg(Color::Yellow),
                    " (Nhấn Enter để chỉnh sửa) ".to_string(),
                )
            }
        } else {
            (Style::default().fg(Color::Gray), "".to_string())
        }
    };

    // 1. Name field
    let (name_style, name_hint) = get_input_style(1);
    let name_val = format!(
        "{}{}",
        app.form.name,
        if app.form.focus_index == 1 && app.form.is_editing_field {
            "█"
        } else {
            ""
        }
    );
    let name_input = Paragraph::new(name_val).block(
        Block::default()
            .title(format!(" Tên hiển thị{}", name_hint))
            .borders(Borders::ALL)
            .border_style(name_style),
    );
    f.render_widget(name_input, inner_chunks[1]);

    // 2. Base URL field
    let (url_style, url_hint) = get_input_style(2);
    let url_val = format!(
        "{}{}",
        app.form.base_url,
        if app.form.focus_index == 2 && app.form.is_editing_field {
            "█"
        } else {
            ""
        }
    );
    let url_input = Paragraph::new(url_val).block(
        Block::default()
            .title(format!(" Base URL (Ví dụ: https://.../v1){}", url_hint))
            .borders(Borders::ALL)
            .border_style(url_style),
    );
    f.render_widget(url_input, inner_chunks[2]);

    // 3. API Key field
    let (key_style, key_hint) = get_input_style(3);
    let key_val = format!(
        "{}{}",
        app.form.api_key,
        if app.form.focus_index == 3 && app.form.is_editing_field {
            "█"
        } else {
            ""
        }
    );
    let key_input = Paragraph::new(key_val).block(
        Block::default()
            .title(format!(" API Key (Ví dụ: tp-xxxxx){}", key_hint))
            .borders(Borders::ALL)
            .border_style(key_style),
    );
    f.render_widget(key_input, inner_chunks[3]);

    // 4. Account key field (chỉ khi endpoint là CKey)
    if show_account_key {
        let (acc_style, acc_hint) = get_input_style(4);
        let acc_val = format!(
            "{}{}",
            app.form.account_key,
            if app.form.focus_index == 4 && app.form.is_editing_field {
                "█"
            } else {
                ""
            }
        );
        let acc_input = Paragraph::new(acc_val).block(
            Block::default()
                .title(format!(" Account key (trang Profile ckey.vn){}", acc_hint))
                .borders(Borders::ALL)
                .border_style(acc_style),
        );
        f.render_widget(acc_input, inner_chunks[4]);
    }

    // Vẽ các nút: Test, Save, Cancel
    let btn_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(inner_chunks[btn_idx]);

    let btn_test_style = if app.form.focus_index == 5 {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White)
    };
    let btn_test = Paragraph::new(" [T] KIỂM TRA (TEST) ")
        .block(Block::default().borders(Borders::ALL).border_style(btn_test_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_test, btn_layout[0]);

    let btn_save_style = if app.form.focus_index == 6 {
        Style::default().bg(Color::Green).fg(Color::Black)
    } else {
        Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White)
    };
    let btn_save = Paragraph::new(" [S] LƯU (SAVE) ")
        .block(Block::default().borders(Borders::ALL).border_style(btn_save_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_save, btn_layout[1]);

    let btn_cancel_style = if app.form.focus_index == 7 {
        Style::default().bg(Color::Red).fg(Color::Black)
    } else {
        Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White)
    };
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

    let test_paragraph = Paragraph::new(test_msg).style(test_style).alignment(Alignment::Center);
    f.render_widget(test_paragraph, inner_chunks[test_idx]);
}

// === MODELS SCAN MODAL ===
fn draw_models_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(60, 75, area);
    f.render_widget(Clear, popup_area);

    let filtered = app.filtered_scanned_models();

    let block = Block::default()
        .title(format!(
            " 🤖 CHỌN CÁC MODEL ĐỂ ĐỒNG BỘ ({}/{}) ",
            filtered.len(),
            app.scanned_models.len()
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(5),    // List of models
            Constraint::Length(3), // Footer hướng dẫn
        ])
        .split(popup_area);

    // 1. Ô tìm kiếm (Search bar)
    let search_style = Style::default().fg(Color::Yellow);
    let search_text = format!(" {}", app.model_search_query);
    let search_paragraph = Paragraph::new(search_text).block(
        Block::default()
            .title(" 🔍 Nhập từ khoá tìm kiếm model (Ví dụ: llama, qwen...) ")
            .borders(Borders::ALL)
            .border_style(search_style),
    );
    f.render_widget(search_paragraph, inner_chunks[0]);

    // Đặt con trỏ nhấp nháy trong ô tìm kiếm để tránh cảm giác bị "đơ" (frozen input)
    let cursor_x = (inner_chunks[0].x + 2 + app.model_search_query.len() as u16)
        .min(inner_chunks[0].x + inner_chunks[0].width - 2);
    let cursor_y = inner_chunks[0].y + 1;
    f.set_cursor(cursor_x, cursor_y);

    // 2. Tạo danh sách checkbox
    if filtered.is_empty() {
        let empty_msg = Paragraph::new("❌ Không tìm thấy model nào phù hợp. Nhập chữ khác hoặc Esc để huỷ.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        f.render_widget(empty_msg, inner_chunks[1]);
    } else {
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|(_, name, checked, stale)| {
                let prefix = if *checked { "[x] " } else { "[ ] " };
                let style = Style::default().fg(Color::Gray);

                let span_prefix = Span::styled(
                    prefix,
                    if *checked {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                );
                let span_name = Span::raw(name.as_str());
                // Model bị provider xoá: cảnh báo đỏ để user thấy rõ sẽ bị xoá khỏi config
                let span_suffix = if *stale {
                    Span::styled(
                        "  (đã bị provider xoá — bỏ chọn để xoá khỏi config)",
                        Style::default().fg(Color::Red),
                    )
                } else {
                    Span::raw("")
                };

                ListItem::new(Line::from(vec![span_prefix, span_name, span_suffix])).style(style)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 50, 70))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        app.models_list_state.select(Some(app.selected_model_idx));
        f.render_stateful_widget(list, inner_chunks[1], &mut app.models_list_state);
    }

    let footer_text = " [Gõ chữ] Tìm kiếm | [Space] Chọn/Bỏ chọn | [Enter] Đồng bộ | [Esc] Huỷ\n [ĐỎ] Model bị provider xoá — bỏ chọn sẽ bị xoá khỏi config ";
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(footer, inner_chunks[2]);
}

// === AUTH KEYS MANAGER MODAL ===
fn draw_auth_keys_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(
            " 🔑 QUẢN LÝ API KEYS TRONG auth.json ({}) ",
            app.auth_keys.len()
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(popup_area);

    if app.auth_keys.is_empty() {
        let empty = Paragraph::new("Không có API key nào trong auth.json.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, inner_chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .auth_keys
            .iter()
            .map(|(name, val)| {
                let obfuscated = obfuscate_key(val);
                let line = Line::from(vec![
                    Span::styled(format!("{:<25}", name), Style::default().fg(Color::Cyan)),
                    Span::styled(obfuscated, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(line).style(Style::default().fg(Color::Gray))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 50, 70))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        app.auth_keys_list_state.select(Some(app.selected_auth_idx));
        f.render_stateful_widget(list, inner_chunks[0], &mut app.auth_keys_list_state);
    }

    let footer_text = " [Delete] Xoá API Key thừa | [Esc] Đóng ";
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
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

    let sub_title =
        Paragraph::new("Chọn các API hỏng bên dưới để xoá hàng loạt:").style(Style::default().fg(Color::Yellow));
    f.render_widget(sub_title, inner_chunks[0]);

    let items: Vec<ListItem> = app
        .clean_list
        .iter()
        .map(|(id, name, status, checked)| {
            let prefix = if *checked { "[x] " } else { "[ ] " };
            let status_desc = match status {
                ApiStatus::InsufficientCredits(_) => "Hết tiền (402)",
                ApiStatus::InvalidKey(_) => "Lỗi Key (401/403)",
                _ => "Lỗi khác",
            };

            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    if *checked {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(format!("{:<15}", name), Style::default().fg(Color::White)),
                Span::styled(format!(" ({})", id), Style::default().fg(Color::DarkGray)),
                Span::raw(" - "),
                Span::styled(status_desc, Style::default().fg(Color::Yellow)),
            ]);

            ListItem::new(line).style(Style::default().fg(Color::Gray))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 50, 70))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    app.clean_list_state.select(Some(app.selected_clean_idx));
    f.render_stateful_widget(list, inner_chunks[1], &mut app.clean_list_state);

    let footer_text = " [Space] Chọn | [Enter] Thực hiện Xoá | [Esc] Huỷ ";
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD),
    );
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
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(popup_area);

    let prompt = match &app.confirm_action {
        Some(ConfirmAction::DeleteProvider(id)) => {
            format!("Bạn có chắc chắn muốn xoá\nProvider '{}' khỏi opencode.json?", id)
        }
        Some(ConfirmAction::DeleteAuthKey(name)) => {
            format!("Bạn có chắc chắn muốn xoá\nAPI Key '{}' khỏi auth.json?", name)
        }
        Some(ConfirmAction::CleanSelected) => "Bạn có chắc chắn muốn xoá tất cả\ncác API lỗi đã chọn?".to_string(),
        Some(ConfirmAction::OverwriteDuplicate {
            duplicate_id,
            duplicate_name,
        }) => {
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
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_chunks[1]);

    let yes_style = if app.confirm_focus_yes {
        Style::default()
            .bg(Color::Red)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::Rgb(50, 40, 40)).fg(Color::Gray)
    };
    let btn_yes = Paragraph::new(" CÓ (YES) ")
        .block(Block::default().borders(Borders::ALL).border_style(yes_style))
        .alignment(Alignment::Center);
    f.render_widget(btn_yes, btn_layout[0]);

    let no_style = if !app.confirm_focus_yes {
        Style::default()
            .bg(Color::Gray)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
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
    // Nếu màn hình nhỏ, tự động giãn rộng phần trăm diện tích để tránh lỗi tràn/cắt chữ
    let dynamic_percent_x = if r.width < 100 {
        90
    } else if r.width < 140 {
        75
    } else {
        percent_x
    };

    let dynamic_percent_y = if r.height < 30 {
        90
    } else if r.height < 45 {
        80
    } else {
        percent_y
    };

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - dynamic_percent_y) / 2),
            Constraint::Percentage(dynamic_percent_y),
            Constraint::Percentage((100 - dynamic_percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - dynamic_percent_x) / 2),
            Constraint::Percentage(dynamic_percent_x),
            Constraint::Percentage((100 - dynamic_percent_x) / 2),
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
    let search_paragraph = Paragraph::new(search_text).block(
        Block::default()
            .title(" 🔍 Nhập từ khoá tìm kiếm (Ví dụ: mimo, openai...) ")
            .borders(Borders::ALL)
            .border_style(search_style),
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
            .block(
                Block::default()
                    .title(format!(" Danh sách preset khớp ({}) ", filtered.len()))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
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

// === CKEY: KIỂM TRA THÔNG TIN TÀI KHOẢN (màn hình XEM — không có input) ===
fn draw_ckey_dashboard_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(70, 82, area);
    f.render_widget(Clear, popup_area);

    // Chưa có account key cho provider đang chọn → popup chọn/nhập key.
    if app.ckey_need_key {
        draw_ckey_need_key_popup(f, popup_area, app);
        return;
    }

    let block = Block::default()
        .title(" 🔑 KIỂM TRA THÔNG TIN TÀI KHOẢN CKEY ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Account key line
            Constraint::Min(6),    // Info (profile/stats/keys)
            Constraint::Min(5),    // Models list
            Constraint::Length(3), // Footer
        ])
        .split(popup_area);

    // 1. Account key đang dùng (masked) + phím tắt
    let current_key = app
        .selected_provider_id()
        .and_then(|pid| app.ckey_account_key(pid))
        .unwrap_or_default();
    let key_line = Paragraph::new(Line::from(vec![
        Span::styled(
            "Account key đang dùng: ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(mask_ckey_key(&current_key), Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .title(" 🔑 ACCOUNT KEY ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(key_line, inner_chunks[0]);

    // 2. Thông tin tài khoản (profile/usage/keys)
    let mut lines: Vec<Line> = Vec::new();
    if let Some(p) = &app.ckey_profile {
        lines.push(Line::from(Span::styled(
            format!(
                "👤 User: {} ({}) | Số dư: {} ({:.0}đ) | Email: {} | Tạo lúc: {}",
                p.username, p.name, p.balance, p.balance_raw, p.email, p.created_at
            ),
            Style::default().fg(Color::LightGreen),
        )));
    }
    if let Some(st) = &app.ckey_stats {
        lines.push(Line::from(Span::styled(
            format!(
                "📊 Usage: {} request ({} thành công) | {} token (in {}, out {}) | Cache: {}/{} | Chi phí: {}",
                st.requests,
                st.success_requests,
                st.total_tokens,
                st.prompt_tokens,
                st.completion_tokens,
                st.cache_read_tokens,
                st.cache_write_tokens,
                st.charged_vnd_text
            ),
            Style::default().fg(Color::Yellow),
        )));
    }
    if !app.ckey_keys.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("🔐 Có {} API key AI.", app.ckey_keys.len()),
            Style::default().fg(Color::Cyan),
        )));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "Chưa có dữ liệu — nhấn R để tải.",
            Style::default().fg(Color::DarkGray),
        )));
    }
    if app.ckey_loading {
        lines.push(Line::from(Span::styled(
            "⏳ Đang tải dữ liệu...",
            Style::default().fg(Color::Yellow),
        )));
    }
    if let Some(err) = &app.ckey_error {
        lines.push(Line::from(Span::styled(
            format!("⚠️ {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let info_paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" ℹ️ THÔNG TIN TÀI KHOẢN ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(info_paragraph, inner_chunks[1]);

    // 3. Danh sách model (tên + giá VND)
    let models_block = Block::default()
        .title(" 🤖 MODELS (giá VND / 1M token) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if app.ckey_models.is_empty() {
        let empty = Paragraph::new("Chưa có model nào được tải.")
            .block(models_block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, inner_chunks[2]);
    } else {
        let rows: Vec<Row> = app
            .ckey_models
            .iter()
            .take(inner_chunks[2].height.saturating_sub(3) as usize)
            .map(|m| {
                Row::new(vec![
                    Cell::from(m.public_name.clone()).style(Style::default().fg(Color::Yellow)),
                    Cell::from(format!("{}₫", format_vnd(m.input_price_per_million_vnd)))
                        .style(Style::default().fg(Color::Gray)),
                    Cell::from(format!("{}₫", format_vnd(m.output_price_per_million_vnd)))
                        .style(Style::default().fg(Color::Gray)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(55),
                Constraint::Percentage(22),
                Constraint::Percentage(23),
            ],
        )
        .header(Row::new(vec![
            Cell::from("Model").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Input").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Output").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]))
        .block(models_block);

        f.render_widget(table, inner_chunks[2]);
    }

    // 4. Footer (hướng dẫn phím)
    let footer_text = " [R] Tải lại | [I] Import model | [U] Lịch sử dùng | [Esc] Đóng ";
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(footer, inner_chunks[3]);
}

/// Popup chọn/nhập account key CKey (khi provider đang chọn chưa có account key).
/// KHÔNG hiển thị key đầy đủ, KHÔNG hiển thị URL.
fn draw_ckey_need_key_popup(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" 🔑 CHỌN / NHẬP ACCOUNT KEY CKEY ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Mode bar
            Constraint::Min(5),    // List / input
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Mode bar
    let mode_text = if app.ckey_pick_mode == CkeyPickMode::Choose {
        " Chế độ: CHỌN từ account key đã lưu — Tab để chuyển sang NHẬP MỚI "
    } else {
        " Chế độ: NHẬP account key mới — Tab để chuyển sang CHỌN "
    };
    let mode_para = Paragraph::new(mode_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD));
    f.render_widget(mode_para, inner[0]);

    match app.ckey_pick_mode {
        CkeyPickMode::Choose => {
            let list_block = Block::default()
                .title(" 📇 Account key đã lưu (mục cuối: nhập key mới) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));

            if app.ckey_account_options.is_empty() {
                let empty = Paragraph::new(
                    "Chưa có account key nào được lưu cho provider khác.\nNhấn Tab để chuyển sang nhập account key mới.",
                )
                .block(list_block)
                .style(Style::default().fg(Color::DarkGray));
                f.render_widget(empty, inner[1]);
            } else {
                let mut items: Vec<ListItem> = app
                    .ckey_account_options
                    .iter()
                    .enumerate()
                    .map(|(i, (pid, key))| {
                        let line = Line::from(vec![
                            Span::styled(format!("{:<18}", pid), Style::default().fg(Color::White)),
                            Span::styled(mask_ckey_key(key), Style::default().fg(Color::DarkGray)),
                            if app.ckey_pick_selected_idx == i {
                                Span::styled("  (đang dùng cho provider này)", Style::default().fg(Color::Green))
                            } else {
                                Span::raw("")
                            },
                        ]);
                        ListItem::new(line).style(Style::default().fg(Color::Gray))
                    })
                    .collect();

                // Mục cuối: nhập account key mới
                items.push(
                    ListItem::new(Line::from(Span::styled(
                        " ➕ Nhập account key mới...",
                        Style::default().fg(Color::Yellow),
                    )))
                    .style(Style::default().fg(Color::Gray)),
                );

                let list = List::new(items)
                    .block(list_block)
                    .highlight_style(
                        Style::default()
                            .bg(Color::Rgb(40, 50, 70))
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                let selected = app.ckey_pick_selected_idx.min(app.ckey_account_options.len());
                let mut state = ratatui::widgets::ListState::default();
                state.select(Some(selected));
                f.render_stateful_widget(list, inner[1], &mut state);
            }

            let footer_text = " [↑/↓] Chọn | [Enter] Dùng key / Nhập mới | [Tab] Đổi chế độ | [Esc] Đóng ";
            let footer = Paragraph::new(footer_text)
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(footer, inner[2]);
        }
        CkeyPickMode::New => {
            let input_para = Paragraph::new(format!(" {}{}", app.ckey_new_key_input, "█"))
                .block(
                Block::default()
                    .title(" ⌨️ Nhập account key mới (từ trang Profile ckey.vn) — Enter để lưu ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(input_para, inner[1]);
            let cx = (inner[1].x + 2 + app.ckey_new_key_input.len() as u16)
                .min(inner[1].x + inner[1].width - 2);
            f.set_cursor(cx, inner[1].y + 1);

            let footer_text = " [Gõ] Nhập account key | [Enter] Lưu & tải dữ liệu | [Tab] Đổi chế độ | [Esc] Đóng ";
            let footer = Paragraph::new(footer_text)
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(footer, inner[2]);
        }
    }
}

// === BULK ADD PROVIDERS (màn hình K — thêm nhanh nhiều provider) ===
fn draw_bulk_add_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(60, 62, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" ⚡ THÊM NHANH NHIỀU PROVIDER ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::LightCyan))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Endpoint
            Constraint::Length(8), // AI keys (multi-line)
            Constraint::Length(3), // Execute button
            Constraint::Length(3), // Footer
        ])
        .split(popup_area);

    // 1. Endpoint field
    let ep_focused = app.bulk_focus == BulkFocus::Endpoint;
    let ep_style = if ep_focused {
        if app.bulk_is_editing {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        }
    } else {
        Style::default().fg(Color::Gray)
    };
    let ep_val = format!(
        " {}{}",
        app.bulk_endpoint_input,
        if ep_focused && app.bulk_is_editing { "█" } else { "" }
    );
    let ep_hint = if ep_focused {
        if app.bulk_is_editing {
            " (Đang gõ... Nhấn Enter để hoàn tất) "
        } else {
            " (Nhấn Enter để chỉnh sửa) "
        }
    } else {
        ""
    };
    let ep_para = Paragraph::new(ep_val).block(
        Block::default()
            .title(format!(" 🌐 Endpoint (URL AI key){}", ep_hint))
            .borders(Borders::ALL)
            .border_style(ep_style),
    );
    f.render_widget(ep_para, inner_chunks[0]);
    if ep_focused && app.bulk_is_editing {
        let cx = (inner_chunks[0].x + 2 + app.bulk_endpoint_input.len() as u16)
            .min(inner_chunks[0].x + inner_chunks[0].width - 2);
        f.set_cursor(cx, inner_chunks[0].y + 1);
    }

    // 2. AI keys field (nhiều dòng)
    let keys_focused = app.bulk_focus == BulkFocus::Keys;
    let keys_style = if keys_focused {
        if app.bulk_is_editing {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        }
    } else {
        Style::default().fg(Color::Gray)
    };
    let keys_val = format!(
        " {}{}",
        app.bulk_keys_input,
        if keys_focused && app.bulk_is_editing { "█" } else { "" }
    );
    let keys_hint = if keys_focused {
        if app.bulk_is_editing {
            " (Đang gõ... Nhấn Enter để hoàn tất) "
        } else {
            " (Nhấn Enter để chỉnh sửa) "
        }
    } else {
        ""
    };
    let keys_para = Paragraph::new(keys_val).block(
        Block::default()
            .title(format!(" 🔑 AI keys (mỗi dòng 1 key){}", keys_hint))
            .borders(Borders::ALL)
            .border_style(keys_style),
    );
    f.render_widget(keys_para, inner_chunks[1]);
    if keys_focused && app.bulk_is_editing {
        let cx = (inner_chunks[1].x + 2 + app.bulk_keys_input.len() as u16)
            .min(inner_chunks[1].x + inner_chunks[1].width - 2);
        f.set_cursor(cx, inner_chunks[1].y + 1);
    }

    // 3. Execute button
    let exe_style = if app.bulk_focus == BulkFocus::Execute {
        Style::default().bg(Color::Green).fg(Color::Black)
    } else {
        Style::default().bg(Color::Rgb(40, 40, 50)).fg(Color::White)
    };
    let exe = Paragraph::new(" ▶ THỰC HIỆN (Enter) ")
        .block(Block::default().borders(Borders::ALL).border_style(exe_style))
        .alignment(Alignment::Center);
    f.render_widget(exe, inner_chunks[2]);

    // 4. Footer
    let footer_text = " [↑/↓] Chọn ô | [Enter] Sửa ô / Thực hiện | [Esc] Thoát ";
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(footer, inner_chunks[3]);
}

/// Mask account key dạng "ck-****abcd" (không hiển thị key đầy đủ).
fn mask_ckey_key(key: &str) -> String {
    if key.is_empty() {
        return "ck-****".to_string();
    }
    if key.len() <= 4 {
        return "ck-****".to_string();
    }
    format!("ck-****{}", &key[key.len() - 4..])
}

// === CKEY: IMPORT MODELS MODAL ===
fn draw_ckey_import_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(65, 78, area);
    f.render_widget(Clear, popup_area);

    let filtered = app.filtered_ckey_import();
    let checked_count = app.ckey_import_list.iter().filter(|(_, c, _, _, _)| *c).count();

    let block = Block::default()
        .title(format!(
            " 🤖 IMPORT MODEL CKEY → opencode.json (chọn {}/{}) ",
            checked_count,
            app.ckey_import_list.len()
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(5),    // List
            Constraint::Length(4), // Footer
        ])
        .split(popup_area);

    let search_style = Style::default().fg(Color::Yellow);
    let search_text = format!(" {}", app.ckey_import_query);
    let search_paragraph = Paragraph::new(search_text).block(
        Block::default()
            .title(" 🔍 Nhập từ khoá tìm kiếm model (Ví dụ: gpt, claude...) ")
            .borders(Borders::ALL)
            .border_style(search_style),
    );
    f.render_widget(search_paragraph, inner_chunks[0]);

    let cursor_x = (inner_chunks[0].x + 2 + app.ckey_import_query.len() as u16)
        .min(inner_chunks[0].x + inner_chunks[0].width - 2);
    let cursor_y = inner_chunks[0].y + 1;
    f.set_cursor(cursor_x, cursor_y);

    if filtered.is_empty() {
        let empty_msg = Paragraph::new("❌ Không tìm thấy model nào phù hợp.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        f.render_widget(empty_msg, inner_chunks[1]);
    } else {
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|(_, name, checked, stale, input_price, output_price)| {
                let prefix = if *checked { "[x] " } else { "[ ] " };
                let span_prefix = Span::styled(
                    prefix,
                    if *checked {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                );
                let span_name = Span::raw(name.as_str());
                let price_str = if !*stale {
                    format!(
                        "  (in {}₫/M · out {}₫/M)",
                        format_vnd(*input_price),
                        format_vnd(*output_price)
                    )
                } else {
                    String::new()
                };
                let span_price = Span::styled(price_str, Style::default().fg(Color::DarkGray));
                let span_stale = if *stale {
                    Span::styled(
                        "  (đã bị CKey xoá — bỏ chọn để xoá khỏi config)",
                        Style::default().fg(Color::Red),
                    )
                } else {
                    Span::raw("")
                };
                ListItem::new(Line::from(vec![span_prefix, span_name, span_price, span_stale]))
                    .style(Style::default().fg(Color::Gray))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 50, 70))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        app.ckey_import_list_state.select(Some(app.ckey_import_idx));
        f.render_stateful_widget(list, inner_chunks[1], &mut app.ckey_import_list_state);
    }

    let footer_text = " [Gõ chữ] Tìm kiếm | [Space] Chọn/Bỏ chọn | [Enter] Đồng bộ | [Esc] Huỷ\n [ĐỎ] Model bị CKey xoá — bỏ chọn sẽ bị xoá khỏi config ";
    let footer = Paragraph::new(footer_text).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(footer, inner_chunks[2]);
}

// === CKEY: USAGE HISTORY MODAL ===
fn draw_ckey_usage_modal(f: &mut Frame, area: Rect, app: &mut App) {
    let popup_area = centered_rect(72, 82, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(
            " 📊 LỊCH SỬ DÙNG AI CKEY (trang {}/{}) ",
            app.ckey_usage_page, app.ckey_usage_total_pages
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .bg(Color::Rgb(15, 20, 30));

    f.render_widget(block, popup_area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(popup_area);

    if app.ckey_usage.is_empty() {
        let empty_msg = Paragraph::new(if app.ckey_loading {
            "⏳ Đang tải lịch sử dùng AI..."
        } else {
            "Chưa có dữ liệu usage. Nhấn R để tải lại."
        })
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty_msg, inner_chunks[0]);
    } else {
        // Số dòng hiển thị được trong viewport
        let view_h = inner_chunks[0].height.saturating_sub(3) as usize;
        let start = app.ckey_usage_scroll.min(app.ckey_usage.len().saturating_sub(1));
        let end = (start + view_h).min(app.ckey_usage.len());

        let rows: Vec<Row> = app.ckey_usage[start..end]
            .iter()
            .map(|u| {
                let status_style = if u.status == "success" {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                Row::new(vec![
                    Cell::from(u.created_at_text.clone()).style(Style::default().fg(Color::DarkGray)),
                    Cell::from(u.model_name.clone()).style(Style::default().fg(Color::Yellow)),
                    Cell::from(u.request_path.clone()).style(Style::default().fg(Color::Gray)),
                    Cell::from(format!("{} ({}p/{}c)", u.total_tokens, u.prompt_tokens, u.completion_tokens))
                        .style(Style::default().fg(Color::Gray)),
                    Cell::from(format!("{}₫", format_vnd(u.charged_vnd))).style(Style::default().fg(Color::LightCyan)),
                    Cell::from(format!("{}ms", u.latency_ms)).style(Style::default().fg(Color::Gray)),
                    Cell::from(u.status.clone()).style(status_style),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(22),
                Constraint::Percentage(24),
                Constraint::Percentage(18),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(8),
                Constraint::Percentage(8),
            ],
        )
        .header(Row::new(vec![
            Cell::from("Thời gian").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Model").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Path").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Tokens").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Chi phí").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Latency").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Status").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::NONE));

        f.render_widget(table, inner_chunks[0]);
    }

    let footer_text = " [↑/↓] Cuộn | [←/→] Trang trước/sau | [R] Tải lại trang | [Esc] Đóng ";
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(footer, inner_chunks[1]);
}

fn format_vnd(v: f64) -> String {
    if v >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.0}k", v / 1_000.0)
    } else if v == v.trunc() {
        format!("{:.0}", v)
    } else {
        format!("{}", v)
    }
}
