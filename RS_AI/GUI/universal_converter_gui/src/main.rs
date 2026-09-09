use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

const PREFS_KEY: &str = "preferences";

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Language {
    English,
    Vietnamese,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum AppTheme {
    System,
    Light,
    Dark,
}

#[derive(Clone, Serialize, Deserialize)]
struct Preferences {
    language: Language,
    theme: AppTheme,
    font: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: Language::Vietnamese,
            theme: AppTheme::System,
            font: "Default".into(),
        }
    }
}

#[derive(Clone)]
struct AvailableFont {
    name: String,
    path: PathBuf,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct UniversalConverterApp {
    // navigation
    selected_tab: Tab,
    preferences: Preferences,
    available_fonts: Vec<AvailableFont>,

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
    Settings,
}

enum AsyncResult {
    DependenciesChecked(String),
    FileClassified(String),
    DirectoryScanned(String, usize),
    Error(String),
}

impl UniversalConverterApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        let preferences = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, PREFS_KEY))
            .unwrap_or_default();
        let available_fonts = available_fonts();
        let mut app = UniversalConverterApp {
            tx,
            rx,
            selected_tab: Tab::default(),
            preferences,
            available_fonts,
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
        };
        if app.preferences.font != "Default"
            && !app.available_fonts.iter().any(|font| font.name == app.preferences.font)
        {
            app.preferences.font = "Default".into();
        }
        app.apply_preferences(&cc.egui_ctx);
        app
    }
}

// ---------------------------------------------------------------------------
// Font setup: thêm font hệ thống hỗ trợ tiếng Việt + ký hiệu đặc biệt
// (egui mặc định thiếu glyph Latin Extended + một số emoji/symbol)
// ---------------------------------------------------------------------------

fn load_font(fonts: &mut egui::FontDefinitions, name: &str, candidates: &[&str], family: egui::FontFamily) {
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

fn font_candidates() -> &'static [(&'static str, &'static str)] {
    &[
        ("Noto Sans", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
        ("DejaVu Sans", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        (
            "Liberation Sans",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ),
        ("Arial Unicode", "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        ("Helvetica", "/System/Library/Fonts/Helvetica.ttc"),
        ("Segoe UI", "C:\\Windows\\Fonts\\segoeui.ttf"),
        ("Arial", "C:\\Windows\\Fonts\\arial.ttf"),
    ]
}

fn available_fonts() -> Vec<AvailableFont> {
    font_candidates()
        .iter()
        .filter(|(_, path)| Path::new(path).is_file())
        .map(|(name, path)| AvailableFont {
            name: (*name).into(),
            path: PathBuf::from(path),
        })
        .collect()
}

fn setup_fonts(ctx: &egui::Context, selected: Option<&AvailableFont>) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(font) = selected {
        if let Ok(bytes) = std::fs::read(&font.path) {
            let name = "selected_font".to_owned();
            fonts
                .font_data
                .insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, name);
        }
    }
    load_font(
        &mut fonts,
        "sans_fallback",
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", // Linux
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", // Linux alt
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf", // macOS
            "C:\\Windows\\Fonts\\segoeui.ttf",                 // Windows
            "C:\\Windows\\Fonts\\arial.ttf",                   // Windows alt
        ],
        egui::FontFamily::Proportional,
    );
    load_font(
        &mut fonts,
        "mono_fallback",
        &[
            "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf", // Linux (đủ tiếng Việt)
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",     // Linux alt
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf", // Linux alt 2
            "/System/Library/Fonts/Menlo.ttc",                         // macOS
            "C:\\Windows\\Fonts\\consola.ttf",                         // Windows
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
        Box::new(|cc| Ok(Box::new(UniversalConverterApp::new(cc)))),
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
                ui.label(self.tr(
                    "Convert popular media, document, and archive formats",
                    "Chuyển đổi media, tài liệu và archive định dạng phổ biến",
                ));
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
                let classify_label = self.tr("📄 Classify File", "📄 Phân loại file");
                let scan_label = self.tr("📁 Scan Directory", "📁 Quét thư mục");
                let settings_label = self.tr("⚙ Settings", "⚙ Cài đặt");
                ui.vertical_centered(|ui| {
                    ui.heading(self.tr("Features", "Chức năng"));
                });
                ui.separator();
                ui.add_space(4.0);

                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, "🏠 Dashboard");
                ui.selectable_value(&mut self.selected_tab, Tab::Dependencies, "🔧 Dependencies");
                ui.selectable_value(&mut self.selected_tab, Tab::ClassifyFile, classify_label);
                ui.selectable_value(&mut self.selected_tab, Tab::ScanDirectory, scan_label);
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, settings_label);
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::Dashboard => self.ui_dashboard(ui),
            Tab::Dependencies => self.ui_dependencies(ui),
            Tab::ClassifyFile => self.ui_classify_file(ui),
            Tab::ScanDirectory => self.ui_scan_directory(ui),
            Tab::Settings => self.ui_settings(ui),
        });

        // ── Bottom log ─────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(80.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.tr("Log:", "Nhật ký:"));
                    if ui.button(self.tr("Clear", "Xóa")).clicked() {
                        self.log.clear();
                    }
                });
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for line in &self.log {
                        ui.monospace(line);
                    }
                });
            });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PREFS_KEY, &self.preferences);
    }
}

