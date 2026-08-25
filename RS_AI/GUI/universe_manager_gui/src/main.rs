use eframe::egui;
use std::sync::mpsc;

use universe_manager::config::{AppEntry, Config};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct UniverseManagerApp {
    // navigation
    selected_tab: Tab,

    // config panel
    config_text: String,
    config_status: String,

    // scan panel
    scan_text: String,
    scan_status: String,

    // app manager panel
    detect_path: String,
    detect_text: String,
    detect_status: String,
    apps_list: Vec<AppEntry>,
    app_action_status: String,

    // search panel
    search_query: String,
    search_text: String,
    search_status: String,

    // output log
    log: Vec<String>,

    // persistent channel: tx cloned for each task, rx drained each frame
    tx: mpsc::Sender<AsyncResult>,
    rx: mpsc::Receiver<AsyncResult>,
}

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Config,
    ScanApps,
    AppManager,
    Search,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Dashboard
    }
}

enum AsyncResult {
    ConfigLoaded(String, Config),
    AppsScanned(String, Vec<AppEntry>),
    AppDetected(String),
    AppStarted(String),
    AppStopped(String),
    SearchCompleted(String),
    Error(String),
}

impl UniverseManagerApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        UniverseManagerApp {
            tx,
            rx,
            selected_tab: Tab::default(),
            config_text: String::new(),
            config_status: String::new(),
            scan_text: String::new(),
            scan_status: String::new(),
            detect_path: String::new(),
            detect_text: String::new(),
            detect_status: String::new(),
            apps_list: Vec::new(),
            app_action_status: String::new(),
            search_query: String::new(),
            search_text: String::new(),
            search_status: String::new(),
            log: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Font setup: thêm font hệ thống hỗ trợ tiếng Việt + ký hiệu đặc biệt
// (egui mặc định thiếu glyph Latin Extended + một số emoji/symbol)
// ---------------------------------------------------------------------------

fn load_font(
    fonts: &mut egui::FontDefinitions,
    name: &str,
    candidates: &[&str],
    family: egui::FontFamily,
) {
    // Load ALL existing fonts (fallback chain tích lũy: font sau bù glyph font trước)
    for (i, path) in candidates.iter().enumerate() {
        if std::path::Path::new(path).exists() {
            if let Ok(bytes) = std::fs::read(path) {
                let font_name = format!("{name}_{i}");
                fonts.font_data.insert(
                    font_name.clone(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                if let Some(list) = fonts.families.get_mut(&family) {
                    list.push(font_name);
                }
            }
        }
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    load_font(
        &mut fonts,
        "sans_fallback",
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",                 // Linux
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", // Linux alt
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",            // macOS
            "C:\\Windows\\Fonts\\segoeui.ttf",                                 // Windows
            "C:\\Windows\\Fonts\\arial.ttf",                                   // Windows alt
        ],
        egui::FontFamily::Proportional,
    );
    load_font(
        &mut fonts,
        "mono_fallback",
        &[
            "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",         // Linux (đủ tiếng Việt)
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",             // Linux alt
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf", // Linux alt 2
            "/System/Library/Fonts/Menlo.ttc",                                 // macOS
            "C:\\Windows\\Fonts\\consola.ttf",                                 // Windows
        ],
        egui::FontFamily::Monospace,
    );
    // Ký hiệu đặc biệt (emoji đơn sắc, mũi tên, dấu kiểm...)
    load_font(
        &mut fonts,
        "symbols_fallback",
        &[
            "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf", // Linux
            "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",  // Linux alt
            "/System/Library/Fonts/Apple Symbols.ttf",                     // macOS
            "C:\\Windows\\Fonts\\seguiemj.ttf",                            // Windows
        ],
        egui::FontFamily::Proportional,
    );

    ctx.set_fonts(fonts);
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 680.0])
            .with_title("Universe Manager — GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "universe_manager_gui",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(UniverseManagerApp::new()))
        }),
    )
}

// ---------------------------------------------------------------------------
// egui app
// ---------------------------------------------------------------------------

