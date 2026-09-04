/*
[INTEGRITY NOTES]
- Mục đích: Vẽ giao diện GUI BUILDER bằng ratatui.
- Trách nhiệm: Hai màn hình — chọn project và theo dõi build (thanh tiến trình
  tổng + thanh tiến trình cargo + log cuộn + tổng kết).
- Tương tác: Đọc trạng thái từ `app::App`, không tự thay đổi trạng thái.
*/

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Screen};
use crate::discovery::BuildKind;

pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Select => draw_select(f, app),
        Screen::Building => draw_building(f, app),
    }
}

/// Khung tiêu đề dùng chung cho mọi màn hình.
fn header(area: Rect, f: &mut Frame, subtitle: &str) {
    let title = Line::from(vec![
        Span::styled(
            " GUI BUILDER ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(subtitle, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(title), area);
}

fn draw_select(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tiêu đề
            Constraint::Min(5),    // danh sách
            Constraint::Length(3), // hướng dẫn
        ])
        .split(f.size());

    header(
        chunks[0],
        f,
        &format!(
            "{} project trong workspace — đã chọn {}",
            app.projects.len(),
            app.selected_count()
        ),
    );

    if app.projects.is_empty() {
        f.render_widget(
            Paragraph::new(
                "Không tìm thấy project nào có binary.\nKiểm tra lại `cargo metadata` ở workspace root.",
            )
            .block(Block::default().borders(Borders::ALL).title(" Project "))
            .wrap(Wrap { trim: true }),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = app
            .projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ticked = app.selected[i];
                let is_cursor = i == app.cursor;

                let mark = if ticked { "[x] " } else { "[ ] " };
                let kind_color = match p.kind {
                    BuildKind::Tauri { .. } => Color::Magenta,
                    BuildKind::Cargo => Color::Blue,
                };

                let mut style = Style::default();
                if is_cursor {
                    style = style
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD);
                }
                if ticked {
                    style = style.fg(Color::Green);
                }

                ListItem::new(Line::from(vec![
                    Span::styled(mark, style),
                    Span::styled(format!("{:<26}", p.bin_name), style),
                    Span::styled(
                        match p.kind {
                            BuildKind::Tauri { .. } => "[Tauri]",
                            BuildKind::Cargo => "[Cargo]",
                        },
                        Style::default().fg(kind_color),
                    ),
                    Span::styled(format!("  {}", p.rel_dir), Style::default().fg(Color::DarkGray)),
                ]))
                .style(style)
            })
            .collect();

        f.render_widget(
            List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Chọn project cần build "),
            ),
            chunks[1],
        );
    }

    let help = Paragraph::new(vec![
        Line::from(vec![
            key("↑/↓"), Span::raw(" di chuyển   "),
            key("Space"), Span::raw(" chọn/bỏ   "),
            key("a"), Span::raw(" chọn tất cả   "),
            key("Enter"), Span::raw(" build   "),
            key("q"), Span::raw(" thoát"),
        ]),
        Line::from(Span::styled(
            "App Tauri build qua `cargo tauri build` để frontend được nhúng vào binary.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Phím "));
    f.render_widget(help, chunks[2]);
}

fn key(k: &str) -> Span<'_> {
    Span::styled(
        k,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

fn draw_building(f: &mut Frame, app: &App) {
    let running = app.is_running();
    let has_results = !app.results.is_empty();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                                    // tiêu đề
            Constraint::Length(3),                                    // tiến trình tổng
            Constraint::Length(3),                                    // tiến trình cargo
            Constraint::Min(6),                                       // log
            Constraint::Length(if has_results { 4 } else { 0 }),      // tổng kết
            Constraint::Length(3),                                    // hướng dẫn
        ])
        .split(f.size());

    let name = app
        .current
        .and_then(|i| app.projects.get(i))
        .map(|p| p.bin_name.as_str())
        .unwrap_or("—");
    header(
        chunks[0],
        f,
        &format!(
            "{}  •  {}/{} project  •  {} cảnh báo",
            if running { "đang build" } else { "đã xong" },
            app.done_count,
            app.total_count,
            app.warnings
        ),
    );

    // ── Tiến trình tổng theo số project ─────────────────────────────────────
    let overall = if app.total_count == 0 {
        0.0
    } else {
        app.done_count as f64 / app.total_count as f64
    };
    f.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Tổng tiến trình — hiện tại: {name} ")),
            )
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(overall.clamp(0.0, 1.0))
            .label(format!("{}/{}", app.done_count, app.total_count)),
        chunks[1],
    );

    // ── Tiến trình cargo theo số crate đã biên dịch ─────────────────────────
    let (done, total) = app.progress;
    let ratio = if total == 0 {
        0.0
    } else {
        (done as f64 / total as f64).clamp(0.0, 1.0)
    };
    let unit = if app.current_unit.is_empty() {
        app.stage.clone()
    } else {
        format!("Compiling {}", app.current_unit)
    };
    f.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" cargo build "),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(ratio)
            .label(if total == 0 {
                unit.clone()
            } else {
                format!("{done}/{total} crate — {unit}")
            }),
        chunks[2],
    );

    // ── Log (tự cuộn xuống cuối) ────────────────────────────────────────────
    let log_height = chunks[3].height.saturating_sub(2) as usize;
    let start = app.logs.len().saturating_sub(log_height);
    let lines: Vec<Line> = app.logs[start..]
        .iter()
        .map(|l| {
            let color = if l.starts_with('✖') || l.contains("error") {
                Color::Red
            } else if l.starts_with('⚠') {
                Color::Yellow
            } else if l.starts_with('✔') {
                Color::Green
            } else if l.starts_with('▶') || l.starts_with("──") {
                Color::Cyan
            } else {
                Color::Gray
            };
            Line::from(Span::styled(l.clone(), Style::default().fg(color)))
        })
        .collect();

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Log ({} dòng) ", app.logs.len())),
        ),
        chunks[3],
    );

    // ── Tổng kết ────────────────────────────────────────────────────────────
    if has_results {
        let lines: Vec<Line> = app
            .results
            .iter()
            .map(|r| {
                Line::from(vec![
                    Span::styled(
                        if r.ok { "✔ " } else { "✖ " },
                        Style::default().fg(if r.ok { Color::Green } else { Color::Red }),
                    ),
                    Span::styled(
                        format!("{:<24}", r.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(r.message.clone()),
                ])
            })
            .collect();
        f.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Kết quả "))
                .wrap(Wrap { trim: true }),
            chunks[4],
        );
    }

    let help = if running {
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Đang build… ",
                Style::default().fg(Color::Yellow),
            ),
            key("Ctrl+C"),
            Span::raw(" thoát cứng (build vẫn có thể còn chạy)"),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            key("Esc"),
            Span::raw(" quay lại danh sách   "),
            key("q"),
            Span::raw(" thoát"),
        ]))
    };
    f.render_widget(
        help.block(Block::default().borders(Borders::ALL).title(" Phím ")),
        chunks[5],
    );
}
