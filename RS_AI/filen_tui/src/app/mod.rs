pub mod operations;
pub mod key_handlers;

use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use crossterm::event::KeyEvent;

use crate::ui;
use operations::{FileItem, Operations};

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    Explorer,
    Account,
    Servers,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PopupState {
    None,
    RenameInput { old_name: String, buffer: String },
    NewFolderInput { buffer: String },
    LoginInput { email_buffer: String, pass_buffer: String, active_field: usize }, // 0: email, 1: password
    ConfirmDelete { name: String },
    #[allow(dead_code)]
    ConfirmEmptyTrash,
    SpecialActionsMenu { selected_idx: usize },
    ViewFile { name: String, content: Vec<String>, scroll: usize },
    Message { title: String, message: String },
    SwitchAccountMenu { selected_idx: usize },
}

#[derive(Debug, Clone)]
pub struct PaneState {
    pub path: String,
    pub items: Vec<FileItem>,
    pub selected_idx: usize,
    pub scroll_offset: usize,
    pub is_local: bool,
    #[allow(dead_code)]
    pub remote: String,
    pub selected_names: HashSet<String>,
    pub shift_anchor: Option<usize>,
    pub shift_active: bool,
    pub loading: bool,
}

impl PaneState {
    pub fn new(is_local: bool, default_path: String) -> Self {
        PaneState {
            path: default_path,
            items: Vec::new(),
            selected_idx: 0,
            scroll_offset: 0,
            is_local,
            remote: if is_local { String::new() } else { "cloud".to_string() },
            selected_names: HashSet::new(),
            shift_anchor: None,
            shift_active: false,
            loading: false,
        }
    }

    pub fn adjust_scroll(&mut self, height: usize) {
        if self.items.is_empty() {
            self.selected_idx = 0;
            self.scroll_offset = 0;
            return;
        }
        if self.selected_idx >= self.items.len() {
            self.selected_idx = self.items.len() - 1;
        }
        if self.selected_idx < self.scroll_offset {
            self.scroll_offset = self.selected_idx;
        } else if self.selected_idx >= self.scroll_offset + height {
            self.scroll_offset = self.selected_idx - height + 1;
        }
    }
}

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    AsyncFinished(Result<(), String>),
    RemoteLoadFinished { is_left: bool, result: Result<Vec<FileItem>, String> },
}

pub struct WebDavServerState {
    pub running: bool,
    pub user: String,
    pub pass: String,
    pub port: String,
    pub https: bool,
    pub child: Option<tokio::process::Child>,
    pub logs: Vec<String>,
}

pub struct S3ServerState {
    pub running: bool,
    pub access_key: String,
    pub secret_key: String,
    pub port: String,
    pub https: bool,
    pub child: Option<tokio::process::Child>,
    pub logs: Vec<String>,
}

pub struct App {
    pub current_screen: Screen,
    pub main_menu_selected: usize,
    pub accounts: Vec<String>,
    pub active_account: Option<String>,
    pub active_account_idx: usize, // index of active account in accounts list (including Default)
    pub storage_used: String,
    pub storage_max: String,
    
    pub left_pane: PaneState,
    pub right_pane: PaneState,
    pub active_pane_left: bool, // true if left pane is active
    
    pub clipboard: Vec<FileItem>,
    pub clipboard_src_path: String,
    pub clipboard_src_is_local: bool,
    pub clipboard_src_account: Option<String>,
    pub clipboard_is_cut: bool,
    
    pub popup_state: PopupState,
    pub edit_cursor_idx: usize,
    pub msg_tx: Option<mpsc::UnboundedSender<AppEvent>>,
    pub is_loading: bool,
    
    pub webdav_server: WebDavServerState,
    pub s3_server: S3ServerState,
    pub active_server_tab: usize, // 0: WebDAV, 1: S3
    pub server_selected_field: usize, // index of configurable fields in Server tab
}

impl App {
    pub fn new() -> Self {
        let home_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")).to_string_lossy().to_string();
        
        App {
            current_screen: Screen::MainMenu,
            main_menu_selected: 0,
            accounts: vec!["Default (Default Session)".to_string()],
            active_account: None,
            active_account_idx: 0,
            storage_used: "0 B".to_string(),
            storage_max: "20 GiB".to_string(),
            
            left_pane: PaneState::new(true, home_path.clone()),
            right_pane: PaneState::new(false, "/".to_string()),
            active_pane_left: true,
            
            clipboard: Vec::new(),
            clipboard_src_path: String::new(),
            clipboard_src_is_local: false,
            clipboard_src_account: None,
            clipboard_is_cut: false,
            
            popup_state: PopupState::None,
            edit_cursor_idx: 0,
            msg_tx: None,
            is_loading: false,
            
            webdav_server: WebDavServerState {
                running: false,
                user: "admin".to_string(),
                pass: "admin123".to_string(),
                port: "8080".to_string(),
                https: false,
                child: None,
                logs: Vec::new(),
            },
            s3_server: S3ServerState {
                running: false,
                access_key: "s3key".to_string(),
                secret_key: "s3secret".to_string(),
                port: "9000".to_string(),
                https: false,
                child: None,
                logs: Vec::new(),
            },
            active_server_tab: 0,
            server_selected_field: 0,
        }
    }

