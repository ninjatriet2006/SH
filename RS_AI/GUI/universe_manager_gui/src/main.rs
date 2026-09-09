use eframe::egui;
use std::path::{Path, PathBuf};
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

    // appearance and localization
    preferences: Preferences,
    available_fonts: Vec<FontOption>,
    settings_status: String,

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
    Settings,
}

#[derive(Clone, Copy, PartialEq)]
enum Language {
    English,
    Vietnamese,
}

#[derive(Clone, Copy, PartialEq)]
enum AppTheme {
    System,
    Light,
    Dark,
}

struct Preferences {
    language: Language,
    theme: AppTheme,
    font_path: Option<PathBuf>,
}

struct FontOption {
    name: String,
    path: PathBuf,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: Language::Vietnamese,
            theme: AppTheme::System,
            font_path: None,
        }
    }
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
    fn new(ctx: &egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut preferences = load_preferences();
        if preferences.font_path.as_ref().is_some_and(|path| !path.is_file()) {
            preferences.font_path = None;
        }
        apply_preferences(ctx, &preferences);
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
            preferences,
            available_fonts: available_fonts(),
            settings_status: String::new(),
        }
    }

    fn text(&self, key: &'static str) -> &'static str {
        text(self.preferences.language, key)
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

fn setup_fonts(ctx: &egui::Context, selected_font: Option<&Path>) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some(path) = selected_font {
        if let Ok(bytes) = std::fs::read(path) {
            let name = "selected_font".to_owned();
            fonts
                .font_data
                .insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, name);
            }
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

fn apply_preferences(ctx: &egui::Context, preferences: &Preferences) {
    setup_fonts(ctx, preferences.font_path.as_deref());
    ctx.set_theme(match preferences.theme {
        AppTheme::System => egui::ThemePreference::System,
        AppTheme::Light => egui::ThemePreference::Light,
        AppTheme::Dark => egui::ThemePreference::Dark,
    });
}

fn preferences_path() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(path).join("universe_manager_gui/preferences.conf");
    }
    if cfg!(target_os = "windows") {
        if let Some(path) = std::env::var_os("APPDATA") {
            return PathBuf::from(path).join("universe_manager_gui/preferences.conf");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config/universe_manager_gui/preferences.conf");
    }
    PathBuf::from("universe_manager_gui.preferences.conf")
}

fn load_preferences() -> Preferences {
    let mut preferences = Preferences::default();
    let Ok(contents) = std::fs::read_to_string(preferences_path()) else {
        return preferences;
    };
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "language" => {
                preferences.language = if value == "en" {
                    Language::English
                } else {
                    Language::Vietnamese
                };
            }
            "theme" => {
                preferences.theme = match value {
                    "light" => AppTheme::Light,
                    "dark" => AppTheme::Dark,
                    _ => AppTheme::System,
                };
            }
            "font" if !value.is_empty() => preferences.font_path = Some(PathBuf::from(value)),
            _ => {}
        }
    }
    preferences
}

fn save_preferences(preferences: &Preferences) -> std::io::Result<()> {
    let path = preferences_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let language = match preferences.language {
        Language::English => "en",
        Language::Vietnamese => "vi",
    };
    let theme = match preferences.theme {
        AppTheme::System => "system",
        AppTheme::Light => "light",
        AppTheme::Dark => "dark",
    };
    let font = preferences
        .font_path
        .as_deref()
        .map_or_else(String::new, |path| path.to_string_lossy().into_owned());
    std::fs::write(path, format!("language={language}\ntheme={theme}\nfont={font}\n"))
}

