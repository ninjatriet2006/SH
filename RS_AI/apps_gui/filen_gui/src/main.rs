mod operations;
mod transfer;

use crate::operations::{
    FileItem, Operations, StoredAccount, SyncPair, load_stored_accounts, mount_args, s3_args,
    save_stored_accounts, webdav_args,
};
use crate::transfer::{
    ProgressUpdate, TransferError, TransferItem, TransferKind, TransferManager, TransferStatus,
    copy_local, delete_local_path, move_local, run_cli_transfer,
};
use eframe::egui;
use std::sync::mpsc;

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// Chế độ hiển thị của một khung (pane): thư mục cục bộ hoặc Filen Cloud.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaneMode {
    Local,
    Cloud,
}

impl PaneMode {
    fn label(&self) -> &'static str {
        match self {
            PaneMode::Local => "Cục bộ",
            PaneMode::Cloud => "Cloud",
        }
    }

    fn glyph(&self) -> &'static str {
        match self {
            PaneMode::Local => "🖥️",
            PaneMode::Cloud => "☁️",
        }
    }
}

/// Trạng thái một khung explorer (trái/phải).
struct PaneState {
    mode: PaneMode,
    path: String,
    items: Vec<FileItem>,
    filter: String,
    /// Danh sách mục đang chọn (multi-select kiểu Nemo).
    selected: Vec<String>,
    /// Mỏ neo cho Shift+click (tên mục cuối được click đơn), để chọn khoảng.
    anchor: Option<String>,
    status: String,
    /// Lịch sử điều hướng (nút Back) — đường dẫn trước mỗi lần chuyển.
    back: Vec<String>,
    /// Lịch sử điều hướng (nút Forward).
    fwd: Vec<String>,
    /// Độ rộng cột [Kích thước, Loại, Ngày sửa] — kéo thả được ở header.
    col_w: [f32; 3],
}

impl Default for PaneState {
    fn default() -> Self {
        PaneState {
            mode: PaneMode::Local,
            path: resolve_home_dir().unwrap_or_else(|| "/".to_string()),
            items: Vec::new(),
            filter: String::new(),
            selected: Vec::new(),
            anchor: None,
            status: "Sẵn sàng".to_string(),
            back: Vec::new(),
            fwd: Vec::new(),
            col_w: [90.0, 80.0, 120.0],
        }
    }
}

/// Trạng thái tài khoản Filen: danh sách đã lưu + account đang active + statfs.
struct AccountState {
    /// Danh sách tài khoản đã lưu để đăng nhập nhanh.
    stored: Vec<StoredAccount>,
    /// Email tài khoản đang hoạt động (None nếu chưa đăng nhập).
    active: Option<String>,
    /// Dung lượng đã dùng / tổng (từ statfs).
    used: String,
    max: String,
    /// Đang xử lý thao tác tài khoản (login/logout) — khóa nút trùng lặp.
    busy: bool,
}

impl Default for AccountState {
    fn default() -> Self {
        AccountState {
            stored: Vec::new(),
            active: None,
            used: "0 B".to_string(),
            max: "0 B".to_string(),
            busy: false,
        }
    }
}

/// Trạng thái form đăng nhập (modal).
#[derive(Default)]
struct LoginFormState {
    open: bool,
    email: String,
    password: String,
    keep_logged: bool,
    twofa: String,
    pending_twofa: bool,
    error: Option<String>,
}

impl LoginFormState {
    /// Form mới: mặc định giữ phiên đăng nhập (giống TUI mặc định "y").
    fn fresh() -> Self {
        LoginFormState {
            keep_logged: true,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 7: các kiểu phục vụ UI tìm kiếm, thao tác file, recents, sync, servers
// ---------------------------------------------------------------------------

/// Trang chính hiển thị ở central panel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MainView {
    Explorer,
    Recents,
    Sync,
    Servers,
}

/// Loại thao tác file (dùng cho async result + log tiếng Việt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOpKind {
    Mkdir,
    Rename,
    Delete,
    Favorite,
    Unfavorite,
    View,
    CopyLink,
}

impl FileOpKind {
    fn verb(&self) -> &'static str {
        match self {
            FileOpKind::Mkdir => "Tạo thư mục",
            FileOpKind::Rename => "Đổi tên",
            FileOpKind::Delete => "Xóa",
            FileOpKind::Favorite => "Yêu thích",
            FileOpKind::Unfavorite => "Bỏ yêu thích",
            FileOpKind::View => "Xem",
            FileOpKind::CopyLink => "Copy link",
        }
    }
}

/// Modal đang mở (popup nhập liệu / xác nhận / hiển thị kết quả).
enum Modal {
    Mkdir { input: String },
    Rename { old: String, input: String },
    Delete { names: Vec<String>, no_trash: bool },
    View { title: String, content: String },
    Link { url: String },
}

/// Server nào đang được thao tác.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ServerWhich {
    WebDav,
    S3,
    Mount,
}

/// Hành động được thu thập từ context menu (áp dụng sau khi closure kết thúc).
enum PaneItemAction {
    Select(String),
    ToggleSelect(String),
    RangeSelect(String),
    /// Double-click: vào thư mục (is_dir) hoặc xem file.
    Activate { name: String, is_dir: bool },
    Navigate(String),
    Rename(String),
    Delete(String),
    Favorite(String),
    Unfavorite(String),
    View(String),
    CopyLink(String),
    CopyPath(String),
}

/// Trạng thái server WebDAV trong GUI (dùng std::process::Child để giữ tiến trình).
struct WebDavGui {
    user: String,
    pass: String,
    port: String,
    https: bool,
    running: bool,
    child: Option<std::process::Child>,
    logs: Vec<String>,
}

impl Default for WebDavGui {
    fn default() -> Self {
        WebDavGui {
            user: "admin".to_string(),
            pass: "admin123".to_string(),
            port: "8080".to_string(),
            https: false,
            running: false,
            child: None,
            logs: Vec::new(),
        }
    }
}

/// Trạng thái server S3 trong GUI.
struct S3Gui {
    access_key: String,
    secret_key: String,
    port: String,
    https: bool,
    running: bool,
    child: Option<std::process::Child>,
    logs: Vec<String>,
}

impl Default for S3Gui {
    fn default() -> Self {
        S3Gui {
            access_key: "s3key".to_string(),
            secret_key: "s3secret".to_string(),
            port: "9000".to_string(),
            https: false,
            running: false,
            child: None,
            logs: Vec::new(),
        }
    }
}

/// Trạng thái mount FUSE trong GUI.
struct MountGui {
    mount_point: String,
    note: String,
    running: bool,
    child: Option<std::process::Child>,
    logs: Vec<String>,
}

impl Default for MountGui {
    fn default() -> Self {
        MountGui {
            mount_point: operations::default_mount_point(),
            note: crate::operations::mount_fuse_note(),
            running: false,
            child: None,
            logs: Vec::new(),
        }
    }
}

#[derive(Default)]
struct ServersState {
    webdav: WebDavGui,
    s3: S3Gui,
    mount: MountGui,
}

// ---------------------------------------------------------------------------
// Phase 12: clipboard nội bộ (Ctrl+C/X/V) + drag & drop giữa hai pane
// ---------------------------------------------------------------------------

/// Nội dung clipboard nội bộ: vị trí nguồn + danh sách mục + chế độ cắt/dán.
struct ClipboardContent {
    src_pane: usize,
    src_mode: PaneMode,
    src_path: String,
    names: Vec<String>,
    cut: bool,
}

/// Trạng thái đang kéo một nhóm mục từ pane này sang pane kia.
struct DragSource {
    src_pane: usize,
    names: Vec<String>,
}

struct FilenGuiApp {
    // Hai khung explorer: index 0 = trái, index 1 = phải.
    panes: [PaneState; 2],
    active_pane: usize,
    account: AccountState,
    login: LoginFormState,
    log: Vec<String>,
    initialized: bool,

    // Trình quản lý transfer (upload/download/copy/move), chạy nền qua thread.
    transfer: TransferManager,

    // ── Phase 7 ────────────────────────────────────────────────────────────
    /// Trang chính đang hiển thị ở central panel.
    view: MainView,
    /// Popup đang mở (mkdir/rename/delete/view/link/export-notes/web-drive).
    modal: Option<Modal>,
    /// Danh sách file gần đây (ops recents).
    recents: Vec<FileItem>,
    recents_status: String,
    /// Các cặp đồng bộ đọc từ syncPairs.json.
    sync_pairs: Vec<SyncPair>,
    sync_error: Option<String>,
    /// Index các cặp đang chạy sync (để hiển thị spinner).
    sync_in_flight: Vec<usize>,
    /// Trạng thái server WebDAV/S3/Mount.
    servers: ServersState,

    // ── Phase 12: clipboard + drag & drop ─────────────────────────────────
    /// Clipboard nội bộ (Ctrl+C / Ctrl+X → Ctrl+V).
    clipboard: Option<ClipboardContent>,
    /// Nhóm mục đang được kéo (drag & drop).
    drag: Option<DragSource>,
    /// Pane đang được hover khi kéo (visual highlight).
    drop_target: Option<usize>,
    /// Rect mỗi pane, cập nhật mỗi frame để phát hiện thả chuột.
    pane_rects: [egui::Rect; 2],

    // persistent channel: tx cloned for each task, rx drained each frame
    tx: mpsc::Sender<AsyncResult>,
    rx: mpsc::Receiver<AsyncResult>,
}

enum AsyncResult {
    FilesListed(usize, Vec<FileItem>),
    Error(usize, String),
    // ── Tài khoản ────────────────────────────────────────────────────────────
    WhoAmIFinished(Result<Option<String>, String>),
    StatfsFinished(Result<(String, String), String>),
    LoginFinished {
        email: String,
        password: String,
        keep_logged: bool,
        result: Result<(), String>,
    },
    LogoutFinished {
        email: String,
        result: Result<(), String>,
    },
    // ── Transfer ────────────────────────────────────────────────────────────
    TransferProgress {
        id: usize,
        progress: Option<f32>,
        bytes_done: u64,
        total_bytes: u64,
    },
    TransferFinished {
        id: usize,
        result: Result<(), TransferError>,
    },
    // ── Phase 7: thao tác file ───────────────────────────────────────────────
    FileOpFinished {
        kind: FileOpKind,
        pane: usize,
        name: String,
        result: Result<(), String>,
    },
    FileTextFinished {
        kind: FileOpKind,
        name: String,
        result: Result<String, String>,
    },
    RecentsFinished(Result<Vec<FileItem>, String>),
    SyncPairsFinished(Result<Vec<SyncPair>, String>),
    SyncPairFinished {
        idx: usize,
        result: Result<(), String>,
    },
    ServerStarted {
        which: ServerWhich,
        result: Result<std::process::Child, String>,
    },
}

impl FilenGuiApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        FilenGuiApp {
            tx,
            rx,
            panes: [
                PaneState::default(),
                PaneState {
                    mode: PaneMode::Cloud,
                    path: "/".to_string(),
                    status: "Sẵn sàng".to_string(),
                    ..Default::default()
                },
            ],
            active_pane: 0,
            account: AccountState {
                stored: load_stored_accounts(),
                ..Default::default()
            },
            login: LoginFormState::fresh(),
            log: Vec::new(),
            initialized: false,
            transfer: TransferManager::new(),
            view: MainView::Explorer,
            modal: None,
            recents: Vec::new(),
            recents_status: "Sẵn sàng".to_string(),
            sync_pairs: Vec::new(),
            sync_error: None,
            sync_in_flight: Vec::new(),
            servers: ServersState::default(),
            clipboard: None,
            drag: None,
            drop_target: None,
            pane_rects: [egui::Rect::NOTHING; 2],
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
        if std::path::Path::new(path).exists()
            && let Ok(bytes) = std::fs::read(path)
        {
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
            .with_inner_size([1100.0, 700.0])
            .with_title("Filen File Manager — GUI"),
        ..Default::default()
    };

    eframe::run_native(
        "filen_gui",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(FilenGuiApp::new()))
        }),
    )
}

// ---------------------------------------------------------------------------
// egui app
// ---------------------------------------------------------------------------

impl eframe::App for FilenGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Initial listing (khung trái: thư mục cục bộ) ─────────────────
        if !self.initialized {
            self.initialized = true;
            self.list_local(0, ctx.clone());
            // Kiểm tra phiên đăng nhập active khi khởi động; pane Cloud sẽ load
            // sau khi có kết quả whoami để tránh lỗi "chưa đăng nhập".
            self.whoami_async(ctx.clone());
        }

        // ── Drain async results ───────────────────────────────────────────
        self.drain_async_results(ctx);

        // ── Phase 12: phím tắt copy/cut/paste (Ctrl+C / Ctrl+X / Ctrl+V) ──
        // Không chặn khi đang gõ trong ô TextEdit (wants_keyboard_input).
        if !ctx.wants_keyboard_input() {
            let (cc, cx, cv) = ctx.input(|i| {
                let ctrl = i.modifiers.ctrl;
                (
                    ctrl && i.key_pressed(egui::Key::C),
                    ctrl && i.key_pressed(egui::Key::X),
                    ctrl && i.key_pressed(egui::Key::V),
                )
            });
            if cc {
                self.copy_selection(false);
            }
            if cx {
                self.copy_selection(true);
            }
            if cv {
                self.paste_clipboard(ctx);
            }
        }

        // ── Panels: bottom → sidebar → central ─────────────────────────
        self.ui_bottom(ctx);
        self.ui_transfers(ctx);
        self.ui_sidebar(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view {
                MainView::Explorer => self.ui_panes(ui),
                MainView::Recents => self.ui_recents(ui),
                MainView::Sync => self.ui_sync(ui),
                MainView::Servers => self.ui_servers(ui),
            }
        });

        // ── Phase 12: theo dõi drag & drop (trước modal để không nuốt sự kiện) ──
        self.handle_drag_drop(ctx);

        // ── Modal đăng nhập + modal thao tác file ─────────────────────────
        self.ui_login_window(ctx);
        self.ui_modal(ctx);
    }
}

