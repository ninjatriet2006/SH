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
    settings_status: String,

    // scan images panel
    scan_dir: String,
    scan_files_list: Vec<PathBuf>,
    scan_status: String,
    scan_count: usize,

    // environment panel
    ffmpeg_status: String,
    ffmpeg_result: String,

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
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        ImgSpltApp {
            tx,
            rx,
            selected_tab: Tab::default(),
            settings_text: String::new(),
            settings_status: String::new(),
            scan_dir: String::new(),
            scan_files_list: Vec::new(),
            scan_status: String::new(),
            scan_count: 0,
            ffmpeg_status: String::new(),
            ffmpeg_result: String::new(),
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
            .with_title("Image Splitter — GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "img_splt_gui",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(ImgSpltApp::new()))
        }),
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
                ui.label("Chia ảnh, upscale và phân phối file bằng ffmpeg");
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
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "⚙️ Settings");
                ui.selectable_value(&mut self.selected_tab, Tab::ScanImages, "📂 Scan Images");
                ui.selectable_value(&mut self.selected_tab, Tab::Environment, "🔍 Environment");
            });

        // ── Central panel ──────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.ui_dashboard(ui),
                Tab::Settings => self.ui_settings(ui),
                Tab::ScanImages => self.ui_scan_images(ui),
                Tab::Environment => self.ui_environment(ui),
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

impl ImgSpltApp {
    fn drain_async_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                AsyncResult::SettingsLoaded(text) => {
                    self.settings_text = text;
                    self.settings_status = "✓ Đã tải".into();
                    self.log.push("✓ Settings loaded".into());
                }
                AsyncResult::ScanFinished { files, dir } => {
                    self.scan_files_list = files;
                    self.scan_count = self.scan_files_list.len();
                    self.scan_status =
                        format!("✓ Tìm thấy {} ảnh trong \"{}\"", self.scan_count, dir);
                    self.log.push(format!(
                        "✓ Scanned {} images in {}",
                        self.scan_count, dir
                    ));
                }
                AsyncResult::FfmpegChecked(text) => {
                    self.ffmpeg_result = text;
                    self.ffmpeg_status = "✓ Hoàn tất".into();
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

        ui.label("Công cụ chia ảnh thành nhiều phần, upscale và phân phối file.");
        ui.label("Sử dụng ffmpeg để xử lý ảnh và chia thư mục tự động.");
        ui.add_space(12.0);

        egui::Grid::new("dashboard_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                ui.label("⚙️ Settings:");
                ui.label(&self.settings_status);
                ui.end_row();
                ui.label("📂 Scan Images:");
                ui.label(&self.scan_status);
                ui.end_row();
                ui.label("🔍 FFmpeg:");
                ui.label(&self.ffmpeg_status);
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label("👉 Chọn tab bên trái để thao tác.");
    }

    // -----------------------------------------------------------------------
    // Tab: Settings
    // -----------------------------------------------------------------------

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Settings");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("📂 Load Settings").clicked() {
                self.load_settings(ui.ctx().clone());
            }
            ui.label(&self.settings_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.settings_text);
                    });
            });
    }

    fn load_settings(&mut self, ctx: egui::Context) {
        self.settings_status = "⏳ Đang tải...".into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // load_or_create_settings() trả Settings trực tiếp và có thể panic
            // khi không thể tạo/đọc settings.yaml (dùng expect bên trong crate).
            let result = std::panic::catch_unwind(img_splt::config::load_or_create_settings);
            match result {
                Ok(settings) => {
                    let text = serde_json::to_string_pretty(&settings)
                        .unwrap_or_else(|e| format!("Serialize error: {}", e));
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
        ui.heading("📂 Scan Images");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Đường dẫn thư mục:");
            ui.text_edit_singleline(&mut self.scan_dir);
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("🔍 Set Dir & Scan").clicked() {
                self.scan_files(ui.ctx().clone());
            }
            ui.label(&self.scan_status);
        });

        if self.scan_count > 0 {
            ui.add_space(4.0);
            ui.label(format!("Tìm thấy {} ảnh", self.scan_count));
        }

        ui.add_space(8.0);
        egui::Frame::default()
            .fill(egui::Color32::from_rgb(20, 22, 30))
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
            self.scan_status = "⚠️ Nhập đường dẫn thư mục".into();
            return;
        }
        self.scan_status = "⏳ Đang quét...".into();
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
                    let _ = tx.send(AsyncResult::Error(format!(
                        "set_current_dir({}): {}",
                        dir, e
                    )));
                }
            }
            ctx.request_repaint();
        });
    }

    // -----------------------------------------------------------------------
    // Tab: Environment
    // -----------------------------------------------------------------------

    fn ui_environment(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Environment");
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("🔍 Check FFmpeg").clicked() {
                self.check_ffmpeg(ui.ctx().clone());
            }
            ui.label(&self.ffmpeg_status);
        });

        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgb(20, 22, 30))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.monospace(&self.ffmpeg_result);
                    });
            });
    }

    fn check_ffmpeg(&mut self, ctx: egui::Context) {
        self.ffmpeg_status = "⏳ Đang kiểm tra...".into();
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
            match std::process::Command::new("ffprobe")
                .arg("-version")
                .output()
            {
                Ok(out) if out.status.success() => text.push_str("\nffprobe: OK\n"),
                _ => text.push_str("\nffprobe: KHÔNG tìm thấy\n"),
            }

            let _ = tx.send(AsyncResult::FfmpegChecked(text));
            ctx.request_repaint();
        });
    }
}