impl eframe::App for UniverseManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Drain async results ───────────────────────────────────────────
        self.drain_async_results();

        // ── Top bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Universe Manager");
                ui.separator();
                ui.label("Quản lý ứng dụng hệ thống: cấu hình, quét, phát hiện, khởi chạy, tìm kiếm");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("v0.1.0");
                });
            });
        });

        // ── Left nav ──────────────────────────────────────────────────────
        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .default_width(170.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Chức năng");
                });
                ui.separator();
                ui.add_space(4.0);

                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, "🏠 Dashboard");
                ui.selectable_value(&mut self.selected_tab, Tab::Config, "⚙️ Config");
                ui.selectable_value(&mut self.selected_tab, Tab::ScanApps, "📡 Scan Apps");
                ui.selectable_value(&mut self.selected_tab, Tab::AppManager, "🛠️ App Manager");
                ui.selectable_value(&mut self.selected_tab, Tab::Search, "🔍 Search");
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.ui_dashboard(ui),
                Tab::Config => self.ui_config(ui),
                Tab::ScanApps => self.ui_scan_apps(ui),
                Tab::AppManager => self.ui_app_manager(ui),
                Tab::Search => self.ui_search(ui),
            }
        });

        // ── Bottom log ─────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(80.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Log:");
                    if ui.button("Clear").clicked() {
                        self.log.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &self.log {
                            ui.monospace(line);
                        }
                    });
            });
    }
}

// ---------------------------------------------------------------------------
// Async helpers
// ---------------------------------------------------------------------------