fn available_fonts() -> Vec<FontOption> {
    let mut roots = Vec::new();
    if cfg!(target_os = "linux") {
        roots.extend([
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ]);
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(&home).join(".fonts"));
            roots.push(PathBuf::from(home).join(".local/share/fonts"));
        }
    } else if cfg!(target_os = "macos") {
        roots.extend([PathBuf::from("/System/Library/Fonts"), PathBuf::from("/Library/Fonts")]);
    } else if cfg!(target_os = "windows") {
        if let Some(windir) = std::env::var_os("WINDIR") {
            roots.push(PathBuf::from(windir).join("Fonts"));
        }
    }

    let mut paths = Vec::new();
    for root in roots {
        collect_fonts(&root, &mut paths);
    }
    paths.sort();
    paths.dedup();

    let mut fonts: Vec<_> = paths
        .into_iter()
        .map(|path| FontOption {
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(['-', '_'], " "),
            path,
        })
        .collect();
    fonts.sort_by_key(|font| font.name.to_lowercase());
    fonts
}

fn collect_fonts(directory: &Path, fonts: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_fonts(&path, fonts);
        } else if path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
            ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf") || ext.eq_ignore_ascii_case("ttc")
        }) {
            fonts.push(path);
        }
    }
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
        Box::new(|cc| Ok(Box::new(UniverseManagerApp::new(&cc.egui_ctx)))),
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
                ui.label(self.text("subtitle"));
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
                    ui.heading(self.text("functions"));
                });
                ui.separator();
                ui.add_space(4.0);

                let dashboard_label = format!("🏠 {}", self.text("dashboard"));
                let config_label = format!("⚙ {}", self.text("config"));
                let scan_label = format!("📡 {}", self.text("scan_apps"));
                let manager_label = format!("🛠 {}", self.text("app_manager"));
                let search_label = format!("🔍 {}", self.text("search"));
                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, dashboard_label);
                ui.selectable_value(&mut self.selected_tab, Tab::Config, config_label);
                ui.selectable_value(&mut self.selected_tab, Tab::ScanApps, scan_label);
                ui.selectable_value(&mut self.selected_tab, Tab::AppManager, manager_label);
                ui.selectable_value(&mut self.selected_tab, Tab::Search, search_label);
                let settings_label = format!("⚙ {}", self.text("settings"));
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, settings_label);
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::Dashboard => self.ui_dashboard(ui),
            Tab::Config => self.ui_config(ui),
            Tab::ScanApps => self.ui_scan_apps(ui),
            Tab::AppManager => self.ui_app_manager(ui),
            Tab::Search => self.ui_search(ui),
            Tab::Settings => self.ui_settings(ui, ctx),
        });

        // ── Bottom log ─────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(80.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(self.text("log"));
                    if ui.button(self.text("clear")).clicked() {
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
}

impl UniverseManagerApp {
    fn ui_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading(self.text("settings"));
        ui.separator();
        ui.add_space(8.0);