// ---------------------------------------------------------------------------
// Transfer: khởi động copy/move giữa hai khung (dùng bởi clipboard Ctrl+C/X/V
// và drag & drop — nút toolbar cũ đã bỏ, thao tác qua kéo thả/phím tắt).
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    /// Chuyển `names` từ pane `src` sang pane `dst`: phân loại theo cặp
    /// nguồn/đích rồi enqueue vào TransferManager (async).
    ///   Local→Local        : fs copy/move
    ///   Local→Cloud        : upload (Move thì xoá nguồn local sau khi xong)
    ///   Cloud→Local        : download (Move thì xoá nguồn cloud sau khi xong)
    ///   Cloud→Cloud        : Operations::cp/mv (cùng account đang active)
    fn start_transfer_between(
        &mut self,
        src: usize,
        dst: usize,
        names: Vec<String>,
        kind: TransferKind,
        ctx: &egui::Context,
    ) {
        let src_local = self.panes[src].mode == PaneMode::Local;
        let dst_local = self.panes[dst].mode == PaneMode::Local;
        let src_path = self.panes[src].path.clone();
        let dst_path = self.panes[dst].path.clone();

        // Cần tài khoản Cloud khi ít nhất một đầu là Cloud.
        if (!src_local || !dst_local) && self.account.active.is_none() {
            self.log.push(
                "⚠️ Chưa đăng nhập Cloud — không thể chuyển qua khung Cloud.".to_string(),
            );
            return;
        }

        for name in &names {
            let src_full = join_path(&src_path, name);
            let dst_full = join_path(&dst_path, name);
            let (item_kind, t_src, t_dst, cleanup_src) = if src_local && dst_local {
                (kind, src_full, dst_full, false)
            } else if src_local && !dst_local {
                (
                    TransferKind::Upload,
                    src_full,
                    dst_path.clone(),
                    kind == TransferKind::Move,
                )
            } else if !src_local && dst_local {
                (
                    TransferKind::Download,
                    src_full,
                    dst_full,
                    kind == TransferKind::Move,
                )
            } else {
                // Cloud→Cloud: app chỉ có một account active nên luôn cùng account;
                // nếu sau này mở rộng nhiều account thì cần kiểm tra cross-account.
                (kind, src_full, dst_full, false)
            };

            self.transfer.enqueue(
                item_kind,
                name.clone(),
                t_src,
                t_dst,
                src_local,
                dst_local,
                cleanup_src,
                src,
                dst,
            );
            self.log.push(format!(
                "⏳ Xếp hàng: {} “{name}” từ khung {} sang khung {}",
                item_kind.label(),
                pane_side(src),
                pane_side(dst),
            ));
        }
        self.start_pending_transfers(ctx);
    }

    /// Khởi động các transfer đang chờ cho đến khi đủ `max_concurrent`.
    fn start_pending_transfers(&mut self, ctx: &egui::Context) {
        while self.transfer.running_count() < self.transfer.max_concurrent {
            let Some(idx) = self.transfer.next_queued_idx() else {
                break;
            };
            let item = self.transfer.items[idx].clone();
            self.transfer.items[idx].status = TransferStatus::Running;
            self.spawn_transfer_thread(item, ctx.clone());
        }
    }

    /// Chạy một transfer trong thread riêng (không block UI): CLI cho
    /// upload/download, Operations::cp/mv cho Cloud→Cloud, fs cho Local→Local.
    fn spawn_transfer_thread(&mut self, item: TransferItem, ctx: egui::Context) {
        let tx = self.tx.clone();
        let account = self.account.active.clone();
        let timeout_secs = self.transfer.timeout_secs;
        std::thread::spawn(move || {
            let id = item.id;
            let kind = item.kind;
            let src = item.src.clone();
            let dst = item.dst.clone();
            let src_local = item.src_local;
            let dst_local = item.dst_local;
            let cancelled = item.cancelled.clone();
            let cleanup_src = item.cleanup_src;

            let tx_progress = tx.clone();
            let on_update = move |upd: ProgressUpdate| {
                let _ = tx_progress.send(AsyncResult::TransferProgress {
                    id,
                    progress: upd.progress,
                    bytes_done: upd.bytes_done,
                    total_bytes: upd.total_bytes,
                });
            };

            let mut result = match kind {
                TransferKind::Upload | TransferKind::Download => tokio::runtime::Runtime::new()
                    .map_err(|e| TransferError::Spawn(e.to_string()))
                    .and_then(|rt| {
                        rt.block_on(run_cli_transfer(
                            kind, &src, &dst, timeout_secs, cancelled, on_update,
                        ))
                    }),
                TransferKind::Copy | TransferKind::Move => {
                    if !src_local && !dst_local {
                        // Cloud → Cloud: cp/mv qua CLI (cùng account active)
                        tokio::runtime::Runtime::new()
                            .map_err(|e| TransferError::Spawn(e.to_string()))
                            .and_then(|rt| {
                                let res = match kind {
                                    TransferKind::Copy => {
                                        rt.block_on(Operations::cp(&account, &src, &dst))
                                    }
                                    TransferKind::Move => {
                                        rt.block_on(Operations::mv(&account, &src, &dst))
                                    }
                                    _ => unreachable!("chỉ Copy/Move"),
                                };
                                res.map_err(TransferError::Failed)
                            })
                    } else if src_local && dst_local {
                        // Local → Local: fs đồng bộ trong thread
                        let res = match kind {
                            TransferKind::Copy => copy_local(&src, &dst),
                            TransferKind::Move => move_local(&src, &dst),
                            _ => unreachable!("chỉ Copy/Move"),
                        };
                        res.map_err(TransferError::Failed)
                    } else {
                        // Không thể xảy ra: start_transfer đã chuyển thành upload/download.
                        Err(TransferError::Spawn(
                            "Copy/Move không hỗ trợ giữa hai đầu khác loại".to_string(),
                        ))
                    }
                }
            };

            // Move qua Cloud: sau khi chuyển xong thì xoá nguồn.
            if result.is_ok() && cleanup_src {
                let cleanup = if src_local {
                    delete_local_path(&src)
                } else {
                    tokio::runtime::Runtime::new()
                        .map_err(|e| e.to_string())
                        .and_then(|rt| rt.block_on(Operations::rm(&account, &src, true)))
                };
                if let Err(e) = cleanup {
                    result = Err(TransferError::Failed(format!(
                        "Đã chuyển xong nhưng không xoá được nguồn: {e}"
                    )));
                }
            }

            let _ = tx.send(AsyncResult::TransferFinished { id, result });
            ctx.request_repaint();
        });
    }
}