impl UniverseManagerApp {
    fn drain_async_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                AsyncResult::ConfigLoaded(text, cfg) => {
                    self.config_text = text;
                    self.config_status = format!("✓ Đã tải ({} ứng dụng)", cfg.apps.len());
                    self.apps_list = cfg.apps;
                    self.log.push("✓ Config loaded".into());
                }
                AsyncResult::AppsScanned(text, apps) => {
                    self.scan_text = text;
                    self.scan_status = format!("✓ Đã quét ({} ứng dụng)", apps.len());
                    self.apps_list = apps;
                    self.log.push("✓ Apps scanned".into());
                }
                AsyncResult::AppDetected(text) => {
                    self.detect_text = text;
                    self.detect_status = "✓ Hoàn tất".into();
                    self.log.push("✓ App detected".into());
                }
                AsyncResult::AppStarted(msg) => {
                    self.app_action_status = msg.clone();
                    self.log.push(format!("▶️ {}", msg));
                }
                AsyncResult::AppStopped(msg) => {
                    self.app_action_status = msg.clone();
                    self.log.push(format!("⏹ {}", msg));
                }
                AsyncResult::SearchCompleted(text) => {
                    self.search_text = text;
                    self.search_status = "✓ Hoàn tất".into();
                    self.log.push("✓ Search completed".into());
                }
                AsyncResult::Error(err) => {
                    self.app_action_status = format!("✗ {}", err);
                    self.log.push(format!("✗ Error: {}", err));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tab: Dashboard
// ---------------------------------------------------------------------------

impl UniverseManagerApp {
    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.separator();
        ui.add_space(4.0);

        ui.label("Công cụ quản lý ứng dụng hệ thống: tích hợp, quét, phát hiện và khởi chạy ứng dụng.");
        ui.add_space(12.0);

        egui::Grid::new("dashboard_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label("⚙️ Config:");
                ui.label(&self.config_status);
                ui.end_row();
                ui.label("📡 Scan Apps:");
                ui.label(&self.scan_status);
                ui.end_row();
                ui.label("🛠️ App Manager:");
                ui.label(&self.app_action_status);
                ui.end_row();
                ui.label("🔍 Search:");
                ui.label(&self.search_status);
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label("👉 Chọn tab bên trái để thao tác.");
    }

    // -----------------------------------------------------------------------
    // Tab: Config
    // -----------------------------------------------------------------------

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Config");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("📂 Load Config").clicked() {
                self.load_config(ui.ctx().clone());
            }
            ui.label(&self.config_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.config_text);
                    });
            });
    }

    fn load_config(&mut self, ctx: egui::Context) {
        self.config_status = "⏳ Đang tải...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let cfg = universe_manager::config::Config::load();
            let text = serde_json::to_string_pretty(&cfg)
                .unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::ConfigLoaded(text, cfg));
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Scan Apps
    // -----------------------------------------------------------------------

    fn ui_scan_apps(&mut self, ui: &mut egui::Ui) {
        ui.heading("📡 Scan Apps");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("🔍 Scan").clicked() {
                self.scan_apps(ui.ctx().clone());
            }
            ui.label(&self.scan_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.scan_text);
                    });
            });
    }

    fn scan_apps(&mut self, ctx: egui::Context) {
        self.scan_status = "⏳ Đang quét...".into();
        self.scan_text.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let apps = universe_manager::scanner::scan_all_system_apps();
            let text = serde_json::to_string_pretty(&apps)
                .unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::AppsScanned(text, apps));
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: App Manager
    // -----------------------------------------------------------------------

    fn ui_app_manager(&mut self, ui: &mut egui::Ui) {
        ui.heading("🛠️ App Manager");
        ui.separator();
        ui.add_space(4.0);

        // Detect App section
        ui.horizontal(|ui| {
            ui.label("Đường dẫn (file/thư mục):");
            ui.add(egui::TextEdit::singleline(&mut self.detect_path).desired_width(320.0));
            if ui.button("🔎 Detect").clicked() {
                self.detect_app(ui.ctx().clone());
            }
            ui.label(&self.detect_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.detect_text);
                    });
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);

        // App list section: start/stop
        ui.horizontal(|ui| {
            ui.label(format!("Ứng dụng đã biết ({}):", self.apps_list.len()));
            ui.label(&self.app_action_status);
        });

        if self.apps_list.is_empty() {
            ui.label("Chưa có dữ liệu. Hãy dùng tab ⚙️ Config (Load Config) hoặc 📡 Scan Apps (Scan) trước.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for i in 0..self.apps_list.len() {
                        let app = self.apps_list[i].clone();
                        ui.horizontal(|ui| {
                            ui.label(&app.name);
                            ui.separator();
                            ui.label(&app.id);
                            ui.separator();
                            ui.monospace(&app.exec_path);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("⏹ Stop").clicked() {
                                        self.stop_app(ui.ctx().clone(), app.clone());
                                    }
                                    if ui.button("▶️ Start").clicked() {
                                        self.start_app(ui.ctx().clone(), app.clone());
                                    }
                                },
                            );
                        });
                    }
                });
        }
    }

    fn detect_app(&mut self, ctx: egui::Context) {
        let path = self.detect_path.trim().to_string();
        if path.is_empty() {
            self.detect_status = "⚠️ Vui lòng nhập đường dẫn".into();
            return;
        }
        self.detect_status = "⏳ Đang phát hiện...".into();
        self.detect_text.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match universe_manager::detector::detect(&path) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::AppDetected(text));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Detect error: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }

    fn start_app(&mut self, ctx: egui::Context, app: AppEntry) {
        let name = app.name.clone();
        self.app_action_status = format!("⏳ Đang khởi động {}...", name);
        self.log.push(format!("▶️ Start: {}", name));
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match universe_manager::manager::start_app(&app) {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::AppStarted(format!("✓ Đã khởi động: {}", name)));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Start {}: {}", name, e)));
                }
            }
            ctx.request_repaint();
        });
    }

    fn stop_app(&mut self, ctx: egui::Context, app: AppEntry) {
        let name = app.name.clone();
        self.app_action_status = format!("⏳ Đang dừng {}...", name);
        self.log.push(format!("⏹ Stop: {}", name));
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match universe_manager::manager::stop_app(&app) {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::AppStopped(format!("✓ Đã dừng: {}", name)));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Stop {}: {}", name, e)));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Search
    // -----------------------------------------------------------------------

    fn ui_search(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Search");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Từ khóa:");
            ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(280.0));
            if ui.button("🔍 Search").clicked() {
                self.search_apps(ui.ctx().clone());
            }
            ui.label(&self.search_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.search_text);
                    });
            });
    }

    fn search_apps(&mut self, ctx: egui::Context) {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            self.search_status = "⚠️ Vui lòng nhập từ khóa".into();
            return;
        }
        self.search_status = "⏳ Đang tìm kiếm...".into();
        self.search_text.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let results = universe_manager::installer::search_apps(&query);
            let text = serde_json::to_string_pretty(&results)
                .unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::SearchCompleted(text));
            ctx.request_repaint();
        });
    }
}