        let mut changed = false;
        egui::Grid::new("settings_grid")
            .num_columns(2)
            .spacing([24.0, 12.0])
            .show(ui, |ui| {
                ui.label(self.text("language"));
                let language_name = match self.preferences.language {
                    Language::English => "English",
                    Language::Vietnamese => "Tiếng Việt",
                };
                egui::ComboBox::from_id_salt("language_selector")
                    .selected_text(language_name)
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut self.preferences.language, Language::English, "English")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut self.preferences.language, Language::Vietnamese, "Tiếng Việt")
                            .changed();
                    });
                ui.end_row();

                ui.label(self.text("theme"));
                let theme_name = match self.preferences.theme {
                    AppTheme::System => self.text("system"),
                    AppTheme::Light => self.text("light"),
                    AppTheme::Dark => self.text("dark"),
                };
                egui::ComboBox::from_id_salt("theme_selector")
                    .selected_text(theme_name)
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut self.preferences.theme,
                                AppTheme::System,
                                text(self.preferences.language, "system"),
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut self.preferences.theme,
                                AppTheme::Light,
                                text(self.preferences.language, "light"),
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut self.preferences.theme,
                                AppTheme::Dark,
                                text(self.preferences.language, "dark"),
                            )
                            .changed();
                    });
                ui.end_row();

                ui.label(self.text("font"));
                let selected_font = self
                    .preferences
                    .font_path
                    .as_ref()
                    .and_then(|path| self.available_fonts.iter().find(|font| &font.path == path))
                    .map_or_else(|| self.text("default_font").to_owned(), |font| font.name.clone());
                let mut next_font = self.preferences.font_path.clone();
                egui::ComboBox::from_id_salt("font_selector")
                    .width(300.0)
                    .selected_text(selected_font)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut next_font, None, self.text("default_font"));
                        for font in &self.available_fonts {
                            ui.selectable_value(&mut next_font, Some(font.path.clone()), &font.name);
                        }
                    });
                if next_font != self.preferences.font_path {
                    self.preferences.font_path = next_font;
                    changed = true;
                }
                ui.end_row();
            });

        if changed {
            apply_preferences(ctx, &self.preferences);
            self.settings_status = match save_preferences(&self.preferences) {
                Ok(()) => self.text("saved").to_owned(),
                Err(error) => format!("{}: {error}", self.text("save_failed")),
            };
        }

        ui.add_space(8.0);
        ui.label(format!(
            "{}: {}",
            self.text("available_fonts"),
            self.available_fonts.len()
        ));
        if !self.settings_status.is_empty() {
            ui.label(&self.settings_status);
        }
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
                    self.config_status =
                        format!("✓ {} ({} {})", self.text("loaded"), cfg.apps.len(), self.text("apps"));
                    self.apps_list = cfg.apps;
                    self.log.push("✓ Config loaded".into());
                }
                AsyncResult::AppsScanned(text, apps) => {
                    self.scan_text = text;
                    self.scan_status = format!("✓ {} ({} {})", self.text("scanned"), apps.len(), self.text("apps"));
                    self.apps_list = apps;
                    self.log.push("✓ Apps scanned".into());
                }
                AsyncResult::AppDetected(text) => {
                    self.detect_text = text;
                    self.detect_status = format!("✓ {}", self.text("completed"));
                    self.log.push("✓ App detected".into());
                }
                AsyncResult::AppStarted(msg) => {
                    self.app_action_status = format!("✓ {}: {msg}", self.text("started"));
                    self.log.push(format!("▶ {}: {msg}", self.text("started")));
                }
                AsyncResult::AppStopped(msg) => {
                    self.app_action_status = format!("✓ {}: {msg}", self.text("stopped"));
                    self.log.push(format!("⏹ {}: {msg}", self.text("stopped")));
                }
                AsyncResult::SearchCompleted(text) => {
                    self.search_text = text;
                    self.search_status = format!("✓ {}", self.text("completed"));
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

fn text(language: Language, key: &'static str) -> &'static str {
    match (language, key) {
        (Language::English, "subtitle") => "System application management: configure, scan, detect, launch, and search",
        (Language::English, "functions") => "Functions",
        (Language::English, "settings") => "Settings",
        (Language::English, "dashboard") => "Dashboard",
        (Language::English, "config") => "Config",
        (Language::English, "scan_apps") => "Scan Apps",
        (Language::English, "app_manager") => "App Manager",
        (Language::English, "log") => "Log:",
        (Language::English, "clear") => "Clear",
        (Language::English, "loaded") => "Loaded",
        (Language::English, "scanned") => "Scanned",
        (Language::English, "apps") => "apps",
        (Language::English, "completed") => "Completed",
        (Language::English, "dashboard_description") => {
            "Manage system applications: integrate, scan, detect, and launch applications."
        }
        (Language::English, "choose_tab") => "Select a tab on the left to continue.",
        (Language::English, "load_config") => "Load Config",
        (Language::English, "loading") => "Loading...",
        (Language::English, "scan") => "Scan",
        (Language::English, "scanning") => "Scanning...",
        (Language::English, "path") => "Path (file/directory):",
        (Language::English, "detect") => "Detect",
        (Language::English, "known_apps") => "Known applications",
        (Language::English, "no_data") => "No data. Load the config or scan applications first.",
        (Language::English, "stop") => "Stop",
        (Language::English, "start") => "Start",
        (Language::English, "enter_path") => "Please enter a path",
        (Language::English, "detecting") => "Detecting...",
        (Language::English, "starting") => "Starting",
        (Language::English, "stopping") => "Stopping",
        (Language::English, "started") => "Started",
        (Language::English, "stopped") => "Stopped",
        (Language::English, "keyword") => "Keyword:",
        (Language::English, "search") => "Search",
        (Language::English, "enter_keyword") => "Please enter a keyword",
        (Language::English, "searching") => "Searching...",
        (Language::English, "language") => "Language",
        (Language::English, "theme") => "Theme",
        (Language::English, "system") => "System",
        (Language::English, "light") => "Light",
        (Language::English, "dark") => "Dark",
        (Language::English, "font") => "Font",
        (Language::English, "default_font") => "Default",
        (Language::English, "available_fonts") => "Available fonts",
        (Language::English, "saved") => "Preferences saved",
        (Language::English, "save_failed") => "Could not save preferences",
        (_, "subtitle") => "Quản lý ứng dụng hệ thống: cấu hình, quét, phát hiện, khởi chạy, tìm kiếm",
        (_, "functions") => "Chức năng",
        (_, "settings") => "Cài đặt",
        (_, "dashboard") => "Tổng quan",
        (_, "config") => "Cấu hình",
        (_, "scan_apps") => "Quét ứng dụng",
        (_, "app_manager") => "Quản lý ứng dụng",
        (_, "log") => "Nhật ký:",
        (_, "clear") => "Xóa",
        (_, "loaded") => "Đã tải",
        (_, "scanned") => "Đã quét",
        (_, "apps") => "ứng dụng",
        (_, "completed") => "Hoàn tất",
        (_, "dashboard_description") => {
            "Công cụ quản lý ứng dụng hệ thống: tích hợp, quét, phát hiện và khởi chạy ứng dụng."
        }
        (_, "choose_tab") => "Chọn tab bên trái để thao tác.",
        (_, "load_config") => "Tải cấu hình",
        (_, "loading") => "Đang tải...",
        (_, "scan") => "Quét",
        (_, "scanning") => "Đang quét...",
        (_, "path") => "Đường dẫn (file/thư mục):",
        (_, "detect") => "Phát hiện",
        (_, "known_apps") => "Ứng dụng đã biết",
        (_, "no_data") => "Chưa có dữ liệu. Hãy tải cấu hình hoặc quét ứng dụng trước.",
        (_, "stop") => "Dừng",
        (_, "start") => "Khởi chạy",
        (_, "enter_path") => "Vui lòng nhập đường dẫn",
        (_, "detecting") => "Đang phát hiện...",
        (_, "starting") => "Đang khởi động",
        (_, "stopping") => "Đang dừng",
        (_, "started") => "Đã khởi động",
        (_, "stopped") => "Đã dừng",
        (_, "keyword") => "Từ khóa:",
        (_, "search") => "Tìm kiếm",
        (_, "enter_keyword") => "Vui lòng nhập từ khóa",
        (_, "searching") => "Đang tìm kiếm...",
        (_, "language") => "Ngôn ngữ",
        (_, "theme") => "Giao diện",
        (_, "system") => "Hệ thống",
        (_, "light") => "Sáng",
        (_, "dark") => "Tối",
        (_, "font") => "Phông chữ",
        (_, "default_font") => "Mặc định",
        (_, "available_fonts") => "Phông chữ khả dụng",
        (_, "saved") => "Đã lưu tùy chọn",
        (_, "save_failed") => "Không thể lưu tùy chọn",
        _ => key,
    }
}

// ---------------------------------------------------------------------------
// Tab: Dashboard
// ---------------------------------------------------------------------------

impl UniverseManagerApp {
    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("dashboard"));
        ui.separator();
        ui.add_space(4.0);

        ui.label(self.text("dashboard_description"));
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
        ui.label(self.text("choose_tab"));
    }

    // -----------------------------------------------------------------------
    // Tab: Config
    // -----------------------------------------------------------------------

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("config"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button(self.text("load_config")).clicked() {
                self.load_config(ui.ctx().clone());
            }
            ui.label(&self.config_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.monospace(&self.config_text);
                });
        });
    }

    fn load_config(&mut self, ctx: egui::Context) {
        self.config_status = self.text("loading").into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let cfg = universe_manager::config::Config::load();
            let text = serde_json::to_string_pretty(&cfg).unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::ConfigLoaded(text, cfg));
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Scan Apps
    // -----------------------------------------------------------------------

    fn ui_scan_apps(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("scan_apps"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button(self.text("scan")).clicked() {
                self.scan_apps(ui.ctx().clone());
            }
            ui.label(&self.scan_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.monospace(&self.scan_text);
                });
        });
    }

    fn scan_apps(&mut self, ctx: egui::Context) {
        self.scan_status = self.text("scanning").into();
        self.scan_text.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let apps = universe_manager::scanner::scan_all_system_apps();
            let text = serde_json::to_string_pretty(&apps).unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::AppsScanned(text, apps));
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: App Manager
    // -----------------------------------------------------------------------

    fn ui_app_manager(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.text("app_manager"));
        ui.separator();
        ui.add_space(4.0);

        // Detect App section
        ui.horizontal(|ui| {
            ui.label(self.text("path"));
            ui.add(egui::TextEdit::singleline(&mut self.detect_path).desired_width(320.0));
            if ui.button(self.text("detect")).clicked() {
                self.detect_app(ui.ctx().clone());
            }
            ui.label(&self.detect_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(ui.visuals().extreme_bg_color)
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
            ui.label(format!("{} ({}):", self.text("known_apps"), self.apps_list.len()));
            ui.label(&self.app_action_status);
        });

        if self.apps_list.is_empty() {
            ui.label(self.text("no_data"));
        } else {
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for i in 0..self.apps_list.len() {
                    let app = self.apps_list[i].clone();
                    ui.horizontal(|ui| {
                        ui.label(&app.name);
                        ui.separator();
                        ui.label(&app.id);
                        ui.separator();
                        ui.monospace(&app.exec_path);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(self.text("stop")).clicked() {
                                self.stop_app(ui.ctx().clone(), app.clone());
                            }
                            if ui.button(self.text("start")).clicked() {
                                self.start_app(ui.ctx().clone(), app.clone());
                            }
                        });
                    });
                }
            });
        }
    }

    fn detect_app(&mut self, ctx: egui::Context) {
        let path = self.detect_path.trim().to_string();
        if path.is_empty() {
            self.detect_status = self.text("enter_path").into();
            return;
        }
        self.detect_status = self.text("detecting").into();
        self.detect_text.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match universe_manager::detector::detect(&path) {
                Ok(result) => {
                    let text =
                        serde_json::to_string_pretty(&result).unwrap_or_else(|e| format!("Serialize error: {}", e));
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
        self.app_action_status = format!("{} {}...", self.text("starting"), name);
        self.log.push(format!("▶️ Start: {}", name));
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match universe_manager::manager::start_app(&app) {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::AppStarted(name));
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
        self.app_action_status = format!("{} {}...", self.text("stopping"), name);
        self.log.push(format!("⏹ Stop: {}", name));
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match universe_manager::manager::stop_app(&app) {
                Ok(()) => {
                    let _ = tx.send(AsyncResult::AppStopped(name));
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
        ui.heading(self.text("search"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(self.text("keyword"));
            ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(280.0));
            if ui.button(self.text("search")).clicked() {
                self.search_apps(ui.ctx().clone());
            }
            ui.label(&self.search_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.monospace(&self.search_text);
                });
        });
    }

    fn search_apps(&mut self, ctx: egui::Context) {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            self.search_status = self.text("enter_keyword").into();
            return;
        }
        self.search_status = self.text("searching").into();
        self.search_text.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let results = universe_manager::installer::search_apps(&query);
            let text = serde_json::to_string_pretty(&results).unwrap_or_else(|e| format!("Serialize error: {}", e));
            let _ = tx.send(AsyncResult::SearchCompleted(text));
            ctx.request_repaint();
        });
    }
}