// ---------------------------------------------------------------------------
// Phase 12: clipboard (Ctrl+C/X/V) + drag & drop giữa hai pane
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    /// Ctrl+C/Ctrl+X: lưu selection của pane hoạt động vào clipboard nội bộ.
    fn copy_selection(&mut self, cut: bool) {
        let src = self.active_pane;
        let names: Vec<String> = self.panes[src]
            .selected
            .iter()
            .filter(|n| n.as_str() != "..")
            .cloned()
            .collect();
        if names.is_empty() {
            self.log.push("⚠️ Chưa chọn mục nào để sao chép/cắt.".to_string());
            return;
        }
        self.clipboard = Some(ClipboardContent {
            src_pane: src,
            src_mode: self.panes[src].mode,
            src_path: self.panes[src].path.clone(),
            names: names.clone(),
            cut,
        });
        let n = names.len();
        if cut {
            self.log.push(format!("✂️ Đã cắt {n} mục — Ctrl+V để dán."));
        } else {
            self.log.push(format!("📋 Đã sao chép {n} mục — Ctrl+V để dán."));
        }
    }

    /// Ctrl+V: dán clipboard vào pane đang hoạt động (đích khác nguồn).
    fn paste_clipboard(&mut self, ctx: &egui::Context) {
        let Some(cb) = &self.clipboard else {
            self.log
                .push("⚠️ Clipboard trống — chọn mục rồi Ctrl+C / Ctrl+X.".to_string());
            return;
        };
        let dst = self.active_pane;
        if cb.src_pane == dst && cb.src_path == self.panes[dst].path {
            self.log
                .push("📍 Đích trùng nguồn — vào thư mục khác rồi dán.".to_string());
            return;
        }
        // Ghi chú nếu pane nguồn đã đổi chế độ Local/Cloud từ lúc copy.
        if self.panes[cb.src_pane].mode != cb.src_mode {
            self.log.push(format!(
                "ℹ️ Khung {} đã đổi chế độ sang {} từ lúc copy — dán theo chế độ hiện tại.",
                pane_side(cb.src_pane),
                self.panes[cb.src_pane].mode.label(),
            ));
        }
        let (src, names, cut) = (cb.src_pane, cb.names.clone(), cb.cut);
        self.paste_names(src, dst, names, cut, ctx);
    }

    /// Dán `names` từ pane `src` sang pane `dst` (dùng chung cho clipboard và
    /// drag & drop): Copy/Move qua `start_transfer_between`; nếu cắt thì tiêu
    /// thụ clipboard và bỏ selection ở pane nguồn.
    fn paste_names(
        &mut self,
        src: usize,
        dst: usize,
        names: Vec<String>,
        cut: bool,
        ctx: &egui::Context,
    ) {
        if src == dst && self.panes[src].path == self.panes[dst].path {
            self.log
                .push("📍 Đích trùng nguồn — vào thư mục khác rồi dán.".to_string());
            return;
        }
        let kind = if cut { TransferKind::Move } else { TransferKind::Copy };
        self.start_transfer_between(src, dst, names, kind, ctx);
        if cut {
            self.clipboard = None;
            // Chỉ xoá selection nếu pane nguồn vẫn còn giữ lựa chọn.
            if !self.panes[src].selected.is_empty() {
                self.panes[src].selected.clear();
            }
        }
    }

    /// Mỗi frame khi đang kéo: highlight pane đích dưới con trỏ; khi thả chuột
    /// thì thực hiện paste (Copy mặc định, Move nếu giữ Shift).
    fn handle_drag_drop(&mut self, ctx: &egui::Context) {
        let Some(drag) = self.drag.as_ref() else {
            return;
        };
        let src = drag.src_pane;
        let names = drag.names.clone();

        let (pos, released, shift) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.any_released(),
                i.modifiers.shift,
            )
        });

        // Tìm pane đích dưới con trỏ (khác pane nguồn).
        let mut target = None;
        if let Some(pos) = pos {
            for i in 0..2 {
                if i != src && self.pane_rects[i].contains(pos) {
                    target = Some(i);
                    break;
                }
            }
        }
        self.drop_target = target;

        if released {
            if let Some(dst) = target {
                self.paste_names(src, dst, names, shift, ctx);
            }
            self.drag = None;
            self.drop_target = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Layout: sidebar trái (danh mục chức năng + tài khoản placeholder)
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    fn ui_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav_panel")
            .resizable(true)
            .default_width(210.0)
            .min_width(80.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // ── Section: Địa điểm (cục bộ) ─────────────────────────────
                ui.label(
                    egui::RichText::new("Địa điểm")
                        .strong()
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 160, 175)),
                );
                ui.add_space(4.0);

                let local_pane = self
                    .panes
                    .iter()
                    .position(|p| p.mode == PaneMode::Local);

                let home = resolve_home_dir().unwrap_or_else(|| "/".to_string());
                let active = local_pane.map(|i| self.panes[i].path == home).unwrap_or(false);
                self.sidebar_row(ui, "🏠", "Trang chủ", &home, active, ctx);
                if let Some(p) = dirs::desktop_dir() {
                    let p = p.to_string_lossy().to_string();
                    let active = local_pane.map(|i| self.panes[i].path == p).unwrap_or(false);
                    self.sidebar_row(ui, "🖥️", "Máy tính", &p, active, ctx);
                }
                if let Some(p) = dirs::document_dir() {
                    let p = p.to_string_lossy().to_string();
                    let active = local_pane.map(|i| self.panes[i].path == p).unwrap_or(false);
                    self.sidebar_row(ui, "📁", "Tài liệu", &p, active, ctx);
                }
                if let Some(p) = dirs::download_dir() {
                    let p = p.to_string_lossy().to_string();
                    let active = local_pane.map(|i| self.panes[i].path == p).unwrap_or(false);
                    self.sidebar_row(ui, "⬇️", "Tải xuống", &p, active, ctx);
                }
                if let Some(p) = dirs::picture_dir() {
                    let p = p.to_string_lossy().to_string();
                    let active = local_pane.map(|i| self.panes[i].path == p).unwrap_or(false);
                    self.sidebar_row(ui, "🖼️", "Hình ảnh", &p, active, ctx);
                }
                if let Some(p) = dirs::audio_dir() {
                    let p = p.to_string_lossy().to_string();
                    let active = local_pane.map(|i| self.panes[i].path == p).unwrap_or(false);
                    self.sidebar_row(ui, "🎵", "Nhạc", &p, active, ctx);
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);

                // ── Section: Filen Cloud ───────────────────────────────────
                ui.label(
                    egui::RichText::new("Filen Cloud")
                        .strong()
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 160, 175)),
                );
                ui.add_space(4.0);

                let cloud_root = self.view == MainView::Explorer;
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::SelectableLabel::new(cloud_root, "☁️ Cloud"),
                    )
                    .on_hover_text("Chuyển tới thư mục gốc Filen Cloud")
                    .clicked()
                {
                    self.show_cloud_root(ctx.clone());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::SelectableLabel::new(self.view == MainView::Recents, "🕘 Gần đây"),
                    )
                    .on_hover_text("Danh sách file vừa dùng (recents)")
                    .clicked()
                {
                    self.view = MainView::Recents;
                    self.load_recents(ui.ctx().clone());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::SelectableLabel::new(self.view == MainView::Sync, "🔄 Đồng bộ"),
                    )
                    .on_hover_text("Cặp đồng bộ trong syncPairs.json")
                    .clicked()
                {
                    self.view = MainView::Sync;
                    self.load_sync_pairs(ui.ctx().clone());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::SelectableLabel::new(self.view == MainView::Servers, "🖥️ Servers"),
                    )
                    .on_hover_text("WebDAV / S3 / Mount (FUSE)")
                    .clicked()
                {
                    self.view = MainView::Servers;
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);

                // ── Section: Tài khoản ─────────────────────────────────────
                ui.label(
                    egui::RichText::new("Tài khoản")
                        .strong()
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 160, 175)),
                );
                ui.add_space(4.0);

                // Account đang hoạt động + thông số dung lượng
                if let Some(email) = &self.account.active {
                    ui.label(format!("👤 {email}"));
                    ui.label(format!("📊 Đã dùng: {}", self.account.used));
                    ui.label(format!("📦 Tổng: {}", self.account.max));
                } else {
                    ui.label("👤 Chưa đăng nhập");
                }

                ui.add_space(8.0);

                // Danh sách tài khoản đã lưu để đăng nhập nhanh
                ui.label(egui::RichText::new("Tài khoản đã lưu").strong());
                if self.account.stored.is_empty() {
                    ui.weak("(trống)");
                } else {
                    for acc in self.account.stored.clone() {
                        let is_active = self.account.active.as_deref() == Some(acc.email.as_str());
                        let has_pass = !acc.password.is_empty();
                        let label = format!(
                            "{} {}{}",
                            if is_active { "●" } else { "○" },
                            acc.email,
                            if has_pass { "" } else { " (phiên CLI)" },
                        );
                        // Dòng 1: tên tài khoản (add_sized co theo panel để
                        // không ép panel nở rộng khi kéo nhỏ).
                        let resp = ui.add_sized(
                            [ui.available_width(), 22.0],
                            egui::SelectableLabel::new(is_active, label),
                        );
                        if resp.clicked() && !is_active && has_pass && !self.account.busy {
                            self.switch_account(acc.clone(), ctx.clone());
                        }
                        // Dòng 2: nút Đăng nhập nhanh (co theo panel width).
                        let can_quick = has_pass && !is_active && !self.account.busy;
                        let btn_resp = ui
                            .add_enabled_ui(can_quick, |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 24.0],
                                    egui::Button::new("Đăng nhập nhanh")
                                        .min_size(egui::vec2(0.0, 0.0)),
                                )
                            })
                            .response
                            .on_hover_text(if has_pass {
                                "Đăng nhập nhanh với mật khẩu đã lưu"
                            } else {
                                "Tài khoản CLI không có mật khẩu lưu"
                            });
                        if btn_resp.clicked() && can_quick {
                            self.switch_account(acc.clone(), ctx.clone());
                        }
                    }
                }

                ui.add_space(8.0);

                if self.account.busy {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label("Đang xử lý tài khoản...");
                    });
                }

                if ui.button("🔑 Đăng nhập mới").clicked() {
                    self.open_login_form();
                }
                let logout_enabled = self.account.active.is_some() && !self.account.busy;
                if ui
                    .add_enabled(logout_enabled, egui::Button::new("🚪 Đăng xuất"))
                    .clicked()
                {
                    let ctx = ctx.clone();
                    self.start_logout(ctx);
                }
            });
    }

    /// Hàng Places kiểu Nemo: hover nhạt, nền accent khi đang ở đúng path.
    fn sidebar_row(
        &mut self,
        ui: &mut egui::Ui,
        icon: &str,
        label: &str,
        path: &str,
        active: bool,
        ctx: &egui::Context,
    ) {
        let resp = ui.add_sized(
            [ui.available_width(), 24.0],
            egui::SelectableLabel::new(active, format!("{icon} {label}")),
        );
        if resp.clicked() && !active {
            self.navigate_local_place(path.to_string(), ctx.clone());
        }
    }

    /// Mở modal đăng nhập (reset form về trạng thái ban đầu).
    fn open_login_form(&mut self) {
        if self.account.busy {
            return;
        }
        self.login = LoginFormState::fresh();
        self.login.open = true;
    }

    /// Modal đăng nhập: email/mật khẩu + tùy chọn giữ phiên; bước 2FA khi cần.
    fn ui_login_window(&mut self, ctx: &egui::Context) {
        if !self.login.open {
            return;
        }
        let mut close = false;
        let mut submit = false;
        egui::Window::new("Đăng nhập Cloud")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                if self.login.pending_twofa {
                    ui.label(egui::RichText::new("🔐 Tài khoản yêu cầu mã 2FA").strong());
                    ui.label(format!("Email: {}", self.login.email));
                    ui.add_space(6.0);
                    ui.label("Mã 2FA:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.twofa)
                            .desired_width(200.0)
                            .hint_text("VD: 123456"),
                    );
                } else {
                    ui.label("Email:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.email)
                            .desired_width(220.0)
                            .hint_text("name@example.com"),
                    );
                    ui.label("Mật khẩu:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.password)
                            .desired_width(220.0)
                            .password(true),
                    );
                    ui.checkbox(
                        &mut self.login.keep_logged,
                        "Giữ phiên đăng nhập (lưu để đăng nhập nhanh)",
                    );
                }

                if let Some(err) = &self.login.error {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::from_rgb(255, 90, 90), err);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let btn_label = if self.login.pending_twofa {
                        "Xác nhận 2FA"
                    } else {
                        "Đăng nhập"
                    };
                    if ui
                        .add_enabled(!self.account.busy, egui::Button::new(btn_label))
                        .clicked()
                    {
                        submit = true;
                    }
                    if ui
                        .add_enabled(!self.account.busy, egui::Button::new("Hủy"))
                        .clicked()
                    {
                        close = true;
                    }
                });

                if self.account.busy {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label("Đang đăng nhập...");
                    });
                }
            });

        if submit {
            let ctx = ctx.clone();
            self.start_login(ctx);
        }
        if close {
            self.login = LoginFormState::fresh();
        }
    }
}

