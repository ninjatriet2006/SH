use std::io::{self, stdout};
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Gauge},
    Terminal,
};
use sysinfo::System;

use crate::config::ConfigManager;
use crate::core::scanner::ScannedFile;


struct TuiApp {
    files: Vec<ScannedFile>,
    file_list_state: ListState,
    active_tab: usize,
    system: System,
    logs: Vec<String>,
    should_quit: bool,
    config_mgr: ConfigManager,
}

impl TuiApp {
    fn new(files: Vec<ScannedFile>) -> Self {
        let mut file_list_state = ListState::default();
        if !files.is_empty() {
            file_list_state.select(Some(0));
        }
        
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            files,
            file_list_state,
            active_tab: 0,
            system,
            logs: vec!["UNI_CONV2 Advanced Terminal GUI initialized.".to_string()],
            should_quit: false,
            config_mgr: ConfigManager::new(),
        }
    }

    fn refresh_system(&mut self) {
        self.system.refresh_cpu();
        self.system.refresh_memory();
    }
}

pub fn start_advanced_tui(scanned_files: Vec<ScannedFile>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(scanned_files);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("[❌] TUI Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(500);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => {
                        app.active_tab = (app.active_tab + 1) % 3;
                    }
                    KeyCode::Down => {
                        if app.active_tab == 0 && !app.files.is_empty() {
                            let i = match app.file_list_state.selected() {
                                Some(i) => {
                                    if i >= app.files.len() - 1 {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            app.file_list_state.select(Some(i));
                        }
                    }
                    KeyCode::Up => {
                        if app.active_tab == 0 && !app.files.is_empty() {
                            let i = match app.file_list_state.selected() {
                                Some(i) => {
                                    if i == 0 {
                                        app.files.len() - 1
                                    } else {
                                        i - 1
                                    }
                                }
                                None => 0,
                            };
                            app.file_list_state.select(Some(i));
                        }
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        // Trigger batch conversion asynchronously or log it
                        app.logs.push("Bắt đầu xử lý hàng loạt...".to_string());
                        // Currently logs to console/UI, we can connect this directly to runner
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.refresh_system();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn draw_ui(f: &mut ratatui::Frame, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(4)])
        .split(f.size());

    // Banner & Tab titles
    let titles = vec!["📁 Tệp Tin & Hàng Đợi", "⚙️ Cấu Hình Hệ Thống", "📊 Giám Sát CPU/RAM"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" UNI_CONV2 - DASHBOARD NÂNG CAO "))
        .select(app.active_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    // Main Workspace depending on active tab
    match app.active_tab {
        0 => draw_files_tab(f, chunks[1], app),
        1 => draw_config_tab(f, chunks[1], app),
        2 => draw_monitor_tab(f, chunks[1], app),
        _ => {}
    }

    // Hotkey Info Panel at the bottom
    let footer_text = vec![
        Span::raw(" Phím nóng: "),
        Span::styled("[Tab]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Chuyển Tab | "),
        Span::styled("[Up/Down]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Di chuyển | "),
        Span::styled("[C]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" Bắt đầu xử lý | "),
        Span::styled("[Q]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(" Thoát Giao Diện "),
    ];
    let footer = Paragraph::new(Line::from(footer_text))
        .block(Block::default().borders(Borders::ALL).title(" Trợ giúp nhanh "));
    f.render_widget(footer, chunks[2]);
}

fn draw_files_tab(f: &mut ratatui::Frame, area: Rect, app: &mut TuiApp) {
    let workspace = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // File list widget
    let items: Vec<ListItem> = app
        .files
        .iter()
        .map(|file| {
            let name = file.path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let type_str = format!("{:?}", file.file_type);
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!(" [{}] ", type_str), Style::default().fg(Color::Magenta)),
                    Span::raw(name),
                ]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Danh sách File được chọn "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, workspace[0], &mut app.file_list_state);

    // Detail Panel
    let selected_idx = app.file_list_state.selected().unwrap_or(0);
    let detail_text = if selected_idx < app.files.len() {
        let file = &app.files[selected_idx];
        vec![
            Line::from(vec![
                Span::styled("Đường dẫn: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(file.path.to_string_lossy().into_owned()),
            ]),
            Line::from(vec![
                Span::styled("Loại file: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", file.file_type)),
            ]),
            Line::from(vec![
                Span::styled("Định dạng gốc: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&file.extension),
            ]),
        ]
    } else {
        vec![Line::from(Span::raw("Chưa có tệp tin nào được chọn"))]
    };

    let detail_block = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(" Chi tiết tệp tin "));
    f.render_widget(detail_block, workspace[1]);
}

fn draw_config_tab(f: &mut ratatui::Frame, area: Rect, app: &mut TuiApp) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(area);

    let v_cfg = app.config_mgr.load_video_config().unwrap_or_default();
    let a_cfg = app.config_mgr.load_audio_config().unwrap_or_default();
    let arc_cfg = app.config_mgr.load_archive_config().unwrap_or_default();

    let config_lines = vec![
        Line::from(vec![
            Span::styled("🎬 CẤU HÌNH VIDEO:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  - Định dạng đích: "),
            Span::styled(&v_cfg.format, Style::default().fg(Color::Green)),
            Span::raw(" | Codec: "),
            Span::styled(&v_cfg.codec, Style::default().fg(Color::Green)),
            Span::raw(" | Tốc độ khung hình: "),
            Span::styled(format!("{:?}", v_cfg.fps.unwrap_or(30)), Style::default().fg(Color::Green)),
            Span::raw(" | Tăng tốc GPU: "),
            Span::styled(&v_cfg.hardware_accel, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw(""),
        ]),
        Line::from(vec![
            Span::styled("🎵 CẤU HÌNH AUDIO:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  - Định dạng đích: "),
            Span::styled(&a_cfg.format, Style::default().fg(Color::Green)),
            Span::raw(" | Bitrate: "),
            Span::styled(&a_cfg.bitrate, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw(""),
        ]),
        Line::from(vec![
            Span::styled("📦 CẤU HÌNH FILE NÉN (ARCHIVE):", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  - Định dạng nén: "),
            Span::styled(&arc_cfg.format, Style::default().fg(Color::Green)),
            Span::raw(" | Mức độ nén: "),
            Span::styled(arc_cfg.compression_level.to_string(), Style::default().fg(Color::Green)),
        ]),
    ];

    let config_panel = Paragraph::new(config_lines)
        .block(Block::default().borders(Borders::ALL).title(" Cấu hình hiện tại (Nạp từ YAML) "));
    f.render_widget(config_panel, main_layout[0]);

    let note = Paragraph::new("Mẹo: Để sửa cấu hình, bạn có thể chỉnh sửa trực tiếp các tệp YAML trong thư mục config hoặc chỉnh ở menu cấu hình ngoài.")
        .block(Block::default().borders(Borders::ALL).title(" Ghi chú "));
    f.render_widget(note, main_layout[1]);
}

fn draw_monitor_tab(f: &mut ratatui::Frame, area: Rect, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU Usage Gauge
    let cpus = app.system.cpus();
    let cpu_avg: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;
    let cpu_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Tổng hiệu năng sử dụng CPU "))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::ITALIC))
        .percent(cpu_avg as u16);
    f.render_widget(cpu_gauge, chunks[0]);

    // RAM Usage Gauge
    let total_mem = app.system.total_memory() as f32;
    let used_mem = app.system.used_memory() as f32;
    let mem_pct = if total_mem > 0.0 { (used_mem / total_mem * 100.0) as u16 } else { 0 };
    let mem_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Sử dụng Bộ nhớ RAM "))
        .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black).add_modifier(Modifier::ITALIC))
        .percent(mem_pct);
    f.render_widget(mem_gauge, chunks[1]);
}
