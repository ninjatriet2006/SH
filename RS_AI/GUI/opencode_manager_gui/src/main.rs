use eframe::egui;
use std::sync::mpsc;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct OpenCodeManagerApp {
    // navigation
    selected_tab: Tab,

    // config panel
    config_text: String,
    config_status: String,

    // auth panel
    auth_text: String,
    auth_status: String,

    // api test panel
    api_base_url: String,
    api_key: String,
    api_status_text: String,
    api_test_result: String,

    // models panel
    models_list: String,
    models_status: String,

    // output log
    log: Vec<String>,

    // persistent channel: tx cloned for each task, rx drained each frame
    tx: mpsc::Sender<AsyncResult>,
    rx: mpsc::Receiver<AsyncResult>,
}

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Configuration,
    Auth,
    ApiTest,
    Models,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Dashboard
    }
}

enum AsyncResult {
    ConfigLoaded(String),
    AuthLoaded(String),
    ApiTested(String),
    ModelsFetched(String),
    Error(String),
}

impl OpenCodeManagerApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        OpenCodeManagerApp {
            tx,
            rx,
            selected_tab: Tab::default(),
            config_text: String::new(),
            config_status: String::new(),
            auth_text: String::new(),
            auth_status: String::new(),
            api_base_url: String::new(),
            api_key: String::new(),
            api_status_text: String::new(),
            api_test_result: String::new(),
            models_list: String::new(),
            models_status: String::new(),
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
            .with_inner_size([900.0, 650.0])
            .with_title("OpenCode Manager — GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "opencode_manager_gui",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(OpenCodeManagerApp::new()))
        }),
    )
}

// ---------------------------------------------------------------------------
// egui app
// ---------------------------------------------------------------------------

impl eframe::App for OpenCodeManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Drain async results ───────────────────────────────────────────
        self.drain_async_results();

        // ── Top bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OpenCode Manager");
                ui.separator();
                ui.label("Quản lý cấu hình AI Provider cho OpenCode");
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
                ui.selectable_value(&mut self.selected_tab, Tab::Configuration, "⚙️ Config");
                ui.selectable_value(&mut self.selected_tab, Tab::Auth, "🔑 Auth");
                ui.selectable_value(&mut self.selected_tab, Tab::ApiTest, "🔍 API Test");
                ui.selectable_value(&mut self.selected_tab, Tab::Models, "📋 Models");
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.ui_dashboard(ui),
                Tab::Configuration => self.ui_configuration(ui),
                Tab::Auth => self.ui_auth(ui),
                Tab::ApiTest => self.ui_api_test(ui),
                Tab::Models => self.ui_models(ui),
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

impl OpenCodeManagerApp {
    fn drain_async_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                AsyncResult::ConfigLoaded(text) => {
                    self.config_text = text;
                    self.config_status = "✓ Đã tải".into();
                    self.log.push("✓ Config loaded".into());
                }
                AsyncResult::AuthLoaded(text) => {
                    self.auth_text = text;
                    self.auth_status = "✓ Đã tải".into();
                    self.log.push("✓ Auth loaded".into());
                }
                AsyncResult::ApiTested(text) => {
                    self.api_test_result = text;
                    self.api_status_text = "✓ Hoàn tất".into();
                    self.log.push("✓ API tested".into());
                }
                AsyncResult::ModelsFetched(text) => {
                    self.models_list = text;
                    self.models_status = "✓ Đã tải".into();
                    self.log.push("✓ Models fetched".into());
                }
                AsyncResult::Error(err) => {
                    self.log.push(format!("✗ Error: {}", err));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tab: Dashboard
// ---------------------------------------------------------------------------

impl OpenCodeManagerApp {
    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.separator();
        ui.add_space(4.0);

        ui.label("Công cụ quản lý cấu hình AI Provider cho OpenCode.");
        ui.add_space(12.0);

        egui::Grid::new("dashboard_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label("📂 Configuration:");
                ui.label(&self.config_status);
                ui.end_row();
                ui.label("🔑 Auth:");
                ui.label(&self.auth_status);
                ui.end_row();
                ui.label("📋 Models:");
                ui.label(&self.models_status);
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label("👉 Chọn tab bên trái để thao tác.");
    }

    // -----------------------------------------------------------------------
    // Tab: Configuration
    // -----------------------------------------------------------------------

    fn ui_configuration(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Configuration");
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
            match opencode_manager::config::OpencodeConfig::load() {
                Ok(cfg) => {
                    let text = serde_json::to_string_pretty(&cfg)
                        .unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::ConfigLoaded(text));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Config load: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Auth
    // -----------------------------------------------------------------------

    fn ui_auth(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔑 Auth Entries");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("🔑 Load Auth").clicked() {
                self.load_auth(ui.ctx().clone());
            }
            ui.label(&self.auth_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.auth_text);
                    });
            });
    }

    fn load_auth(&mut self, ctx: egui::Context) {
        self.auth_status = "⏳ Đang tải...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match opencode_manager::config::AuthEntry::load_config() {
                Ok(auth) => {
                    let text = serde_json::to_string_pretty(&auth)
                        .unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::AuthLoaded(text));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Auth load: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: API Test
    // -----------------------------------------------------------------------

    fn ui_api_test(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 API Test");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Base URL:");
            ui.text_edit_singleline(&mut self.api_base_url);
        });
        ui.horizontal(|ui| {
            ui.label("API Key:  ");
            ui.add(egui::TextEdit::singleline(&mut self.api_key).password(true));
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("🚀 Test API").clicked() {
                self.test_api(ui.ctx().clone());
            }

            if ui.button("📋 Fetch Models").clicked() {
                self.fetch_models(ui.ctx().clone());
            }

            ui.label(&self.api_status_text);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.api_test_result);
                    });
            });
    }

    fn test_api(&mut self, ctx: egui::Context) {
        let base_url = self.api_base_url.clone();
        let api_key = self.api_key.clone();
        if base_url.is_empty() || api_key.is_empty() {
            self.api_status_text = "⚠️ Nhập URL và API Key".into();
            return;
        }
        self.api_status_text = "⏳ Testing...".into();
        self.api_test_result.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let client = opencode_manager::api::ApiClient::new();
            let status = rt.block_on(async { client.test_api(&base_url, &api_key).await });
            drop(rt);
            let text = serde_json::to_string_pretty(&status)
                .unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::ApiTested(text));
            ctx.request_repaint();
        });
    }

    fn fetch_models(&mut self, ctx: egui::Context) {
        let base_url = self.api_base_url.clone();
        let api_key = self.api_key.clone();
        if base_url.is_empty() || api_key.is_empty() {
            self.api_status_text = "⚠️ Nhập URL và API Key".into();
            return;
        }
        self.models_status = "⏳ Fetching...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let client = opencode_manager::api::ApiClient::new();
            let result = rt.block_on(async { client.fetch_models(&base_url, &api_key).await });
            drop(rt);
            match result {
                Ok(models) => {
                    let text = models.join("\n");
                    let _ = tx.send(AsyncResult::ModelsFetched(text));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Fetch models: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Models
    // -----------------------------------------------------------------------

    fn ui_models(&mut self, ui: &mut egui::Ui) {
        ui.heading("📋 Models");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("📋 Fetch Models").clicked() {
                self.fetch_models(ui.ctx().clone());
            }
            ui.label(&self.models_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.models_list);
                    });
            });
    }
}