// ---------------------------------------------------------------------------
// Layout: central 2 pane song song
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    fn ui_panes(&mut self, ui: &mut egui::Ui) {
        let half = (ui.available_width() * 0.5).max(200.0);
        let left = egui::SidePanel::left("pane_left")
            .resizable(true)
            .default_width(half)
            .show_inside(ui, |ui| {
                self.ui_pane(ui, 0);
            });
        // Lưu rect khung trái để phát hiện thả chuột khi drag.
        self.pane_rects[0] = left.response.rect;
        // Khung phải chiếm phần còn lại (không để hở giữa hai khung)
        let rest = ui.available_rect_before_wrap();
        let right = ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rest), |ui| {
            self.ui_pane(ui, 1);
        });
        self.pane_rects[1] = right.response.rect;
    }

    fn ui_pane(&mut self, ui: &mut egui::Ui, idx: usize) {
        let is_active = self.active_pane == idx;
        let fill = if is_active {
            egui::Color32::from_rgb(22, 28, 40)
        } else {
            egui::Color32::from_rgb(15, 18, 26)
        };
        let stroke = if is_active {
            egui::Stroke::new(1.0, egui::Color32::from_rgb(94, 156, 255))
        } else {
            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 50, 58))
        };

        let frame_resp = egui::Frame::default()
            .inner_margin(egui::Margin::same(6))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.ui_pane_header(ui, idx);
                self.ui_pane_items(ui, idx);
            });

        // ── Phase 12: highlight pane đích khi đang kéo (drag & drop) ─────
        if self.drop_target == Some(idx) {
            let rect = frame_resp.response.rect;
            ui.painter().rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(94, 156, 255)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center_top() + egui::vec2(0.0, 8.0),
                egui::Align2::CENTER_TOP,
                "Thả để sao chép / di chuyển",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(94, 156, 255),
            );
        }
    }

    /// Toolbar mỗi pane kiểu Nemo: 1 hàng = điều hướng + breadcrumb + ô lọc
    /// (phải). Các thao tác file nằm trong context menu (chuột phải).
    fn ui_pane_header(&mut self, ui: &mut egui::Ui, idx: usize) {
        let can_back = !self.panes[idx].back.is_empty();
        let can_fwd = !self.panes[idx].fwd.is_empty();
        let ctx = ui.ctx().clone();

        // ── Hàng 1: điều hướng + breadcrumb + ô lọc ─────────────────────
        ui.horizontal(|ui| {
            if ui
                .add_enabled(can_back, egui::Button::new("←"))
                .on_hover_text("Quay lại (Back)")
                .clicked()
            {
                self.active_pane = idx;
                self.go_back(idx, ctx.clone());
            }
            if ui
                .add_enabled(can_fwd, egui::Button::new("→"))
                .on_hover_text("Đi tiếp (Forward)")
                .clicked()
            {
                self.active_pane = idx;
                self.go_forward(idx, ctx.clone());
            }
            if ui.button("⬆").on_hover_text("Đi lên thư mục cha").clicked() {
                self.active_pane = idx;
                self.go_up(idx, ctx.clone());
            }
            if ui.button("🏠").on_hover_text("Về thư mục gốc").clicked() {
                self.active_pane = idx;
                self.go_home(idx, ctx.clone());
            }
            if ui.button("⟳").on_hover_text("Tải lại danh sách").clicked() {
                self.active_pane = idx;
                self.list_pane(idx, ctx.clone());
            }
            if ui
                .button(format!("🔄 {}", self.panes[idx].mode.glyph()))
                .on_hover_text("Đổi chế độ Cục bộ / Cloud")
                .clicked()
            {
                self.active_pane = idx;
                self.toggle_mode(idx, ctx.clone());
            }
            ui.separator();
            self.ui_breadcrumb(ui, idx);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.panes[idx].filter)
                        .desired_width(130.0)
                        .hint_text("Lọc…"),
                );
                ui.label("🔍");
            });
        });
        ui.separator();
    }

    /// Breadcrumb kiểu Nemo: mỗi segment là nút điều hướng; nếu hẹp chỉ vẽ
    /// các segment cuối và hiện "…" phía trước.
    fn ui_breadcrumb(&mut self, ui: &mut egui::Ui, idx: usize) {
        let current = self.panes[idx].path.clone();
        let segments = segment_paths(&current);
        let avail = ui.available_width();
        let est = |s: &str| s.chars().count() as f32 * 8.0 + 18.0;
        let total: f32 = segments.iter().map(|(l, _)| est(l)).sum();
        let mut start = 0usize;
        if total > avail && segments.len() > 1 {
            let mut used = 16.0;
            let mut idx_last = segments.len();
            while idx_last > 0 {
                let w = est(&segments[idx_last - 1].0);
                if used + w > avail {
                    break;
                }
                used += w;
                idx_last -= 1;
            }
            start = idx_last.min(segments.len().saturating_sub(1));
            ui.weak("…");
        }
        for (i, (label, path)) in segments.iter().skip(start).enumerate() {
            if i > 0 {
                ui.label(egui::RichText::new("›").weak());
            }
            let is_last = path == &current;
            if is_last {
                ui.label(egui::RichText::new(label).strong());
            } else if ui.add(egui::Link::new(label)).clicked() {
                self.navigate_to_path(idx, path.clone(), ui.ctx().clone());
            }
        }
    }

    fn ui_pane_items(&mut self, ui: &mut egui::Ui, idx: usize) {
        let query = self.panes[idx].filter.trim().to_lowercase();
        let filtered: Vec<FileItem> = self.panes[idx]
            .items
            .iter()
            .filter(|it| query.is_empty() || it.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        let selected = self.panes[idx].selected.clone();
        let is_cloud = self.panes[idx].mode == PaneMode::Cloud;

        // Hành động được thu thập từ click + context menu; áp dụng sau ScrollArea.
        let mut pending: Option<PaneItemAction> = None;

        // Bố cục cột phải (khớp giữa header và row): Kích thước / Loại / Ngày sửa.
        const ROW_H: f32 = 26.0;
        const ACCENT: egui::Color32 = egui::Color32::from_rgb(94, 156, 255);
        const HOVER: egui::Color32 = egui::Color32::from_rgb(38, 46, 60);
        const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(33, 39, 50);
        const SUB: egui::Color32 = egui::Color32::from_rgb(150, 158, 170);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ── Header cột (độ rộng col_w, kéo thả được) ─────────────
                let (hrect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 24.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(hrect, 4.0, HEADER_BG);
                let hfont = egui::FontId::proportional(13.0);
                let hcol = egui::Color32::from_rgb(205, 210, 220);
                let hcy = hrect.center().y;
                let right = hrect.max.x - 8.0;
                let col_w = self.panes[idx].col_w;
                // Ranh giới trái của cột [Kích thước, Loại, Ngày sửa].
                let size_left = right - col_w[0] - col_w[1] - col_w[2];
                let kind_left = right - col_w[1] - col_w[2];
                let date_left = right - col_w[2];

                // Tên (căn giữa cột trái, cắt "…" khi hẹp).
                let name_center = (hrect.min.x + 8.0 + size_left) * 0.5;
                ellipsis_painter_text(
                    ui.painter(),
                    egui::pos2(name_center, hcy),
                    egui::Align2::CENTER_CENTER,
                    "Tên",
                    hfont.clone(),
                    hcol,
                    (size_left - hrect.min.x - 16.0).max(20.0),
                    hrect,
                );
                ellipsis_painter_text(
                    ui.painter(),
                    egui::pos2((size_left + kind_left) * 0.5, hcy),
                    egui::Align2::CENTER_CENTER,
                    "Kích thước",
                    hfont.clone(),
                    hcol,
                    col_w[0] - 6.0,
                    hrect,
                );
                ellipsis_painter_text(
                    ui.painter(),
                    egui::pos2((kind_left + date_left) * 0.5, hcy),
                    egui::Align2::CENTER_CENTER,
                    "Loại",
                    hfont.clone(),
                    hcol,
                    col_w[1] - 6.0,
                    hrect,
                );
                ellipsis_painter_text(
                    ui.painter(),
                    egui::pos2((date_left + right) * 0.5, hcy),
                    egui::Align2::CENTER_CENTER,
                    "Ngày sửa",
                    hfont,
                    hcol,
                    col_w[2] - 6.0,
                    hrect,
                );

                // ── Drag handle giữa các cột (kẹp 50–400px) ──────────────
                for (i, x) in [size_left, kind_left, date_left].into_iter().enumerate() {
                    let hrect = egui::Rect::from_center_size(
                        egui::pos2(x, hcy),
                        egui::vec2(6.0, 24.0),
                    );
                    let hresp = ui.interact(
                        hrect,
                        ui.id().with(("col_handle", idx, i)),
                        egui::Sense::drag(),
                    );
                    ui.painter().line_segment(
                        [
                            egui::pos2(x, hrect.min.y + 3.0),
                            egui::pos2(x, hrect.max.y - 3.0),
                        ],
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(160, 172, 190)),
                    );
                    if hresp.dragged() {
                        // Các cột này neo bên PHẢI (right-aligned): khi cột rộng
                        // lên, biên trái (chỗ handle) dịch sang TRÁI, ngược hướng
                        // chuột → phải trừ delta để handle bám theo chuột.
                        self.panes[idx].col_w[i] =
                            (col_w[i] - hresp.drag_delta().x).clamp(50.0, 400.0);
                    }
                }
                ui.separator();

                if filtered.is_empty() {
                    ui.weak("(trống)");
                }

                for item in &filtered {
                    let name = item.name.clone();
                    let is_dir = item.is_dir;
                    let is_sel = selected.contains(&name);
                    let is_parent = name == "..";

                    let desired = egui::vec2(ui.available_width(), ROW_H);
                    let (rect, resp) =
                        ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

                    // ── Bắt đầu kéo (drag) mục sang khung kia ──────────────
                    if resp.drag_started() && !is_parent {
                        let names = if selected.contains(&name) {
                            selected.clone()
                        } else {
                            vec![name.clone()]
                        };
                        let n = names.len();
                        self.drag = Some(DragSource {
                            src_pane: idx,
                            names,
                        });
                        self.drop_target = None;
                        self.log.push(format!(
                            "🖱️ Đang kéo {n} mục — thả sang khung kia để sao chép (giữ Shift = di chuyển)."
                        ));
                    }

                    // ── Fill hover / selected trên cả hàng ────────────────
                    let fill = if is_sel {
                        ACCENT
                    } else if resp.hovered() {
                        HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    if fill != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, 4.0, fill);
                    }

                    // ── Nội dung dòng ─────────────────────────────────────
                    let ry = rect.center().y;
                    let name_color = if is_sel {
                        egui::Color32::WHITE
                    } else if is_dir {
                        egui::Color32::from_rgb(120, 170, 255)
                    } else {
                        egui::Color32::from_rgb(230, 233, 238)
                    };
                    let sub_color = if is_sel {
                        egui::Color32::from_rgb(235, 240, 250)
                    } else {
                        SUB
                    };
                    let icon = file_icon(&name, is_dir);
                    let right = rect.max.x - 8.0;
                    let size_left = right - col_w[0] - col_w[1] - col_w[2];
                    let kind_left = right - col_w[1] - col_w[2];
                    let date_left = right - col_w[2];
                    let row_clip = egui::Rect::from_min_max(rect.min, rect.max);
                    // Tên (trái, lấp phần còn lại) — cắt "…" khi hẹp.
                    ellipsis_painter_text(
                        ui.painter(),
                        egui::pos2(rect.min.x + 8.0, ry),
                        egui::Align2::LEFT_CENTER,
                        &format!("{icon} {name}"),
                        egui::FontId::proportional(14.0),
                        name_color,
                        (size_left - rect.min.x - 16.0).max(20.0),
                        row_clip,
                    );
                    // 3 cột phải: căn giữa trong cột, cắt "…" khi tràn.
                    let size_str = format_size(item.size);
                    ellipsis_painter_text(
                        ui.painter(),
                        egui::pos2((size_left + kind_left) * 0.5, ry),
                        egui::Align2::CENTER_CENTER,
                        if is_dir { "—" } else { size_str.as_str() },
                        egui::FontId::proportional(13.0),
                        sub_color,
                        col_w[0] - 6.0,
                        row_clip,
                    );
                    ellipsis_painter_text(
                        ui.painter(),
                        egui::pos2((kind_left + date_left) * 0.5, ry),
                        egui::Align2::CENTER_CENTER,
                        if is_dir { "thư mục" } else { "tệp tin" },
                        egui::FontId::proportional(13.0),
                        sub_color,
                        col_w[1] - 6.0,
                        row_clip,
                    );
                    ellipsis_painter_text(
                        ui.painter(),
                        egui::pos2((date_left + right) * 0.5, ry),
                        egui::Align2::CENTER_CENTER,
                        &item.mod_time,
                        egui::FontId::proportional(13.0),
                        sub_color,
                        col_w[2] - 6.0,
                        row_clip,
                    );

                    // ── Click chọn / double-click mở ──────────────────────
                    if resp.clicked() {
                        let (ctrl, shift) =
                            ui.input(|i| (i.modifiers.ctrl, i.modifiers.shift));
                        if resp.double_clicked() {
                            pending = Some(PaneItemAction::Activate {
                                name: name.clone(),
                                is_dir,
                            });
                        } else if is_parent {
                            // ".." click = đi lên, không select.
                            pending = Some(PaneItemAction::Navigate(name.clone()));
                        } else if ctrl {
                            pending = Some(PaneItemAction::ToggleSelect(name.clone()));
                        } else if shift {
                            pending = Some(PaneItemAction::RangeSelect(name.clone()));
                        } else {
                            pending = Some(PaneItemAction::Select(name.clone()));
                        }
                    }

                    // ── Chuột phải: select mục trước khi mở menu (Nemo) ───
                    if resp.secondary_clicked() && !is_parent {
                        let ctrl = ui.input(|i| i.modifiers.ctrl);
                        if ctrl {
                            pending = Some(PaneItemAction::ToggleSelect(name.clone()));
                        } else if !is_sel {
                            pending = Some(PaneItemAction::Select(name.clone()));
                        }
                    }

                    // ── Context menu chuột phải (gom các thao tác file) ───
                    resp.context_menu(|ui| {
                        if is_dir {
                            if ui.button("📂 Đi vào").clicked() {
                                ui.close_menu();
                                pending = Some(PaneItemAction::Navigate(name.clone()));
                            }
                        } else if ui.button("👁️ Xem nội dung").clicked() {
                            ui.close_menu();
                            pending = Some(PaneItemAction::View(name.clone()));
                        }
                        ui.separator();
                        if !is_parent {
                            if ui.button("✏️ Đổi tên").clicked() {
                                ui.close_menu();
                                pending = Some(PaneItemAction::Rename(name.clone()));
                            }
                            if ui.button("🗑️ Xóa").clicked() {
                                ui.close_menu();
                                pending = Some(PaneItemAction::Delete(name.clone()));
                            }
                        }
                        if is_cloud && !is_parent {
                            if ui.button("⭐ Yêu thích").clicked() {
                                ui.close_menu();
                                pending = Some(PaneItemAction::Favorite(name.clone()));
                            }
                            if ui.button("☆ Bỏ yêu thích").clicked() {
                                ui.close_menu();
                                pending = Some(PaneItemAction::Unfavorite(name.clone()));
                            }
                            if ui.button("🔗 Copy link").clicked() {
                                ui.close_menu();
                                pending = Some(PaneItemAction::CopyLink(name.clone()));
                            }
                        }
                        if !is_parent
                            && ui.button("📋 Sao chép đường dẫn").clicked()
                        {
                            ui.close_menu();
                            pending = Some(PaneItemAction::CopyPath(name.clone()));
                        }
                    });
                }

                // ── Vùng trống: menu tạo thư mục mới / tải lại ────────────
                let (_, empty_resp) = ui.allocate_exact_size(
                    egui::vec2(
                        ui.available_width(),
                        ui.available_height().max(20.0),
                    ),
                    egui::Sense::click(),
                );
                empty_resp.context_menu(|ui| {
                    if ui.button("📁 Tạo thư mục mới").clicked() {
                        ui.close_menu();
                        self.active_pane = idx;
                        self.modal = Some(Modal::Mkdir {
                            input: String::new(),
                        });
                    }
                    if ui.button("⟳ Tải lại").clicked() {
                        ui.close_menu();
                        self.active_pane = idx;
                        self.list_pane(idx, ui.ctx().clone());
                    }
                });
            });

        // ── Status bar dưới mỗi pane ──────────────────────────────────────
        let n = self.panes[idx].items.len();
        let sel_count = self.panes[idx].selected.len();
        let total_size: u64 = self.panes[idx]
            .selected
            .iter()
            .filter(|n| n.as_str() != "..")
            .filter_map(|n| {
                self.panes[idx]
                    .items
                    .iter()
                    .find(|i| &i.name == n)
                    .map(|i| i.size)
            })
            .sum();
        ui.separator();
        let status = if sel_count > 0 {
            format!(
                "{n} mục — đã chọn {sel_count} (tổng {})",
                format_size(total_size)
            )
        } else {
            format!("{n} mục")
        };
        ui.label(egui::RichText::new(status).weak());

        if let Some(action) = pending {
            self.active_pane = idx;
            let ctx = ui.ctx().clone();
            match action {
                PaneItemAction::Select(name) => {
                    // Click đơn: chọn đúng mục, xoá lựa chọn cũ.
                    self.panes[idx].selected = vec![name.clone()];
                    self.panes[idx].anchor = Some(name);
                }
                PaneItemAction::ToggleSelect(name) => {
                    if let Some(pos) = self.panes[idx].selected.iter().position(|n| n == &name) {
                        self.panes[idx].selected.remove(pos);
                    } else {
                        self.panes[idx].selected.push(name.clone());
                    }
                }
                PaneItemAction::RangeSelect(name) => {
                    // Chọn khoảng từ anchor (hoặc đầu danh sách) tới mục click.
                    let items: Vec<String> = self.panes[idx]
                        .items
                        .iter()
                        .filter(|i| i.name != "..")
                        .map(|i| i.name.clone())
                        .collect();
                    let click_idx = items.iter().position(|n| n == &name);
                    let anchor_idx = self.panes[idx]
                        .anchor
                        .as_ref()
                        .and_then(|a| items.iter().position(|n| n == a));
                    match (click_idx, anchor_idx) {
                        (Some(ci), Some(ai)) => {
                            let (lo, hi) = (ai.min(ci), ai.max(ci));
                            self.panes[idx].selected = items[lo..=hi].to_vec();
                        }
                        (Some(ci), None) => {
                            self.panes[idx].selected = items[..=ci].to_vec();
                        }
                        _ => {}
                    }
                }
                PaneItemAction::Activate { name, is_dir } => {
                    if is_dir {
                        let new_path = join_path(&self.panes[idx].path, &name);
                        self.navigate_to_path(idx, new_path, ctx);
                    } else {
                        self.op_cat(idx, &name, ctx);
                    }
                }
                PaneItemAction::Navigate(dir) => {
                    let new_path = join_path(&self.panes[idx].path, &dir);
                    self.navigate_to_path(idx, new_path, ctx);
                }
                PaneItemAction::Rename(name) => self.open_rename_modal(idx, &name),
                PaneItemAction::Delete(name) => self.open_delete_modal(idx, &[name]),
                PaneItemAction::Favorite(name) => self.op_favorite(idx, &name, true, ctx),
                PaneItemAction::Unfavorite(name) => self.op_favorite(idx, &name, false, ctx),
                PaneItemAction::View(name) => self.op_cat(idx, &name, ctx),
                PaneItemAction::CopyLink(name) => self.op_copy_link(idx, &name, ctx),
                PaneItemAction::CopyPath(name) => {
                    let full = join_path(&self.panes[idx].path, &name);
                    match Operations::copy_to_clipboard(&full) {
                        Ok(()) => self.log.push(format!("📋 Đã copy đường dẫn: {full}")),
                        Err(e) => self.log.push(format!("⚠️ Không copy được đường dẫn: {e}")),
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 7: Panel Recents / Sync / Servers
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    // ── Recents ─────────────────────────────────────────────────────────────

    fn load_recents(&mut self, ctx: egui::Context) {
        let Some(account) = self.account.active.clone() else {
            self.recents.clear();
            self.recents_status = "Cần đăng nhập Cloud để xem file gần đây".to_string();
            return;
        };
        self.recents_status = "⏳ Đang tải...".to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::recents(&Some(account))));
            let _ = tx.send(AsyncResult::RecentsFinished(result));
            ctx.request_repaint();
        });
    }

    fn ui_recents(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading("🕘 Gần đây");
            if ui.button("⟳ Tải lại").clicked() {
                self.load_recents(ui.ctx().clone());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(&self.recents_status);
            });
        });
        ui.separator();

        if self.recents.is_empty() {
            ui.weak("Chưa có file gần đây. Bấm “Tải lại” để lấy danh sách.");
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut pending: Option<PaneItemAction> = None;
                let items = self.recents.clone();
                for item in &items {
                    let name = item.name.clone();
                    let is_dir = item.is_dir;
                    let row = ui.horizontal(|ui| {
                        ui.label(if is_dir { "📁" } else { "📄" });
                        ui.label(&name);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(&item.mod_time);
                            },
                        );
                    }).response;
                    row.context_menu(|ui| {
                        if !is_dir && ui.button("👁️ Xem nội dung").clicked() {
                            ui.close_menu();
                            pending = Some(PaneItemAction::View(name.clone()));
                        }
                        if !is_dir && ui.button("🔗 Copy link").clicked() {
                            ui.close_menu();
                            pending = Some(PaneItemAction::CopyLink(name.clone()));
                        }
                    });
                }
                if let Some(action) = pending {
                    let ctx = ui.ctx().clone();
                    // Recents trả tên dạng đường dẫn đầy đủ — dùng pane 0/1
                    // (pane Cloud) để thực hiện op; không cần join_path.
                    let pane = self
                        .panes
                        .iter()
                        .position(|p| p.mode == PaneMode::Cloud)
                        .unwrap_or(0);
                    match action {
                        PaneItemAction::View(name) => self.op_cat_path(pane, &name, ctx),
                        PaneItemAction::CopyLink(name) => self.op_copy_link_path(pane, &name, ctx),
                        _ => {}
                    }
                }
            });
    }

    // ── Sync ────────────────────────────────────────────────────────────────

    fn load_sync_pairs(&mut self, ctx: egui::Context) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = Operations::sync_pairs();
            let _ = tx.send(AsyncResult::SyncPairsFinished(result));
            ctx.request_repaint();
        });
    }

    fn ui_sync(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading("🔄 Đồng bộ");
            if ui.button("⟳ Đọc lại syncPairs.json").clicked() {
                self.load_sync_pairs(ui.ctx().clone());
            }
        });
        ui.separator();

        if let Some(err) = &self.sync_error {
            ui.colored_label(egui::Color32::from_rgb(255, 90, 90), err);
            return;
        }
        if self.sync_pairs.is_empty() {
            ui.weak("Chưa có cặp đồng bộ. Kiểm tra syncPairs.json trong thư mục dữ liệu filen-cli.");
            return;
        }

        // Header bảng
        ui.horizontal(|ui| {
            ui.strong("Local");
            ui.separator();
            ui.strong("Remote");
            ui.separator();
            ui.strong("Mode");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.strong("Hành động");
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let pairs = self.sync_pairs.clone();
                let in_flight = self.sync_in_flight.clone();
                for (idx, pair) in pairs.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(&pair.local);
                        ui.separator();
                        ui.label(&pair.remote);
                        ui.separator();
                        let mode = if pair.sync_mode.is_empty() {
                            "mặc định"
                        } else {
                            &pair.sync_mode
                        };
                        ui.label(mode);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let running = in_flight.contains(&idx);
                                if running {
                                    ui.add(egui::Spinner::new());
                                    ui.weak("Đang chạy…");
                                } else if ui.button("▶ Chạy").clicked() {
                                    self.run_sync_pair(idx, ui.ctx().clone());
                                }
                            },
                        );
                    });
                    ui.separator();
                }
            });
    }

    fn run_sync_pair(&mut self, idx: usize, ctx: egui::Context) {
        if self.account.active.is_none() {
            self.log.push("⚠️ Cần đăng nhập Cloud để đồng bộ.".to_string());
            return;
        }
        let Some(account) = self.account.active.clone() else {
            return;
        };
        let Some(pair) = self.sync_pairs.get(idx).cloned() else {
            return;
        };
        self.sync_in_flight.push(idx);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::sync_pair_once(&Some(account), &pair)));
            let _ = tx.send(AsyncResult::SyncPairFinished { idx, result });
            ctx.request_repaint();
        });
    }

    // ── Servers ─────────────────────────────────────────────────────────────

    fn ui_servers(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.heading("🖥️ Servers");
        ui.weak("Chạy server WebDAV / S3 / Mount FUSE từ filen-cli.");
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.ui_server_webdav(ui);
                self.ui_server_s3(ui);
                self.ui_server_mount(ui);
            });
    }

    fn server_running_badge(ui: &mut egui::Ui, running: bool) {
        if running {
            ui.colored_label(egui::Color32::from_rgb(90, 200, 120), "● Đang chạy");
        } else {
            ui.colored_label(egui::Color32::from_rgb(120, 130, 140), "○ Đã dừng");
        }
    }

    fn server_logs(ui: &mut egui::Ui, logs: &[String]) {
        egui::ScrollArea::vertical()
            .max_height(90.0)
            .show(ui, |ui| {
                if logs.is_empty() {
                    ui.weak("(chưa có log)");
                } else {
                    for line in logs {
                        ui.monospace(line);
                    }
                }
            });
    }

    fn ui_server_webdav(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(format!(
            "🌐 WebDAV Server ({})",
            if self.servers.webdav.running { "đang chạy" } else { "đã dừng" }
        ))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("User:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.webdav.user).desired_width(120.0));
                ui.label("Pass:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.webdav.pass).desired_width(120.0));
                ui.label("Port:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.webdav.port).desired_width(60.0));
                ui.checkbox(&mut self.servers.webdav.https, "HTTPS");
            });
            ui.horizontal(|ui| {
                Self::server_running_badge(ui, self.servers.webdav.running);
                if self.servers.webdav.running {
                    if ui.button("⏹ Dừng").clicked() {
                        self.stop_webdav();
                    }
                } else if ui.button("▶ Bắt đầu").clicked() {
                    self.start_webdav(ui.ctx().clone());
                }
            });
            ui.label("Log:");
            Self::server_logs(ui, &self.servers.webdav.logs);
        });
        ui.separator();
    }

    fn ui_server_s3(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(format!(
            "☁️ S3 Server ({})",
            if self.servers.s3.running { "đang chạy" } else { "đã dừng" }
        ))
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Access Key:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.s3.access_key).desired_width(120.0));
                ui.label("Secret:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.s3.secret_key).desired_width(120.0));
                ui.label("Port:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.s3.port).desired_width(60.0));
                ui.checkbox(&mut self.servers.s3.https, "HTTPS");
            });
            ui.horizontal(|ui| {
                Self::server_running_badge(ui, self.servers.s3.running);
                if self.servers.s3.running {
                    if ui.button("⏹ Dừng").clicked() {
                        self.stop_s3();
                    }
                } else if ui.button("▶ Bắt đầu").clicked() {
                    self.start_s3(ui.ctx().clone());
                }
            });
            ui.label("Log:");
            Self::server_logs(ui, &self.servers.s3.logs);
        });
        ui.separator();
    }

    fn ui_server_mount(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(format!(
            "💾 Mount FUSE ({})",
            if self.servers.mount.running { "đang chạy" } else { "đã dừng" }
        ))
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Mount point:");
                ui.add(egui::TextEdit::singleline(&mut self.servers.mount.mount_point).desired_width(200.0));
            });
            ui.weak(&self.servers.mount.note);
            ui.horizontal(|ui| {
                Self::server_running_badge(ui, self.servers.mount.running);
                if self.servers.mount.running {
                    if ui.button("⏹ Dừng (unmount)").clicked() {
                        self.stop_mount();
                    }
                } else if ui.button("▶ Mount").clicked() {
                    self.start_mount(ui.ctx().clone());
                }
            });
            ui.label("Log:");
            Self::server_logs(ui, &self.servers.mount.logs);
        });
        ui.separator();
    }

    fn start_webdav(&mut self, ctx: egui::Context) {
        if self.servers.webdav.running {
            return;
        }
        let Some(account) = self.account.active.clone() else {
            self.log.push("⚠️ Cần đăng nhập Cloud để chạy server.".to_string());
            return;
        };
        let (user, pass, port, https) = (
            self.servers.webdav.user.clone(),
            self.servers.webdav.pass.clone(),
            self.servers.webdav.port.clone(),
            self.servers.webdav.https,
        );
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = spawn_webdav(&Some(account), &user, &pass, &port, https);
            let _ = tx.send(AsyncResult::ServerStarted {
                which: ServerWhich::WebDav,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn stop_webdav(&mut self) {
        if let Some(mut child) = self.servers.webdav.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.servers.webdav.running = false;
        self.servers.webdav.logs.push("Máy chủ WebDAV đã dừng.".to_string());
    }

    fn start_s3(&mut self, ctx: egui::Context) {
        if self.servers.s3.running {
            return;
        }
        let Some(account) = self.account.active.clone() else {
            self.log.push("⚠️ Cần đăng nhập Cloud để chạy server.".to_string());
            return;
        };
        let (access, secret, port, https) = (
            self.servers.s3.access_key.clone(),
            self.servers.s3.secret_key.clone(),
            self.servers.s3.port.clone(),
            self.servers.s3.https,
        );
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = spawn_s3(&Some(account), &access, &secret, &port, https);
            let _ = tx.send(AsyncResult::ServerStarted {
                which: ServerWhich::S3,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn stop_s3(&mut self) {
        if let Some(mut child) = self.servers.s3.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.servers.s3.running = false;
        self.servers.s3.logs.push("Máy chủ S3 đã dừng.".to_string());
    }

    fn start_mount(&mut self, ctx: egui::Context) {
        if self.servers.mount.running {
            return;
        }
        let Some(account) = self.account.active.clone() else {
            self.log.push("⚠️ Cần đăng nhập Cloud để mount.".to_string());
            return;
        };
        let mount_point = self.servers.mount.mount_point.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = spawn_mount(&Some(account), &mount_point);
            let _ = tx.send(AsyncResult::ServerStarted {
                which: ServerWhich::Mount,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn stop_mount(&mut self) {
        if let Some(mut child) = self.servers.mount.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.servers.mount.running = false;
        self.servers.mount.logs.push("Mount đã dừng (unmount).".to_string());
    }
}

// ---------------------------------------------------------------------------
// Phase 7: Modal + thao tác file (mkdir/rename/delete/favorite/view/copy-link)
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    fn open_rename_modal(&mut self, _idx: usize, name: &str) {
        self.modal = Some(Modal::Rename {
            old: name.to_string(),
            input: name.to_string(),
        });
    }

    fn open_delete_modal(&mut self, _idx: usize, names: &[String]) {
        let names: Vec<String> = names
            .iter()
            .filter(|n| n.as_str() != "..")
            .cloned()
            .collect();
        self.modal = Some(Modal::Delete {
            names,
            no_trash: false,
        });
    }

    fn require_account(&mut self) -> Option<String> {
        let acc = self.account.active.clone();
        if acc.is_none() {
            self.log.push("⚠️ Cần đăng nhập Cloud để thực hiện thao tác này.".to_string());
        }
        acc
    }

    /// Chạy async op trả về `()` và gửi FileOpFinished qua mpsc.
    fn run_file_op(
        &self,
        pane: usize,
        kind: FileOpKind,
        name: String,
        f: impl FnOnce() -> Result<(), String> + Send + 'static,
        ctx: egui::Context,
    ) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = f();
            let _ = tx.send(AsyncResult::FileOpFinished { kind, pane, name, result });
            ctx.request_repaint();
        });
    }

    /// Chạy async op trả về text (cat / create_link) qua mpsc.
    fn run_file_text_op(
        &self,
        kind: FileOpKind,
        name: String,
        f: impl FnOnce() -> Result<String, String> + Send + 'static,
        ctx: egui::Context,
    ) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = f();
            let _ = tx.send(AsyncResult::FileTextFinished { kind, name, result });
            ctx.request_repaint();
        });
    }

    fn op_mkdir(&mut self, pane: usize, name: &str, ctx: egui::Context) {
        let name = name.trim().to_string();
        if name.is_empty() {
            self.log.push("⚠️ Tên thư mục không được để trống.".to_string());
            return;
        }
        if name == ".." || name.contains('/') || name.contains('\\') {
            self.log.push("⚠️ Tên thư mục không hợp lệ.".to_string());
            return;
        }
        let path = join_path(&self.panes[pane].path, &name);
        if self.panes[pane].mode == PaneMode::Cloud {
            let Some(account) = self.require_account() else {
                return;
            };
            self.run_file_op(pane, FileOpKind::Mkdir, name, move || {
                tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Khởi tạo runtime: {e}"))
                    .and_then(|rt| rt.block_on(Operations::mkdir(&Some(account), &path)))
            }, ctx);
        } else {
            self.run_file_op(pane, FileOpKind::Mkdir, name, move || {
                std::fs::create_dir_all(&path).map_err(|e| e.to_string())
            }, ctx);
        }
    }

    fn op_rename(&mut self, pane: usize, old: &str, new: &str, ctx: egui::Context) {
        let new = new.trim().to_string();
        if new.is_empty() {
            self.log.push("⚠️ Tên mới không được để trống.".to_string());
            return;
        }
        if new == old {
            return;
        }
        let src = join_path(&self.panes[pane].path, old);
        let dst = join_path(&self.panes[pane].path, &new);
        if self.panes[pane].mode == PaneMode::Cloud {
            let Some(account) = self.require_account() else {
                return;
            };
            self.run_file_op(pane, FileOpKind::Rename, old.to_string(), move || {
                tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Khởi tạo runtime: {e}"))
                    .and_then(|rt| rt.block_on(Operations::mv(&Some(account), &src, &dst)))
            }, ctx);
        } else {
            self.run_file_op(pane, FileOpKind::Rename, old.to_string(), move || {
                std::fs::rename(&src, &dst).map_err(|e| e.to_string())
            }, ctx);
        }
    }

    fn op_delete(&mut self, pane: usize, name: &str, no_trash: bool, ctx: egui::Context) {
        let path = join_path(&self.panes[pane].path, name);
        if self.panes[pane].mode == PaneMode::Cloud {
            let Some(account) = self.require_account() else {
                return;
            };
            self.run_file_op(pane, FileOpKind::Delete, name.to_string(), move || {
                tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Khởi tạo runtime: {e}"))
                    .and_then(|rt| rt.block_on(Operations::rm(&Some(account), &path, no_trash)))
            }, ctx);
        } else {
            self.run_file_op(pane, FileOpKind::Delete, name.to_string(), move || {
                let p = std::path::Path::new(&path);
                let res = if p.is_dir() {
                    std::fs::remove_dir_all(p)
                } else {
                    std::fs::remove_file(p)
                };
                res.map_err(|e| e.to_string())
            }, ctx);
        }
    }

    fn op_favorite(&mut self, pane: usize, name: &str, favorite: bool, ctx: egui::Context) {
        if self.panes[pane].mode != PaneMode::Cloud {
            self.log.push("⚠️ Yêu thích chỉ áp dụng cho file Cloud.".to_string());
            return;
        }
        let Some(account) = self.require_account() else {
            return;
        };
        let path = join_path(&self.panes[pane].path, name);
        let kind = if favorite {
            FileOpKind::Favorite
        } else {
            FileOpKind::Unfavorite
        };
        self.run_file_op(pane, kind, name.to_string(), move || {
            tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| {
                    if favorite {
                        rt.block_on(Operations::favorite(&Some(account), &path))
                    } else {
                        rt.block_on(Operations::unfavorite(&Some(account), &path))
                    }
                })
        }, ctx);
    }

    /// Xem nội dung file trong pane hiện tại (path = join pane.path + name).
    fn op_cat(&mut self, pane: usize, name: &str, ctx: egui::Context) {
        let path = join_path(&self.panes[pane].path, name);
        self.op_cat_path(pane, &path, ctx);
    }

    /// Xem nội dung theo đường dẫn đầy đủ (dùng cho cả Recents).
    fn op_cat_path(&mut self, pane: usize, path: &str, ctx: egui::Context) {
        let name = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_string();
        if self.panes[pane].mode == PaneMode::Cloud {
            let Some(account) = self.require_account() else {
                return;
            };
            let path = path.to_string();
            self.run_file_text_op(FileOpKind::View, name, move || {
                tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Khởi tạo runtime: {e}"))
                    .and_then(|rt| rt.block_on(Operations::cat(&Some(account), &path)))
            }, ctx);
        } else {
            let path = path.to_string();
            self.run_file_text_op(FileOpKind::View, name, move || {
                std::fs::read_to_string(&path).map_err(|e| e.to_string())
            }, ctx);
        }
    }

    /// Copy link trong pane hiện tại (path = join pane.path + name).
    fn op_copy_link(&mut self, pane: usize, name: &str, ctx: egui::Context) {
        let path = join_path(&self.panes[pane].path, name);
        self.op_copy_link_path(pane, &path, ctx);
    }

    /// Copy link theo đường dẫn đầy đủ (dùng cho cả Recents).
    fn op_copy_link_path(&mut self, pane: usize, path: &str, ctx: egui::Context) {
        if self.panes[pane].mode != PaneMode::Cloud {
            self.log.push("⚠️ Copy link chỉ áp dụng cho file Cloud.".to_string());
            return;
        }
        let Some(account) = self.require_account() else {
            return;
        };
        let name = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_string();
        let path = path.to_string();
        self.run_file_text_op(FileOpKind::CopyLink, name, move || {
            tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::create_link(&Some(account), &path)))
        }, ctx);
    }

    // ── Modal ────────────────────────────────────────────────────────────────

    fn ui_modal(&mut self, ctx: &egui::Context) {
        let mut action: Option<ModalAction> = None;
        if let Some(modal) = &mut self.modal {
            match modal {
                Modal::Mkdir { input } => {
                    egui::Window::new("📁 Tạo thư mục")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label("Tên thư mục mới (tạo trong thư mục hiện tại):");
                            ui.add(
                                egui::TextEdit::singleline(input).desired_width(240.0),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Tạo").clicked() {
                                    action = Some(ModalAction::Mkdir(input.clone()));
                                }
                                if ui.button("Hủy").clicked() {
                                    action = Some(ModalAction::Close);
                                }
                            });
                        });
                }
                Modal::Rename { old, input } => {
                    let old = old.clone();
                    egui::Window::new("✏️ Đổi tên")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label(format!("Đổi tên: {old}"));
                            ui.add(
                                egui::TextEdit::singleline(input).desired_width(240.0),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Đổi tên").clicked() {
                                    action = Some(ModalAction::Rename {
                                        old: old.clone(),
                                        new: input.clone(),
                                    });
                                }
                                if ui.button("Hủy").clicked() {
                                    action = Some(ModalAction::Close);
                                }
                            });
                        });
                }
                Modal::Delete { names, no_trash } => {
                    let names = names.clone();
                    let label = if names.len() == 1 {
                        format!("Xóa “{}”?", names[0])
                    } else {
                        format!(
                            "Xóa {} mục (bắt đầu “{}”)?",
                            names.len(),
                            names[0]
                        )
                    };
                    egui::Window::new("🗑️ Xóa")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label(label);
                            ui.checkbox(no_trash, "Xóa vĩnh viễn (không vào Thùng rác)");
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(egui::Button::new("Xóa").fill(egui::Color32::from_rgb(120, 40, 40)))
                                    .clicked()
                                {
                                    action = Some(ModalAction::Delete {
                                        names: names.clone(),
                                        no_trash: *no_trash,
                                    });
                                }
                                if ui.button("Hủy").clicked() {
                                    action = Some(ModalAction::Close);
                                }
                            });
                        });
                }
                Modal::View { title, content } => {
                    let title = title.clone();
                    let content = content.clone();
                    egui::Window::new(format!("👁️ Xem: {title}"))
                        .default_width(520.0)
                        .default_height(360.0)
                        .show(ctx, |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.monospace(&content);
                                });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy nội dung").clicked() {
                                    action = Some(ModalAction::CopyText(content.clone()));
                                }
                                if ui.button("Đóng").clicked() {
                                    action = Some(ModalAction::Close);
                                }
                            });
                        });
                }
                Modal::Link { url } => {
                    let url = url.clone();
                    egui::Window::new("🔗 Public Link")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(&url).monospace()));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy link").clicked() {
                                    action = Some(ModalAction::CopyText(url.clone()));
                                }
                                if ui.button("Đóng").clicked() {
                                    action = Some(ModalAction::Close);
                                }
                            });
                        });
                }
            }
        }

        match action {
            Some(ModalAction::Close) => {
                self.modal = None;
            }
            Some(ModalAction::Mkdir(name)) => {
                self.modal = None;
                let pane = self.active_pane;
                self.op_mkdir(pane, &name, ctx.clone());
            }
            Some(ModalAction::Rename { old, new }) => {
                self.modal = None;
                let pane = self.active_pane;
                self.op_rename(pane, &old, &new, ctx.clone());
            }
            Some(ModalAction::Delete { names, no_trash }) => {
                self.modal = None;
                let pane = self.active_pane;
                for name in &names {
                    self.op_delete(pane, name, no_trash, ctx.clone());
                }
            }
            Some(ModalAction::CopyText(text)) => {
                match Operations::copy_to_clipboard(&text) {
                    Ok(()) => self.log.push("📋 Đã copy vào clipboard.".to_string()),
                    Err(e) => self.log.push(format!("⚠️ Không copy được: {e}")),
                }
            }
            None => {}
        }
    }
}