// ---------------------------------------------------------------------------
// Async helpers
// ---------------------------------------------------------------------------

impl UniversalConverterApp {
    fn tr<'a>(&self, en: &'a str, vi: &'a str) -> &'a str {
        match self.preferences.language {
            Language::English => en,
            Language::Vietnamese => vi,
        }
    }

    fn apply_preferences(&self, ctx: &egui::Context) {
        ctx.set_theme(match self.preferences.theme {
            AppTheme::System => egui::ThemePreference::System,
            AppTheme::Light => egui::ThemePreference::Light,
            AppTheme::Dark => egui::ThemePreference::Dark,
        });
        let selected = self
            .available_fonts
            .iter()
            .find(|font| font.name == self.preferences.font);
        setup_fonts(ctx, selected);
    }

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

        ui.label(self.tr(
            "Convert media (video/audio/image), documents, and archives.",
            "Công cụ chuyển đổi media (video/audio/image), tài liệu và archive.",
        ));
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
        ui.label(self.tr(
            "👉 Select a tab on the left to begin.",
            "👉 Chọn tab bên trái để thao tác.",
        ));
    }

    // -----------------------------------------------------------------------
    // Tab: Dependencies
    // -----------------------------------------------------------------------

    fn ui_dependencies(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔧 Dependencies");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui
                .button(self.tr("🔧 Check Dependencies", "🔧 Kiểm tra dependencies"))
                .clicked()
            {
                self.check_dependencies(ui.ctx().clone());
            }
            ui.label(&self.deps_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(20, 22, 30))
                .corner_radius(4.0)
                .show(ui, |ui| {
                    match self.deps_is_ok {
                        Some(true) => {
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 200, 100),
                                self.tr("✓ OK - all dependencies found", "✓ OK - đủ dependencies"),
                            );
                        }
                        Some(false) => {
                            ui.colored_label(
                                egui::Color32::from_rgb(220, 80, 80),
                                self.tr("✗ Missing dependencies", "✗ Thiếu dependencies"),
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
            let result = rt.block_on(async { universal_converter::system::dependencies::check_all().await });
            drop(rt);
            match result {
                Ok(deps) => {
                    let text =
                        serde_json::to_string_pretty(&deps).unwrap_or_else(|e| format!("Serialize error: {}", e));
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
        ui.heading(self.tr("📄 Classify File", "📄 Phân loại file"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(self.tr("Path:", "Đường dẫn:"));
            ui.add(egui::TextEdit::singleline(&mut self.classify_path).desired_width(400.0));
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(self.tr("📄 Classify", "📄 Phân loại")).clicked() {
                self.classify_file(ui.ctx().clone());
            }
            ui.label(&self.classify_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
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
            let text = serde_json::to_string_pretty(&sc).unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::FileClassified(text));
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Scan Directory
    // -----------------------------------------------------------------------

    fn ui_scan_directory(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.tr("📁 Scan Directory", "📁 Quét thư mục"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(self.tr("Path:", "Đường dẫn:"));
            ui.add(egui::TextEdit::singleline(&mut self.scan_path).desired_width(400.0));
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(self.tr("📁 Scan", "📁 Quét")).clicked() {
                self.scan_directory(ui.ctx().clone());
            }
            ui.label(&self.scan_status);
            if !self.scan_count.is_empty() {
                ui.separator();
                ui.label(&self.scan_count);
            }
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
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
            match universal_converter::core::scanner::scan_directory(Path::new(&path), &allowed_types) {
                Ok(files) => {
                    let count = files.len();
                    let text =
                        serde_json::to_string_pretty(&files).unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::DirectoryScanned(text, count));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("Scan directory: {}", e)));
                }
            }
            ctx.request_repaint();
        });
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.tr("⚙ Settings", "⚙ Cài đặt"));
        ui.separator();
        ui.add_space(4.0);

        let old_preferences = self.preferences.clone();
        let system_label = self.tr("System", "Hệ thống");
        let light_label = self.tr("Light", "Sáng");
        let dark_label = self.tr("Dark", "Tối");
        let default_font_label = self.tr("Default", "Mặc định");
        egui::Grid::new("settings_grid")
            .num_columns(2)
            .spacing([20.0, 12.0])
            .show(ui, |ui| {
                ui.label(self.tr("Language", "Ngôn ngữ"));
                egui::ComboBox::from_id_salt("language")
                    .selected_text(match self.preferences.language {
                        Language::English => "English",
                        Language::Vietnamese => "Tiếng Việt",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.preferences.language, Language::English, "English");
                        ui.selectable_value(&mut self.preferences.language, Language::Vietnamese, "Tiếng Việt");
                    });
                ui.end_row();

                ui.label(self.tr("Theme", "Giao diện"));
                let theme_label = match self.preferences.theme {
                    AppTheme::System => self.tr("System", "Hệ thống"),
                    AppTheme::Light => self.tr("Light", "Sáng"),
                    AppTheme::Dark => self.tr("Dark", "Tối"),
                };
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(theme_label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.preferences.theme, AppTheme::System, system_label);
                        ui.selectable_value(&mut self.preferences.theme, AppTheme::Light, light_label);
                        ui.selectable_value(&mut self.preferences.theme, AppTheme::Dark, dark_label);
                    });
                ui.end_row();

                ui.label(self.tr("Font", "Phông chữ"));
                egui::ComboBox::from_id_salt("font")
                    .selected_text(&self.preferences.font)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.preferences.font, "Default".into(), default_font_label);
                        for font in &self.available_fonts {
                            ui.selectable_value(&mut self.preferences.font, font.name.clone(), &font.name);
                        }
                    });
                ui.end_row();
            });

        if old_preferences.language != self.preferences.language
            || old_preferences.theme != self.preferences.theme
            || old_preferences.font != self.preferences.font
        {
            self.apply_preferences(ui.ctx());
        }
        ui.add_space(12.0);
        ui.label(self.tr(
            "Changes apply immediately and are saved automatically.",
            "Thay đổi được áp dụng ngay và tự động lưu.",
        ));
        if self.available_fonts.is_empty() {
            ui.weak(self.tr(
                "No supported system fonts found; using the default font.",
                "Không tìm thấy font hệ thống được hỗ trợ; đang dùng font mặc định.",
            ));
        }
    }
}
