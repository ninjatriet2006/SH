use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct ImgSpltApp {
    // navigation
    selected_tab: Tab,

    // settings panel
    settings_text: String,
    settings_status: SettingsStatus,

    // scan images panel
    scan_dir: String,
    scan_files_list: Vec<PathBuf>,
    scan_status: ScanStatus,
    scan_count: usize,

    // environment panel
    ffmpeg_status: FfmpegStatus,
    ffmpeg_result: String,

    // GUI preferences
    language: Language,
    theme: Theme,
    selected_font: String,
    available_fonts: Vec<FontOption>,

    // output log
    log: Vec<String>,

    // persistent channel: tx cloned for each task, rx drained each frame
    tx: mpsc::Sender<AsyncResult>,
    rx: mpsc::Receiver<AsyncResult>,
}

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Settings,
    ScanImages,
    Environment,
}

#[derive(Clone, Copy, PartialEq)]
enum Language {
    English,
    Vietnamese,
}

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    Dark,
    Light,
}

#[derive(Clone)]
struct FontOption {
    name: &'static str,
    path: String,
}

#[derive(Default)]
enum SettingsStatus {
    #[default]
    Idle,
    Loading,
    Loaded,
}

#[derive(Default)]
enum ScanStatus {
    #[default]
    Idle,
    MissingDir,
    Loading,
    Finished(String),
}

#[derive(Default)]
enum FfmpegStatus {
    #[default]
    Idle,
    Checking,
    Done,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Dashboard
    }
}

enum AsyncResult {
    SettingsLoaded(String),
    ScanFinished { files: Vec<PathBuf>, dir: String },
    FfmpegChecked(String),
    Error(String),
}

impl ImgSpltApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        let storage = cc.storage;
        let language = match storage.and_then(|s| s.get_string("language")).as_deref() {
            Some("vi") => Language::Vietnamese,
            _ => Language::English,
        };
        let theme = match storage.and_then(|s| s.get_string("theme")).as_deref() {
            Some("light") => Theme::Light,
            _ => Theme::Dark,
        };
        let available_fonts = available_fonts();
        let mut selected_font = storage.and_then(|s| s.get_string("font")).unwrap_or_default();
        if !selected_font.is_empty() && !available_fonts.iter().any(|font| font.path == selected_font) {
            selected_font.clear();
        }

        apply_theme(&cc.egui_ctx, theme);
        setup_fonts(
            &cc.egui_ctx,
            (!selected_font.is_empty()).then_some(selected_font.as_str()),
        );

        ImgSpltApp {
            tx,
            rx,
            selected_tab: Tab::default(),
            settings_text: String::new(),
            settings_status: SettingsStatus::default(),
            scan_dir: String::new(),
            scan_files_list: Vec::new(),
            scan_status: ScanStatus::default(),
            scan_count: 0,
            ffmpeg_status: FfmpegStatus::default(),
            ffmpeg_result: String::new(),
            language,
            theme,
            selected_font,
            available_fonts,
            log: Vec::new(),
        }
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

fn setup_fonts(ctx: &egui::Context, selected_font: Option<&str>) {
    let mut fonts = egui::FontDefinitions::default();

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

    if let Some(path) = selected_font {
        if let Ok(bytes) = std::fs::read(path) {
            let name = "selected_font".to_owned();
            fonts
                .font_data
                .insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, name);
        }
    }

    ctx.set_fonts(fonts);
}