/// Hành động thu thập từ modal, xử lý sau khi window đóng.
enum ModalAction {
    Close,
    Mkdir(String),
    Rename { old: String, new: String },
    Delete { names: Vec<String>, no_trash: bool },
    CopyText(String),
}

// ---------------------------------------------------------------------------
// Layout: bottom (status bar + log panel)
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    fn ui_bottom(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .default_height(26.0)
            .show(ctx, |ui| {
                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    let last = self
                        .log
                        .last()
                        .cloned()
                        .unwrap_or_else(|| "Sẵn sàng".to_string());
                    ui.label(egui::RichText::new(last).weak());
                    let active = self.transfer.running_count();
                    if active > 0 {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "⏳ {active} transfer đang chạy"
                                    ))
                                    .color(egui::Color32::from_rgb(94, 156, 255)),
                                );
                            },
                        );
                    }
                });
                ui.add_space(3.0);
            });
    }
}

// ---------------------------------------------------------------------------
// Layout: panel transfer (collapsible, nằm trên log panel)
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    fn ui_transfers(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("transfer_panel")
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                let total = self.transfer.items.len();
                let active = self.transfer.running_count();
                egui::CollapsingHeader::new(format!("📦 Transfer ({total} — {active} đang chạy)"))
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.transfer.items.is_empty() {
                            ui.weak("Chưa có transfer nào.");
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    for idx in 0..self.transfer.items.len() {
                                        self.ui_transfer_row(ui, idx);
                                        ui.separator();
                                    }
                                });
                            ui.horizontal(|ui| {
                                if ui.small_button("Xoá mục đã xong").clicked() {
                                    self.transfer.remove_finished();
                                }
                                if ui.small_button("Huỷ tất cả").clicked() {
                                    self.transfer.cancel_all();
                                }
                            });
                        }
                    });
            });
    }

    fn ui_transfer_row(&mut self, ui: &mut egui::Ui, idx: usize) {
        let Some(item) = self.transfer.items.get(idx) else {
            return;
        };
        let id = item.id;
        let name = item.name.clone();
        let kind = item.kind;
        let status = item.status;
        let progress = item.progress;
        let bytes_done = item.bytes_done;
        let total_bytes = item.total_bytes;
        let msg = item.msg.clone();

        ui.horizontal(|ui| {
            ui.label(format!("{} {}", kind.glyph(), kind.label()));
            ui.label(name);
            ui.separator();
            match status {
                TransferStatus::Running => {
                    if let Some(p) = progress {
                        let pct = (p * 100.0).round() as u32;
                        ui.add(
                            egui::ProgressBar::new(p)
                                .desired_width(180.0)
                                .text(format!("{pct}%")),
                        );
                    } else {
                        ui.add(egui::Spinner::new());
                        ui.weak("Đang xử lý…");
                    }
                    if total_bytes > 0 {
                        ui.weak(format!(
                            "{} / {}",
                            format_size(bytes_done),
                            format_size(total_bytes)
                        ));
                    }
                }
                TransferStatus::Queued => {
                    ui.weak("⏳ Chờ…");
                }
                TransferStatus::Done => {
                    ui.colored_label(egui::Color32::from_rgb(90, 200, 120), "✓ Xong");
                }
                TransferStatus::Error => {
                    ui.colored_label(egui::Color32::from_rgb(255, 90, 90), &msg);
                }
                TransferStatus::Cancelled => {
                    ui.colored_label(egui::Color32::from_rgb(240, 200, 90), "✕ Đã huỷ");
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if matches!(
                    status,
                    TransferStatus::Queued | TransferStatus::Running
                ) && ui.small_button("Hủy").clicked()
                {
                    self.transfer.cancel(id);
                }
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Async helpers
// ---------------------------------------------------------------------------

impl FilenGuiApp {
    fn drain_async_results(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                AsyncResult::FilesListed(idx, items) => {
                    let count = items.len();
                    // Giữ lại lựa chọn còn tồn tại trong danh sách mới; xoá mục đã
                    // biến mất (đổi tên/xóa) hoặc ".." để tránh thao tác sai đường dẫn.
                    self.panes[idx].selected.retain(|n| {
                        n != ".." && items.iter().any(|i| &i.name == n)
                    });
                    self.panes[idx].items = items;
                    self.panes[idx].status = format!("✓ {count} mục");
                    self.log.push(format!(
                        "✓ [Khung {}] Đã liệt kê {count} mục tại {}",
                        pane_side(idx),
                        self.panes[idx].path,
                    ));
                }
                AsyncResult::Error(idx, err) => {
                    self.panes[idx].status = format!("✗ Lỗi: {err}");
                    self.log.push(format!("✗ [Khung {}] {err}", pane_side(idx)));
                }
                AsyncResult::WhoAmIFinished(result) => match result {
                    Ok(Some(email)) => {
                        self.account.active = Some(email.clone());
                        // Account đang active từ CLI mà chưa có trong danh sách đã lưu
                        // → thêm vào (password rỗng, sidebar hiển thị "(phiên CLI)").
                        if !self.account.stored.iter().any(|a| a.email == email) {
                            self.account.stored.push(StoredAccount {
                                email: email.clone(),
                                password: String::new(),
                            });
                            if let Err(e) = save_stored_accounts(&self.account.stored) {
                                self.log
                                    .push(format!("⚠️ Không lưu được danh sách tài khoản: {e}"));
                            }
                        }
                        self.log.push(format!("👤 Phiên đăng nhập active: {email}"));
                        self.reload_cloud_panes(ctx);
                        self.statfs_async(ctx.clone());
                    }
                    Ok(None) => {
                        self.account.active = None;
                        self.reload_cloud_panes(ctx);
                        self.log.push("👤 Chưa có tài khoản nào đang active.".into());
                    }
                    Err(e) => {
                        self.account.active = None;
                        self.reload_cloud_panes(ctx);
                        self.log
                            .push(format!("⚠️ Không kiểm tra được phiên đăng nhập: {e}"));
                    }
                },
                AsyncResult::StatfsFinished(result) => match result {
                    Ok((used, max)) => {
                        self.log.push(format!("📊 Dung lượng Cloud: {used} / {max}"));
                        self.account.used = used;
                        self.account.max = max;
                    }
                    Err(e) => {
                        self.log.push(format!("✗ Không lấy được dung lượng Cloud: {e}"));
                    }
                },
                AsyncResult::LoginFinished {
                    email,
                    password,
                    keep_logged,
                    result,
                } => {
                    self.account.busy = false;
                    match result {
                        Ok(()) => {
                            if keep_logged {
                                self.save_stored(email.clone(), password);
                            }
                            self.login = LoginFormState::fresh();
                            self.account.active = Some(email.clone());
                            self.log.push(format!("🔑 Đăng nhập thành công: {email}"));
                            self.reload_cloud_panes(ctx);
                            self.statfs_async(ctx.clone());
                        }
                        Err(e) => {
                            let err_lower = e.to_lowercase();
                            if e == "2FA_REQUIRED"
                                || err_lower.contains("twofactor")
                                || err_lower.contains("2fa")
                                || err_lower.contains("recovery key")
                            {
                                // Bước 2: mở lại form ở chế độ nhập mã 2FA
                                self.login.open = true;
                                self.login.pending_twofa = true;
                                self.login.email = email;
                                self.login.password = password;
                                self.login.keep_logged = keep_logged;
                                self.login.error =
                                    Some("Tài khoản bật 2FA. Nhập mã xác thực để tiếp tục.".to_string());
                                self.log
                                    .push("🔐 Tài khoản yêu cầu mã xác thực 2FA.".to_string());
                            } else {
                                self.login.error = Some(e.clone());
                                self.log.push(format!("✗ Đăng nhập thất bại: {e}"));
                            }
                        }
                    }
                }
                AsyncResult::LogoutFinished { email, result } => {
                    self.account.busy = false;
                    match result {
                        Ok(()) => {
                            self.remove_stored(&email);
                            if self.account.active.as_deref() == Some(email.as_str()) {
                                self.account.active = None;
                                self.account.used = "0 B".to_string();
                                self.account.max = "0 B".to_string();
                            }
                            self.log
                                .push(format!("🚪 Đã đăng xuất và gỡ tài khoản: {email}"));
                            self.reload_cloud_panes(ctx);
                        }
                        Err(e) => {
                            self.log.push(format!("✗ Đăng xuất {email} thất bại: {e}"));
                        }
                    }
                }
                AsyncResult::TransferProgress {
                    id,
                    progress,
                    bytes_done,
                    total_bytes,
                } => {
                    if let Some(item) = self.transfer.get_mut(id) {
                        item.progress = progress;
                        item.bytes_done = bytes_done;
                        item.total_bytes = total_bytes;
                    }
                }
                AsyncResult::TransferFinished { id, result } => {
                    let summary = {
                        let Some(item) = self.transfer.get_mut(id) else {
                            continue;
                        };
                        match result {
                            Ok(()) => {
                                item.status = TransferStatus::Done;
                                item.progress = Some(1.0);
                                item.msg = "Xong".to_string();
                            }
                            Err(e) => {
                                if e == TransferError::Cancelled {
                                    item.status = TransferStatus::Cancelled;
                                    item.msg = "Đã huỷ".to_string();
                                } else {
                                    item.status = TransferStatus::Error;
                                    item.msg = e.to_string();
                                }
                            }
                        }
                        (
                            item.kind,
                            item.name.clone(),
                            item.src_pane,
                            item.dst_pane,
                            item.cleanup_src,
                            item.status,
                            item.msg.clone(),
                        )
                    };
                    let (kind, name, src_pane, dst_pane, cleanup_src, status, msg) = summary;
                    let icon = match status {
                        TransferStatus::Done => "✓",
                        TransferStatus::Error => "✗",
                        TransferStatus::Cancelled => "✕",
                        _ => "⏳",
                    };
                    let detail = if status == TransferStatus::Error && !msg.is_empty() {
                        format!(": {msg}")
                    } else {
                        String::new()
                    };
                    self.log.push(format!(
                        "{icon} {} “{name}” — {}{detail}",
                        kind.label(),
                        status.label(),
                    ));
                    // Refresh pane đích (và cả nguồn khi là Move) sau khi xong.
                    if status == TransferStatus::Done {
                        self.list_pane(dst_pane, ctx.clone());
                        if kind == TransferKind::Move || cleanup_src {
                            self.list_pane(src_pane, ctx.clone());
                        }
                    }
                    self.start_pending_transfers(ctx);
                }
                // ── Phase 7: thao tác file ─────────────────────────────────
                AsyncResult::FileOpFinished {
                    kind,
                    pane,
                    name,
                    result,
                } => match result {
                    Ok(()) => {
                        self.log.push(format!("✓ {} “{name}” thành công.", kind.verb()));
                        self.list_pane(pane, ctx.clone());
                    }
                    Err(e) => {
                        self.log.push(format!("✗ {} “{name}”: {e}", kind.verb()));
                    }
                },
                AsyncResult::FileTextFinished {
                    kind,
                    name,
                    result,
                } => match kind {
                    FileOpKind::View => match result {
                        Ok(content) => {
                            self.modal = Some(Modal::View {
                                title: name,
                                content,
                            });
                        }
                        Err(e) => self.log.push(format!("✗ Xem “{name}”: {e}")),
                    },
                    FileOpKind::CopyLink => match result {
                        Ok(url) => {
                            let trimmed = url.trim().to_string();
                            self.modal = Some(Modal::Link {
                                url: trimmed.clone(),
                            });
                            match Operations::copy_to_clipboard(&trimmed) {
                                Ok(()) => self
                                    .log
                                    .push(format!("🔗 Đã copy link “{name}” vào clipboard.")),
                                Err(e) => {
                                    self.log
                                        .push(format!("🔗 Link: {trimmed} — không copy được: {e}"))
                                }
                            }
                        }
                        Err(e) => self.log.push(format!("✗ Copy link “{name}”: {e}")),
                    },
                    _ => {}
                },
                AsyncResult::RecentsFinished(result) => match result {
                    Ok(items) => {
                        self.recents = items;
                        self.recents_status = format!("✓ {} mục", self.recents.len());
                    }
                    Err(e) => {
                        self.recents.clear();
                        self.recents_status = format!("✗ Lỗi: {e}");
                        self.log.push(format!("✗ Recents: {e}"));
                    }
                },
                AsyncResult::SyncPairsFinished(result) => match result {
                    Ok(pairs) => {
                        self.sync_pairs = pairs;
                        self.sync_error = None;
                        self.log
                            .push(format!("🔄 Đã đọc {} cặp đồng bộ.", self.sync_pairs.len()));
                    }
                    Err(e) => {
                        self.sync_pairs.clear();
                        self.sync_error = Some(e.clone());
                        self.log.push(format!("✗ Đọc syncPairs: {e}"));
                    }
                },
                AsyncResult::SyncPairFinished { idx, result } => {
                    self.sync_in_flight.retain(|&i| i != idx);
                    match result {
                        Ok(()) => {
                            self.log
                                .push(format!("🔄 Đồng bộ cặp #{} thành công.", idx + 1));
                        }
                        Err(e) => {
                            self.log
                                .push(format!("✗ Đồng bộ cặp #{} lỗi: {e}", idx + 1));
                        }
                    }
                }
                AsyncResult::ServerStarted { which, result } => match which {
                    ServerWhich::WebDav => {
                        let state = &mut self.servers.webdav;
                        match result {
                            Ok(child) => {
                                state.child = Some(child);
                                state.running = true;
                                state.logs.push(format!(
                                    "Đã khởi chạy WebDAV trên cổng {}.",
                                    state.port
                                ));
                            }
                            Err(e) => state.logs.push(format!("Lỗi khi bật WebDAV: {e}")),
                        }
                    }
                    ServerWhich::S3 => {
                        let state = &mut self.servers.s3;
                        match result {
                            Ok(child) => {
                                state.child = Some(child);
                                state.running = true;
                                state.logs.push(format!(
                                    "Đã khởi chạy S3 trên cổng {}.",
                                    state.port
                                ));
                            }
                            Err(e) => state.logs.push(format!("Lỗi khi bật S3: {e}")),
                        }
                    }
                    ServerWhich::Mount => {
                        let state = &mut self.servers.mount;
                        match result {
                            Ok(child) => {
                                state.child = Some(child);
                                state.running = true;
                                state.logs.push(format!(
                                    "Đã mount tại {}.",
                                    state.mount_point
                                ));
                            }
                            Err(e) => state.logs.push(format!("Lỗi khi mount: {e}")),
                        }
                    }
                },
            }
        }
    }

    /// Tài khoản Filen đang hoạt động (None nếu chưa đăng nhập).
    fn active_account(&self) -> Option<String> {
        self.account.active.clone()
    }

    /// Liệt kê file/thư mục cục bộ của một khung (async, chạy nền).
    fn list_local(&mut self, idx: usize, ctx: egui::Context) {
        let pane = &mut self.panes[idx];
        pane.status = "⏳ Đang liệt kê...".to_string();
        let target = pane.path.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match Operations::list_local(&target) {
                Ok(items) => {
                    let _ = tx.send(AsyncResult::FilesListed(idx, items));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(idx, format!("list_local({target}): {e}")));
                }
            }
            ctx.request_repaint();
        });
    }

    /// Liệt kê thư mục Cloud của một khung: chạy `list_remote` (async) trong thread
    /// với tokio runtime riêng, gửi kết quả qua mpsc. Nếu chưa có account active
    /// thì chỉ hiển thị trạng thái, không gọi CLI.
    fn list_cloud(&mut self, idx: usize, ctx: egui::Context) {
        let active_account = self.active_account();
        if active_account.is_none() {
            let pane = &mut self.panes[idx];
            pane.items.clear();
            pane.status = "Chưa đăng nhập tài khoản Cloud".to_string();
            return;
        }
        let pane = &mut self.panes[idx];
        pane.status = "⏳ Đang liệt kê Cloud...".to_string();
        let target = pane.path.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::list_remote(&active_account, &target)));
            match result {
                Ok(items) => {
                    let _ = tx.send(AsyncResult::FilesListed(idx, items));
                }
                Err(e) => {
                    let _ = tx.send(AsyncResult::Error(idx, format!("list_remote({target}): {e}")));
                }
            }
            ctx.request_repaint();
        });
    }

    /// Liệt kê theo chế độ của khung: Local gọi `list_local`, Cloud gọi `list_cloud`.
    fn list_pane(&mut self, idx: usize, ctx: egui::Context) {
        if self.panes[idx].mode == PaneMode::Local {
            self.list_local(idx, ctx);
        } else {
            self.list_cloud(idx, ctx);
        }
    }

    /// Đẩy đường dẫn hiện tại vào lịch sử "back" trước khi điều hướng
    /// (giới hạn 100 phần tử để tránh phình bộ nhớ).
    fn push_nav_history(&mut self, idx: usize) {
        let path = self.panes[idx].path.clone();
        let back = &mut self.panes[idx].back;
        if back.last().map(|s| s == &path).unwrap_or(false) {
            return;
        }
        back.push(path);
        if back.len() > 100 {
            back.remove(0);
        }
        self.panes[idx].fwd.clear();
    }

    /// Điều hướng pane tới `path` (đẩy lịch sử back, clear selection, list).
    fn navigate_to_path(&mut self, idx: usize, path: String, ctx: egui::Context) {
        self.active_pane = idx;
        if self.panes[idx].path == path {
            return;
        }
        self.push_nav_history(idx);
        self.panes[idx].path = path.clone();
        self.panes[idx].selected.clear();
        self.panes[idx].anchor = None;
        let side = pane_side(idx);
        self.log.push(format!("📂 [Khung {side}] Đi vào: {path}"));
        self.list_pane(idx, ctx);
    }

    /// Back: lấy đường dẫn trước từ stack, đẩy đường dẫn hiện tại vào fwd.
    fn go_back(&mut self, idx: usize, ctx: egui::Context) {
        let Some(prev) = self.panes[idx].back.pop() else {
            return;
        };
        let cur = self.panes[idx].path.clone();
        self.panes[idx].fwd.push(cur);
        self.panes[idx].path = prev.clone();
        self.panes[idx].selected.clear();
        self.panes[idx].anchor = None;
        let side = pane_side(idx);
        self.log.push(format!("◀ [Khung {side}] Quay lại: {prev}"));
        self.list_pane(idx, ctx);
    }

    /// Forward: lấy đường dẫn tiếp từ stack, đẩy đường dẫn hiện tại vào back.
    fn go_forward(&mut self, idx: usize, ctx: egui::Context) {
        let Some(next) = self.panes[idx].fwd.pop() else {
            return;
        };
        let cur = self.panes[idx].path.clone();
        self.panes[idx].back.push(cur);
        self.panes[idx].path = next.clone();
        self.panes[idx].selected.clear();
        self.panes[idx].anchor = None;
        let side = pane_side(idx);
        self.log.push(format!("▶ [Khung {side}] Đi tiếp: {next}"));
        self.list_pane(idx, ctx);
    }

    /// Điều hướng tới một "địa điểm" cục bộ: ưu tiên pane hoạt động nếu là
    /// Local, nếu không dùng pane Local còn lại; nếu cả hai đều Cloud thì
    /// chuyển pane hoạt động sang Local.
    fn navigate_local_place(&mut self, path: String, ctx: egui::Context) {
        let idx = if self.panes[self.active_pane].mode == PaneMode::Local {
            self.active_pane
        } else if self.panes[1 - self.active_pane].mode == PaneMode::Local {
            1 - self.active_pane
        } else {
            self.panes[self.active_pane].mode = PaneMode::Local;
            self.panes[self.active_pane].back.clear();
            self.panes[self.active_pane].fwd.clear();
            self.active_pane
        };
        self.navigate_to_path(idx, path, ctx);
    }

    /// Chuyển tới thư mục gốc Cloud: đưa pane Cloud ra hoạt động rồi về "/".
    fn show_cloud_root(&mut self, ctx: egui::Context) {
        self.view = MainView::Explorer;
        let idx = if self.panes[self.active_pane].mode == PaneMode::Cloud {
            self.active_pane
        } else if self.panes[1 - self.active_pane].mode == PaneMode::Cloud {
            1 - self.active_pane
        } else {
            self.panes[self.active_pane].mode = PaneMode::Cloud;
            self.panes[self.active_pane].back.clear();
            self.panes[self.active_pane].fwd.clear();
            self.active_pane
        };
        self.navigate_to_path(idx, "/".to_string(), ctx);
    }

    /// Về thư mục gốc của khung: Home máy (Local) hoặc "/" (Cloud).
    fn go_home(&mut self, idx: usize, ctx: egui::Context) {
        let path = match self.panes[idx].mode {
            PaneMode::Cloud => "/".to_string(),
            PaneMode::Local => resolve_home_dir().unwrap_or_else(|| "/".to_string()),
        };
        self.navigate_to_path(idx, path.clone(), ctx);
    }

    /// Đi lên thư mục cha của khung (nút "⬆"): cắt path về parent, cloud/local
    /// tương ứng; nếu đã ở gốc thì không làm gì.
    fn go_up(&mut self, idx: usize, ctx: egui::Context) {
        let current = self.panes[idx].path.clone();
        let parent = join_path(&current, "..");
        if parent != current {
            self.navigate_to_path(idx, parent, ctx);
        }
    }

    /// Đổi chế độ Local/Cloud của một khung rồi liệt kê lại theo chế độ mới.
    fn toggle_mode(&mut self, idx: usize, ctx: egui::Context) {
        let new_mode = match self.panes[idx].mode {
            PaneMode::Local => PaneMode::Cloud,
            PaneMode::Cloud => PaneMode::Local,
        };
        let pane = &mut self.panes[idx];
        pane.mode = new_mode;
        pane.items.clear();
        pane.selected.clear();
        pane.anchor = None;
        pane.back.clear();
        pane.fwd.clear();
        pane.status = "Sẵn sàng".to_string();
        pane.path = match pane.mode {
            PaneMode::Cloud => "/".to_string(),
            PaneMode::Local => resolve_home_dir().unwrap_or_else(|| "/".to_string()),
        };
        let side = pane_side(idx);
        let mode_label = pane.mode.label();
        self.log.push(format!("🔄 Khung {side} chuyển sang chế độ {mode_label}"));
        self.list_pane(idx, ctx);
    }

    // ── Luồng tài khoản (whoami / statfs / login / logout / switch) ──────────

    /// Chạy whoami nền: trả về email account active hoặc None nếu chưa đăng nhập.
    fn whoami_async(&mut self, ctx: egui::Context) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::whoami(&None)));
            let mapped = result.map(|email| {
                let email_clean = email.trim().to_string();
                if !email_clean.is_empty()
                    && !email_clean.contains("Please enter")
                    && !email_clean.contains("credentials")
                    && email_clean != "anonymous@filen.io"
                {
                    Some(email_clean)
                } else {
                    None
                }
            });
            let _ = tx.send(AsyncResult::WhoAmIFinished(mapped));
            ctx.request_repaint();
        });
    }

    /// Chạy statfs nền để cập nhật used/max cho sidebar.
    fn statfs_async(&mut self, ctx: egui::Context) {
        if self.account.active.is_none() {
            self.account.used = "0 B".to_string();
            self.account.max = "0 B".to_string();
            return;
        }
        let tx = self.tx.clone();
        let active = self.account.active.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::statfs(&active)));
            let _ = tx.send(AsyncResult::StatfsFinished(result));
            ctx.request_repaint();
        });
    }

    /// Bắt đầu đăng nhập từ form (có thể kèm mã 2FA ở lần thử thứ hai).
    fn start_login(&mut self, ctx: egui::Context) {
        let email = self.login.email.trim().to_string();
        let password = self.login.password.clone();
        let keep_logged = self.login.keep_logged;
        let twofa_code = if self.login.pending_twofa {
            Some(self.login.twofa.trim().to_string())
        } else {
            None
        };
        if email.is_empty() || password.is_empty() {
            self.login.error = Some("Vui lòng nhập đầy đủ Email và Mật khẩu.".to_string());
            return;
        }
        self.account.busy = true;
        self.login.error = None;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let keep_str = if keep_logged { "y" } else { "n" };
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| {
                    rt.block_on(Operations::login_new(
                        &email,
                        &password,
                        twofa_code.as_deref(),
                        keep_str,
                        None,
                    ))
                });
            let _ = tx.send(AsyncResult::LoginFinished {
                email,
                password,
                keep_logged,
                result,
            });
            ctx.request_repaint();
        });
    }

    /// Đăng xuất account active (bất đồng bộ); khi thành công sẽ gỡ khỏi danh sách.
    fn start_logout(&mut self, ctx: egui::Context) {
        let Some(email) = self.account.active.clone() else {
            return;
        };
        self.account.busy = true;
        let tx = self.tx.clone();
        let active = Some(email.clone());
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| rt.block_on(Operations::logout(&active)));
            let _ = tx.send(AsyncResult::LogoutFinished { email, result });
            ctx.request_repaint();
        });
    }

    /// Chuyển account active bằng cách tái đăng nhập với mật khẩu đã lưu
    /// (giống Alt+S của TUI: login_new với keep="y").
    fn switch_account(&mut self, acc: StoredAccount, ctx: egui::Context) {
        if self.account.busy {
            return;
        }
        self.account.busy = true;
        let tx = self.tx.clone();
        let email = acc.email;
        let password = acc.password;
        self.log.push(format!("🔄 Đang chuyển sang tài khoản: {email}"));
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Khởi tạo runtime: {e}"))
                .and_then(|rt| {
                    rt.block_on(Operations::login_new(&email, &password, None, "y", None))
                });
            let _ = tx.send(AsyncResult::LoginFinished {
                email,
                password,
                keep_logged: true,
                result,
            });
            ctx.request_repaint();
        });
    }

    /// Tải lại tất cả pane Cloud sau khi đổi account (reload).
    fn reload_cloud_panes(&mut self, ctx: &egui::Context) {
        for idx in 0..self.panes.len() {
            if self.panes[idx].mode == PaneMode::Cloud {
                self.list_cloud(idx, ctx.clone());
            }
        }
    }

    /// Thêm/cập nhật tài khoản vào danh sách đã lưu rồi ghi ra file JSON.
    fn save_stored(&mut self, email: String, password: String) {
        if let Some(pos) = self.account.stored.iter().position(|a| a.email == email) {
            self.account.stored[pos].password = password;
        } else {
            self.account.stored.push(StoredAccount { email, password });
        }
        if let Err(e) = save_stored_accounts(&self.account.stored) {
            self.log.push(format!("⚠️ Không lưu được danh sách tài khoản: {e}"));
        }
    }

    /// Gỡ tài khoản khỏi danh sách đã lưu và ghi ra file JSON.
    fn remove_stored(&mut self, email: &str) {
        if let Some(pos) = self.account.stored.iter().position(|a| a.email == email) {
            self.account.stored.remove(pos);
            if let Err(e) = save_stored_accounts(&self.account.stored) {
                self.log.push(format!("⚠️ Không lưu được danh sách tài khoản: {e}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Vẽ text theo anchor nhưng cắt (clip) theo rect giới hạn — dùng cho cột Tên
/// khi pane hẹp (text không tràn sang cột bên phải).
/// Vẽ text có giới hạn bề rộng: nếu vượt quá `max_w` thì cắt và thêm "…"
/// (binary search trên số ký tự để giữ đúng `max_w`), sau đó clip theo `clip`.
#[allow(clippy::too_many_arguments)]
fn ellipsis_painter_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    anchor: egui::Align2,
    text: &str,
    font_id: egui::FontId,
    color: egui::Color32,
    max_w: f32,
    clip: egui::Rect,
) {
    let mut content = text.to_string();
    let mut galley = painter.layout_no_wrap(content.clone(), font_id.clone(), color);
    if galley.size().x > max_w && !content.is_empty() {
        let ell = "…";
        let total = content.chars().count();
        let mut lo = 0usize;
        let mut hi = total; // số ký tự giữ lại (chưa tính dấu "…")
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let truncated: String = content.chars().take(mid).collect();
            let g = painter.layout_no_wrap(format!("{truncated}{ell}"), font_id.clone(), color);
            if g.size().x <= max_w {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        content = format!("{}…", content.chars().take(lo).collect::<String>());
        galley = painter.layout_no_wrap(content, font_id.clone(), color);
    }
    let rect = anchor.anchor_size(pos, galley.size());
    let painter = painter.with_clip_rect(clip);
    painter.galley(rect.min, galley, color);
}

/// Tên hiển thị của khung theo index (0 = trái, 1 = phải).
fn pane_side(idx: usize) -> &'static str {
    if idx == 0 { "trái" } else { "phải" }
}

/// Định dạng kích thước byte thành chuỗi dễ đọc (KB/MB/GB nếu lớn).
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if bytes as f64 >= GB {
        format!("{:.2} GB", bytes as f64 / GB)
    } else if bytes as f64 >= MB {
        format!("{:.2} MB", bytes as f64 / MB)
    } else if bytes as f64 >= KB {
        format!("{:.2} KB", bytes as f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Tách đường dẫn thành các segment `(nhãn, đường dẫn tích lũy)` cho breadcrumb.
/// Ví dụ `/home/user` → [("/", "/"), ("home", "/home"), ("user", "/home/user")].
fn segment_paths(path: &str) -> Vec<(String, String)> {
    let mut segments = Vec::new();
    let mut acc = String::new();
    if path.starts_with('/') {
        segments.push(("/".to_string(), "/".to_string()));
    }
    for seg in path.split('/').filter(|s| !s.is_empty()) {
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(seg);
        segments.push((seg.to_string(), acc.clone()));
    }
    if segments.is_empty() {
        segments.push((path.to_string(), path.to_string()));
    }
    segments
}

/// Icon cho dòng file dựa theo loại (thư mục / phần mở rộng).
fn file_icon(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return "📁";
    }
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "txt" | "md" | "rs" | "json" | "toml" | "log" | "csv" | "yaml" | "yml" => "📄",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" => "🖼️",
        "mp3" | "wav" | "ogg" | "flac" | "m4a" => "🎵",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "🎬",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "📦",
        "pdf" => "📕",
        _ => "📄",
    }
}

/// Nối tên thư mục vào đường dẫn hiện tại; xử lý ".." (về thư mục cha).
fn join_path(base: &str, name: &str) -> String {
    if name == ".." {
        std::path::Path::new(base)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| base.to_string())
    } else {
        std::path::Path::new(base)
            .join(name)
            .to_string_lossy()
            .to_string()
    }
}

/// Xác định thư mục home theo nền tảng (không dùng thư viện ngoài).
fn resolve_home_dir() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return Some(userprofile);
        }
        if let (Ok(drive), Ok(path)) =
            (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH"))
        {
            return Some(format!("{}{}", drive, path));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(home);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Phase 7: spawn tiến trình server (std::process) + mở trình duyệt
// ---------------------------------------------------------------------------

/// Spawn server WebDAV: trả về `Child` (giữ trong app để dừng sau; không kill
/// khi drop). Chạy trong thread để không block UI.
fn spawn_webdav(
    account: &Option<String>,
    user: &str,
    pass: &str,
    port: &str,
    https: bool,
) -> Result<std::process::Child, String> {
    let mut cmd = Operations::get_std_command(account);
    cmd.args(webdav_args(user, pass, port, https));
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd.spawn().map_err(|e| format!("Không khởi động được server WebDAV: {e}"))
}

/// Spawn server S3: trả về `Child`.
fn spawn_s3(
    account: &Option<String>,
    access_key: &str,
    secret_key: &str,
    port: &str,
    https: bool,
) -> Result<std::process::Child, String> {
    let mut cmd = Operations::get_std_command(account);
    cmd.args(s3_args(access_key, secret_key, port, https));
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd.spawn().map_err(|e| format!("Không khởi động được server S3: {e}"))
}

/// Spawn mount FUSE: trả về `Child` (tiến trình chạy nền; kill để unmount).
fn spawn_mount(
    account: &Option<String>,
    mount_point: &str,
) -> Result<std::process::Child, String> {
    let mut cmd = Operations::get_std_command(account);
    cmd.args(mount_args(Some(mount_point)));
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd.spawn().map_err(|e| format!("Không mount được (kiểm tra FUSE): {e}"))
}
