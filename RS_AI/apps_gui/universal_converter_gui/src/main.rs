use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct UniversalConverterApp {
    // navigation
    selected_tab: Tab,

    // dependencies panel
    deps_status: String,
    deps_result: String,
    deps_is_ok: Option<bool>,

    // classify file panel
    classify_path: String,
    classify_status: String,
    classify_result: String,

    // scan directory panel
    scan_path: String,
    scan_status: String,
    scan_result: String,
    scan_count: String,

    // output log
    log: Vec<String>,

    // persistent channel: tx cloned for each task, rx drained each frame
    tx: mpsc::Sender<AsyncResult>,
    rx: mpsc::Receiver<AsyncResult>,
}

#[derive(PartialEq, Default)]
enum Tab {
    #[default]
    Dashboard,
    Dependencies,
    ClassifyFile,
    ScanDirectory,
}

enum AsyncResult {
    DependenciesChecked(String),
    FileClassified(String),
    DirectoryScanned(String, usize),
    Error(String),
}

impl UniversalConverterApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        UniversalConverterApp {
            tx,
            rx,
            selected_tab: Tab::default(),
            deps_status: String::new(),
            deps_result: String::new(),
            deps_is_ok: None,
            classify_path: String::new(),
            classify_status: String::new(),
            classify_result: String::new(),
            scan_path: String::new(),
            scan_status: String::new(),
            scan_result: String::new(),
            scan_count: String::new(),
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
            .with_title("Universal Converter — GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "universal_converter_gui",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(UniversalConverterApp::new()))
        }),
    )
}

// ---------------------------------------------------------------------------
// egui app
// ---------------------------------------------------------------------------

impl eframe::App for UniversalConverterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Drain async results ───────────────────────────────────────────
        self.drain_async_results();

        // ── Top bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Universal Converter");
                ui.separator();
                ui.label("Chuyển đổi media, tài liệu và archive định dạng phổ biến");
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
                ui.selectable_value(&mut self.selected_tab, Tab::Dependencies, "🔧 Dependencies");
                ui.selectable_value(&mut self.selected_tab, Tab::ClassifyFile, "📄 Classify File");
                ui.selectable_value(&mut self.selected_tab, Tab::ScanDirectory, "📁 Scan Directory");
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.ui_dashboard(ui),
                Tab::Dependencies => self.ui_dependencies(ui),
                Tab::ClassifyFile => self.ui_classify_file(ui),
                Tab::ScanDirectory => self.ui_scan_directory(ui),
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

impl UniversalConverterApp {
    fn drain_async_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                AsyncResult::DependenciesChecked(text) => {
                    self.deps_result = text.clone();
                    self.deps_is_ok = serde_json::from_str::<serde_json::Value>(&text)
                        .ok()
                        .and_then(|v| v.get("is_ok").and_then(|b| b.as_bool()));
                    self.deps_status = "✓ Đã kiểm tra".into();
                    self.log.push("✓ Dependencies checked".into());
                }
                AsyncResult::FileClassified(text) => {
                    self.classify_result = text;
                    self.classify_status = "✓ Đã phân loại".into();
                    self.log.push("✓ File classified".into());
                }
                AsyncResult::DirectoryScanned(text, count) => {
                    self.scan_result = text;
                    self.scan_count = format!("{} file", count);
                    self.scan_status = "✓ Đã quét".into();
                    self.log.push(format!("✓ Directory scanned: {} files", count));
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

impl UniversalConverterApp {
    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.separator();
        ui.add_space(4.0);

        ui.label("Công cụ chuyển đổi media (video/audio/image), tài liệu và archive.");
        ui.add_space(12.0);

        egui::Grid::new("dashboard_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label("🔧 Dependencies:");
                ui.label(&self.deps_status);
                ui.end_row();
                ui.label("📄 Classify File:");
                ui.label(&self.classify_status);
                ui.end_row();
                ui.label("📁 Scan Directory:");
                ui.label(&self.scan_status);
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label("👉 Chọn tab bên trái để thao tác.");
    }

    // -----------------------------------------------------------------------
    // Tab: Dependencies
    // -----------------------------------------------------------------------

    fn ui_dependencies(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔧 Dependencies");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("🔧 Check Dependencies").clicked() {
                self.check_dependencies(ui.ctx().clone());
            }
            ui.label(&self.deps_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        match self.deps_is_ok {
                            Some(true) => {
                                ui.colored_label(
                                    egui::Color32::from_rgb(100, 200, 100),
                                    "✓ OK — đủ dependencies",
                                );
                            }
                            Some(false) => {
                                ui.colored_label(
                                    egui::Color32::from_rgb(220, 80, 80),
                                    "✗ Thiếu dependencies",
                                );
                            }
                            None => {}
                        }
                        ui.add_space(4.0);
                        ui.monospace(&self.deps_result);
                    });
            });
    }

    fn check_dependencies(&mut self, ctx: egui::Context) {
        self.deps_status = "⏳ Đang kiểm tra...".into();
        self.deps_result.clear();
        self.deps_is_ok = None;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                universal_converter::system::dependencies::check_all().await
            });
            drop(rt);
            match result {
                Ok(deps) => {
                    let text = serde_json::to_string_pretty(&deps)
                        .unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::DependenciesChecked(text));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Check dependencies: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Classify File
    // -----------------------------------------------------------------------

    fn ui_classify_file(&mut self, ui: &mut egui::Ui) {
        ui.heading("📄 Classify File");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.add(
                egui::TextEdit::singleline(&mut self.classify_path).desired_width(400.0),
            );
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("📄 Classify").clicked() {
                self.classify_file(ui.ctx().clone());
            }
            ui.label(&self.classify_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.classify_result);
                    });
            });
    }

    fn classify_file(&mut self, ctx: egui::Context) {
        let path = self.classify_path.trim().to_string();
        if path.is_empty() {
            self.classify_status = "⚠️ Nhập đường dẫn".into();
            return;
        }
        self.classify_status = "⏳ Đang phân loại...".into();
        self.classify_result.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let sc = universal_converter::core::scanner::classify_file(PathBuf::from(path));
            let text = serde_json::to_string_pretty(&sc)
                .unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::FileClassified(text));
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Scan Directory
    // -----------------------------------------------------------------------

    fn ui_scan_directory(&mut self, ui: &mut egui::Ui) {
        ui.heading("📁 Scan Directory");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.add(egui::TextEdit::singleline(&mut self.scan_path).desired_width(400.0));
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("📁 Scan").clicked() {
                self.scan_directory(ui.ctx().clone());
            }
            ui.label(&self.scan_status);
            if !self.scan_count.is_empty() {
                ui.separator();
                ui.label(&self.scan_count);
            }
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.scan_result);
                    });
            });
    }

    fn scan_directory(&mut self, ctx: egui::Context) {
        let path = self.scan_path.trim().to_string();
        if path.is_empty() {
            self.scan_status = "⚠️ Nhập đường dẫn".into();
            return;
        }
        self.scan_status = "⏳ Đang quét...".into();
        self.scan_result.clear();
        self.scan_count.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // Video, Audio, Image, Document, Archive được bật; Directory, Unknown tắt
            let allowed_types = vec![true, true, true, true, true, false, false];
            match universal_converter::core::scanner::scan_directory(
                Path::new(&path),
                &allowed_types,
            ) {
                Ok(files) => {
                    let count = files.len();
                    let text = serde_json::to_string_pretty(&files)
                        .unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::DirectoryScanned(text, count));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Scan directory: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }
}