fn available_fonts() -> Vec<FontOption> {
    const CANDIDATES: &[(&str, &str)] = &[
        ("Noto Sans", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"),
        ("DejaVu Sans", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        (
            "Liberation Sans",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ),
        ("Ubuntu", "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf"),
        ("Arial", "/System/Library/Fonts/Supplemental/Arial.ttf"),
        ("Helvetica", "/System/Library/Fonts/Helvetica.ttc"),
        ("Segoe UI", "C:\\Windows\\Fonts\\segoeui.ttf"),
        ("Arial", "C:\\Windows\\Fonts\\arial.ttf"),
    ];

    let mut fonts = vec![FontOption {
        name: "egui Default",
        path: String::new(),
    }];
    fonts.extend(
        CANDIDATES
            .iter()
            .filter(|(_, path)| std::path::Path::new(path).is_file())
            .map(|(name, path)| FontOption {
                name,
                path: (*path).to_owned(),
            }),
    );
    fonts
}

fn tr(language: Language, english: &'static str, vietnamese: &'static str) -> &'static str {
    match language {
        Language::English => english,
        Language::Vietnamese => vietnamese,
    }
}

fn apply_theme(ctx: &egui::Context, theme: Theme) {
    match theme {
        Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
        Theme::Light => ctx.set_visuals(egui::Visuals::light()),
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_title("Image Splitter — GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "img_splt_gui",
        options,
        Box::new(|cc| Ok(Box::new(ImgSpltApp::new(cc)))),
    )
}

// ---------------------------------------------------------------------------
// egui app
// ---------------------------------------------------------------------------

impl eframe::App for ImgSpltApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Drain async results ───────────────────────────────────────────
        self.drain_async_results();

        // ── Top bar ───────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Image Splitter");
                ui.separator();
                ui.label(tr(
                    self.language,
                    "Split, upscale, and distribute images with ffmpeg",
                    "Chia ảnh, upscale và phân phối file bằng ffmpeg",
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
                ui.vertical_centered(|ui| {
                    ui.heading(tr(self.language, "Features", "Chức năng"));
                });
                ui.separator();
                ui.add_space(4.0);

                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, "🏠 Dashboard");
                ui.selectable_value(
                    &mut self.selected_tab,
                    Tab::Settings,
                    tr(self.language, "⚙ Settings", "⚙ Cài đặt"),
                );
                ui.selectable_value(
                    &mut self.selected_tab,
                    Tab::ScanImages,
                    tr(self.language, "📂 Scan Images", "📂 Quét ảnh"),
                );
                ui.selectable_value(
                    &mut self.selected_tab,
                    Tab::Environment,
                    tr(self.language, "🔍 Environment", "🔍 Môi trường"),
                );
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::Dashboard => self.ui_dashboard(ui),
            Tab::Settings => self.ui_settings(ui),
            Tab::ScanImages => self.ui_scan_images(ui),
            Tab::Environment => self.ui_environment(ui),
        });

        // ── Bottom log ─────────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("log_panel")
            .resizable(true)
            .default_height(80.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr(self.language, "Log:", "Nhật ký:"));
                    if ui.button(tr(self.language, "Clear", "Xóa hiển thị")).clicked() {
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
        storage.set_string(
            "language",
            match self.language {
                Language::English => "en",
                Language::Vietnamese => "vi",
            }
            .to_owned(),
        );
        storage.set_string(
            "theme",
            match self.theme {
                Theme::Dark => "dark",
                Theme::Light => "light",
            }
            .to_owned(),
        );
        storage.set_string("font", self.selected_font.clone());
    }
}

// ---------------------------------------------------------------------------
// Async helpers
// ---------------------------------------------------------------------------

impl ImgSpltApp {
    fn scan_status_text(&self) -> String {
        match &self.scan_status {
            ScanStatus::Idle => String::new(),
            ScanStatus::MissingDir => tr(self.language, "Enter a directory path", "Nhập đường dẫn thư mục").to_owned(),
            ScanStatus::Loading => tr(self.language, "Scanning...", "Đang quét...").to_owned(),
            ScanStatus::Finished(dir) => format!(
                "{} {} {} \"{}\"",
                tr(self.language, "Found", "Tìm thấy"),
                self.scan_count,
                tr(self.language, "images in", "ảnh trong"),
                dir
            ),
        }
    }