    // Làm mới danh sách tài khoản đã lưu
    pub fn refresh_accounts(&mut self) {
        self.accounts = vec!["Default (Default Session)".to_string()];
        if let Some(home) = dirs::home_dir() {
            let accounts_dir = home.join(".config/filen-cli/accounts");
            if accounts_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(accounts_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if entry.path().is_dir() {
                                self.accounts.push(name);
                            }
                        }
                    }
                }
            }
        }
        
        // Điều chỉnh con trỏ index tài khoản active
        if let Some(ref active) = self.active_account {
            if let Some(pos) = self.accounts.iter().position(|a| a == active) {
                self.active_account_idx = pos;
            } else {
                self.active_account = None;
                self.active_account_idx = 0;
            }
        } else {
            self.active_account_idx = 0;
        }
    }

    // Di chuyển con trỏ duyệt file
    pub fn select_next(&mut self) {
        let pane = if self.active_pane_left { &mut self.left_pane } else { &mut self.right_pane };
        if !pane.items.is_empty() {
            pane.selected_idx = (pane.selected_idx + 1) % pane.items.len();
        }
    }

    pub fn select_prev(&mut self) {
        let pane = if self.active_pane_left { &mut self.left_pane } else { &mut self.right_pane };
        if !pane.items.is_empty() {
            if pane.selected_idx == 0 {
                pane.selected_idx = pane.items.len() - 1;
            } else {
                pane.selected_idx -= 1;
            }
        }
    }

    // Làm mới danh sách file trên giao diện duyệt file
    pub async fn refresh_active_pane(&mut self) {
        let is_left = self.active_pane_left;
        let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
        pane.loading = true;
        
        let path = pane.path.clone();
        let is_local = pane.is_local;
        let active_account = self.active_account.clone();
        
        if is_local {
            match Operations::list_local(&path) {
                Ok(items) => {
                    let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
                    pane.items = items;
                    if pane.selected_idx >= pane.items.len() {
                        pane.selected_idx = 0;
                    }
                }
                Err(e) => {
                    self.popup_state = PopupState::Message {
                        title: "Lỗi đọc Local".to_string(),
                        message: e,
                    };
                }
            }
            let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
            pane.loading = false;
        } else {
            // Thực hiện gọi CLI bất đồng bộ để không treo UI
            let tx = self.msg_tx.clone();
            tokio::spawn(async move {
                let res = Operations::list_remote(&active_account, &path).await;
                if let Some(tx) = tx {
                    let _ = tx.send(AppEvent::RemoteLoadFinished {
                        is_left,
                        result: res,
                    });
                }
            });
        }
    }

    // Làm mới thông số bộ nhớ đám mây
    pub async fn refresh_storage_info(&mut self) {
        if let Ok((used, max)) = Operations::statfs(&self.active_account).await {
            self.storage_used = used;
            self.storage_max = max;
        }
    }

    // Thực chạy vòng lặp TUI chính
    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.msg_tx = Some(tx.clone());

        // Lấy danh sách tài khoản ban đầu (chỉ đọc file cục bộ, không gọi mạng)
        self.refresh_accounts();
        
        // Load danh sách file ban đầu
        self.refresh_active_pane().await;
        self.active_pane_left = false;
        self.refresh_active_pane().await;
        self.active_pane_left = true;

        // Thread đọc bàn phím
        let tx_key = tx.clone();
        std::thread::spawn(move || {
            loop {
                if crossterm::event::poll(std::time::Duration::from_millis(50)).unwrap() {
                    if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
                        let _ = tx_key.send(AppEvent::Key(key));
                    }
                }
            }
        });

        // Thread gửi nhịp đập màn hình (Tick)
        let tx_tick = tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let _ = tx_tick.send(AppEvent::Tick);
            }
        });

        loop {
            // Dựng giao diện vẽ
            terminal.draw(|f| {
                ui::layout::draw(self, f);
            })?;

            // Xử lý sự kiện từ hàng chờ
            if let Some(event) = rx.recv().await {
                match event {
                    AppEvent::Key(key) => {
                        let mut quit = false;
                        key_handlers::handle_key(self, key, &mut quit).await;
                        if quit {
                            break;
                        }
                    }
                    AppEvent::Tick => {
                        // Cập nhật logs của máy chủ WebDAV/S3 nếu đang chạy
                        // Ở đây chúng ta sẽ thực hiện đọc không chặn nếu cần
                    }
                    AppEvent::AsyncFinished(res) => {
                        self.is_loading = false;
                        match res {
                            Ok(()) => {
                                self.popup_state = PopupState::Message {
                                    title: "Đồng bộ thành công".to_string(),
                                    message: "Đồng bộ thư mục thành công!".to_string(),
                                };
                                // Refresh both panes to see the updated files
                                self.refresh_active_pane().await;
                                self.active_pane_left = !self.active_pane_left;
                                self.refresh_active_pane().await;
                                self.active_pane_left = !self.active_pane_left;
                            }
                            Err(e) => {
                                self.popup_state = PopupState::Message {
                                    title: "Lỗi đồng bộ".to_string(),
                                    message: e,
                                };
                            }
                        }
                    }
                    AppEvent::RemoteLoadFinished { is_left, result } => {
                        let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
                        pane.loading = false;
                        match result {
                            Ok(items) => {
                                pane.items = items;
                                if pane.selected_idx >= pane.items.len() {
                                    pane.selected_idx = 0;
                                }
                            }
                            Err(e) => {
                                self.popup_state = PopupState::Message {
                                    title: "Lỗi kết nối Cloud".to_string(),
                                    message: e,
                                };
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