    fn drain_async_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                AsyncResult::SettingsLoaded(text) => {
                    self.settings_text = text;
                    self.settings_status = SettingsStatus::Loaded;
                    self.log.push("✓ Settings loaded".into());
                }
                AsyncResult::ScanFinished { files, dir } => {
                    self.scan_files_list = files;
                    self.scan_count = self.scan_files_list.len();
                    self.scan_status = ScanStatus::Finished(dir.clone());
                    self.log
                        .push(format!("✓ Scanned {} images in {}", self.scan_count, dir));
                }
                AsyncResult::FfmpegChecked(text) => {
                    self.ffmpeg_result = text;
                    self.ffmpeg_status = FfmpegStatus::Done;
                    self.log.push("✓ FFmpeg check done".into());
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

impl ImgSpltApp {
    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.separator();
        ui.add_space(4.0);

        ui.label(tr(
            self.language,
            "Split images into parts, upscale them, and distribute files.",
            "Công cụ chia ảnh thành nhiều phần, upscale và phân phối file.",
        ));
        ui.label(tr(
            self.language,
            "Uses ffmpeg for image processing and automatic folder organization.",
            "Sử dụng ffmpeg để xử lý ảnh và chia thư mục tự động.",
        ));
        ui.add_space(12.0);

        egui::Grid::new("dashboard_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label(tr(self.language, "⚙ Settings:", "⚙ Cài đặt:"));
                ui.label(match self.settings_status {
                    SettingsStatus::Idle => "",
                    SettingsStatus::Loading => tr(self.language, "Loading...", "Đang tải..."),
                    SettingsStatus::Loaded => tr(self.language, "Loaded", "Đã tải"),
                });
                ui.end_row();
                ui.label(tr(self.language, "📂 Scan Images:", "📂 Quét ảnh:"));
                ui.label(self.scan_status_text());
                ui.end_row();
                ui.label("🔍 FFmpeg:");
                ui.label(match self.ffmpeg_status {
                    FfmpegStatus::Idle => "",
                    FfmpegStatus::Checking => tr(self.language, "Checking...", "Đang kiểm tra..."),
                    FfmpegStatus::Done => tr(self.language, "Done", "Hoàn tất"),
                });
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(tr(
            self.language,
            "Select a tab on the left to begin.",
            "Chọn tab bên trái để thao tác.",
        ));
    }

    // -----------------------------------------------------------------------
    // Tab: Settings
    // -----------------------------------------------------------------------

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(tr(self.language, "⚙ Settings", "⚙ Cài đặt"));
        ui.separator();
        ui.add_space(4.0);

        egui::Grid::new("gui_preferences")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(tr(self.language, "Language", "Ngôn ngữ"));
                egui::ComboBox::from_id_salt("language")
                    .selected_text(match self.language {
                        Language::English => "English",
                        Language::Vietnamese => "Tiếng Việt",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.language, Language::English, "English");
                        ui.selectable_value(&mut self.language, Language::Vietnamese, "Tiếng Việt");
                    });
                ui.end_row();

                ui.label(tr(self.language, "Theme", "Giao diện"));
                let old_theme = self.theme;
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(match self.theme {
                        Theme::Dark => tr(self.language, "Dark", "Tối"),
                        Theme::Light => tr(self.language, "Light", "Sáng"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.theme, Theme::Dark, tr(self.language, "Dark", "Tối"));
                        ui.selectable_value(&mut self.theme, Theme::Light, tr(self.language, "Light", "Sáng"));
                    });
                if old_theme != self.theme {
                    apply_theme(ui.ctx(), self.theme);
                }
                ui.end_row();

                ui.label(tr(self.language, "Font", "Phông chữ"));
                let old_font = self.selected_font.clone();
                let selected_name = self
                    .available_fonts
                    .iter()
                    .find(|font| font.path == self.selected_font)
                    .map_or("egui Default", |font| font.name);
                egui::ComboBox::from_id_salt("font")
                    .selected_text(selected_name)
                    .show_ui(ui, |ui| {
                        for font in &self.available_fonts {
                            ui.selectable_value(&mut self.selected_font, font.path.clone(), font.name);
                        }
                    });
                if old_font != self.selected_font {
                    setup_fonts(
                        ui.ctx(),
                        (!self.selected_font.is_empty()).then_some(self.selected_font.as_str()),
                    );
                }
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.label(tr(
            self.language,
            "Image splitter configuration",
            "Cấu hình bộ chia ảnh",
        ));

        ui.horizontal(|ui| {
            if ui
                .button(tr(self.language, "📂 Load Settings", "📂 Tải cài đặt"))
                .clicked()
            {
                self.load_settings(ui.ctx().clone());
            }
            ui.label(match self.settings_status {
                SettingsStatus::Idle => "",
                SettingsStatus::Loading => tr(self.language, "Loading...", "Đang tải..."),
                SettingsStatus::Loaded => tr(self.language, "Loaded", "Đã tải"),
            });
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.monospace(&self.settings_text);
                });
        });
    }

    fn load_settings(&mut self, ctx: egui::Context) {
        self.settings_status = SettingsStatus::Loading;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // load_or_create_settings() trả Settings trực tiếp và có thể panic
            // khi không thể tạo/đọc settings.yaml (dùng expect bên trong crate).
            let result = std::panic::catch_unwind(img_splt::config::load_or_create_settings);
            match result {
                Ok(settings) => {
                    let text =
                        serde_json::to_string_pretty(&settings).unwrap_or_else(|e| format!("Serialize error: {}", e));
                    let _ = tx.send(AsyncResult::SettingsLoaded(text));
                }
                Err(_) => {
                    let _ = tx.send(AsyncResult::Error(
                        "Không thể đọc/tạo settings.yaml trong thư mục hiện tại".into(),
                    ));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Scan Images
    // -----------------------------------------------------------------------

    fn ui_scan_images(&mut self, ui: &mut egui::Ui) {
        ui.heading(tr(self.language, "📂 Scan Images", "📂 Quét ảnh"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(tr(self.language, "Directory:", "Đường dẫn thư mục:"));
            ui.text_edit_singleline(&mut self.scan_dir);
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button(tr(self.language, "🔍 Set Directory & Scan", "🔍 Chọn thư mục và quét"))
                .clicked()
            {
                self.scan_files(ui.ctx().clone());
            }
            ui.label(self.scan_status_text());
        });

        if self.scan_count > 0 {
            ui.add_space(4.0);
            ui.label(format!(
                "{} {}",
                tr(self.language, "Images found:", "Số ảnh tìm thấy:"),
                self.scan_count
            ));
        }

        ui.add_space(8.0);
        egui::Frame::default()
            .fill(ui.visuals().extreme_bg_color)
            .corner_radius(4.0)
            .show(ui, |ui| {
                let total = self.scan_files_list.len();
                let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .max_height(300.0)
                    .show_rows(ui, row_height, total, |ui, range| {
                        for i in range {
                            ui.monospace(self.scan_files_list[i].display().to_string());
                        }
                    });
            });
    }

    fn scan_files(&mut self, ctx: egui::Context) {
        let dir = self.scan_dir.trim().to_string();
        if dir.is_empty() {
            self.scan_status = ScanStatus::MissingDir;
            return;
        }
        self.scan_status = ScanStatus::Loading;
        self.scan_count = 0;
        self.scan_files_list.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // scan_files() của crate quét theo CWD nên phải set_current_dir trước.
            match std::env::set_current_dir(&dir) {
                Ok(_) => {
                    let all_files = img_splt::scanner::scan_files();
                    let images: Vec<PathBuf> = all_files
                        .into_iter()
                        .filter(|p| img_splt::scanner::is_image_extension(p))
                        .collect();
                    let _ = tx.send(AsyncResult::ScanFinished { files: images, dir });
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(format!("set_current_dir({}): {}", dir, e)));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Environment
    // -----------------------------------------------------------------------

    fn ui_environment(&mut self, ui: &mut egui::Ui) {
        ui.heading(tr(self.language, "🔍 Environment", "🔍 Môi trường"));
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui
                .button(tr(self.language, "🔍 Check FFmpeg", "🔍 Kiểm tra FFmpeg"))
                .clicked()
            {
                self.check_ffmpeg(ui.ctx().clone());
            }
            ui.label(match self.ffmpeg_status {
                FfmpegStatus::Idle => "",
                FfmpegStatus::Checking => tr(self.language, "Checking...", "Đang kiểm tra..."),
                FfmpegStatus::Done => tr(self.language, "Done", "Hoàn tất"),
            });
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            egui::Frame::default()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.monospace(&self.ffmpeg_result);
                });
        });
    }

    fn check_ffmpeg(&mut self, ctx: egui::Context) {
        self.ffmpeg_status = FfmpegStatus::Checking;
        self.ffmpeg_result.clear();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // Gọi hàm kiểm tra của crate (in ra stdout của terminal).
            img_splt::env_check::check_ffmpeg();

            // Thu thập riêng stdout của ffmpeg -version để hiển thị trong log GUI.
            let mut text = String::new();
            match std::process::Command::new("ffmpeg").arg("-version").output() {
                Ok(out) if out.status.success() => {
                    text.push_str("ffmpeg: OK\n");
                    text.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                Ok(out) => {
                    text.push_str(&format!("ffmpeg: exit code {:?}\n", out.status.code()));
                    text.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                Err(e) => {
                    text.push_str(&format!("ffmpeg: không tìm thấy ({})\n", e));
                }
            }
            match std::process::Command::new("ffprobe").arg("-version").output() {
                Ok(out) if out.status.success() => text.push_str("\nffprobe: OK\n"),
                _ => text.push_str("\nffprobe: KHÔNG tìm thấy\n"),
            }

            let _ = tx.send(AsyncResult::FfmpegChecked(text));
            ctx.request_repaint();
        });
    }
}
