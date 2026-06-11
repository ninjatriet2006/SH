pub mod operations;
pub mod key_handlers;

use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use crossterm::event::KeyEvent;
use serde::{Serialize, Deserialize};

use crate::ui;
use operations::{FileItem, Operations};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredAccount {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub accounts: Vec<StoredAccount>,
}

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
    LoginInput { email_buffer: String, pass_buffer: String, keep_logged: String, active_field: usize, error_msg: Option<String> }, // 0: email, 1: password, 2: keep_logged
    TwoFAInput { email: String, password: String, keep_logged: String, twofa_buffer: String }, // bước 2: nhập mã 2FA
    ConfirmDelete { name: String },
    #[allow(dead_code)]
    ConfirmEmptyTrash,
    SpecialActionsMenu { selected_idx: usize },
    ViewFile { name: String, content: Vec<String>, scroll: usize },
    Message { title: String, message: String },
    SwitchAccountMenu { selected_idx: usize },
    QuickLoginSelect { options: Vec<StoredAccount>, selected_idx: usize },
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
    pub error: Option<String>,
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
            error: None,
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
    LoginFinished { email: String, password: String, keep_logged: String, result: Result<(), String> },
    AccountsRefreshed { accounts: Vec<String>, default_email: Option<String> },
    StorageInfoRefreshed { used: String, max: String },
    LoginLog(String),
    ExportFinished { is_api_key: bool, result: Result<String, String> },
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
    pub default_email: Option<String>,
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
    pub login_logs: Vec<String>,
    
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
            accounts: Vec::new(),
            active_account: None,
            default_email: None,
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
            login_logs: Vec::new(),
            
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
}


pub fn get_default_data_dir() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        let dot_filen = home.join(".filen-cli");
        if dot_filen.is_dir() {
            Some(dot_filen)
        } else {
            Some(home.join(".config/filen-cli"))
        }
    } else {
        None
    }
}

pub(crate) fn load_stored_accounts() -> Vec<StoredAccount> {
    if let Some(home) = dirs::home_dir() {
        let file_path = home.join(".config/filen-cli/accounts.yaml");
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(config) = serde_yaml::from_str::<AccountConfig>(&content) {
                    return config.accounts;
                }
            }
        }
    }
    Vec::new()
}

pub(crate) fn save_stored_accounts(accounts: &[StoredAccount]) {
    if let Some(home) = dirs::home_dir() {
        let config_dir = home.join(".config/filen-cli");
        let _ = std::fs::create_dir_all(&config_dir);
        let file_path = config_dir.join("accounts.yaml");
        let config = AccountConfig {
            accounts: accounts.to_vec(),
        };
        if let Ok(content) = serde_yaml::to_string(&config) {
            if let Ok(_) = std::fs::write(&file_path, content) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o600));
                }
            }
        }
    }
}

impl App {


    // Làm mới danh sách tài khoản đã lưu
    pub async fn refresh_accounts(&mut self) {
        let mut loaded_accounts = Vec::new();

        // 1. Load tất cả các tài khoản lưu trong accounts.yaml
        let stored = load_stored_accounts();
        for acc in &stored {
            if !loaded_accounts.contains(&acc.email) {
                loaded_accounts.push(acc.email.clone());
            }
        }

        self.accounts = loaded_accounts;

        // 2. Kiểm tra xem tài khoản nào đang thực sự đăng nhập trên CLI mặc định
        let default_res = Operations::whoami(&None).await;
        if let Ok(email) = default_res {
            let email_clean = email.trim().to_string();
            if !email_clean.is_empty() 
                && !email_clean.contains("Please enter") 
                && !email_clean.contains("credentials") 
                && email_clean != "anonymous@filen.io" 
            {
                self.active_account = Some(email_clean);
            } else {
                self.active_account = None;
            }
        } else {
            self.active_account = None;
        }

        // 3. Điều chỉnh con trỏ index tài khoản active
        if let Some(ref active) = self.active_account {
            if let Some(pos) = self.accounts.iter().position(|a| a == active) {
                self.active_account_idx = pos;
            } else {
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
                    pane.error = None;
                    if pane.selected_idx >= pane.items.len() {
                        pane.selected_idx = 0;
                    }
                }
                Err(e) => {
                    let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
                    pane.items = Vec::new();
                    pane.error = Some(e);
                }
            }
            let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
            pane.loading = false;
        } else {
            let pane = if is_left { &mut self.left_pane } else { &mut self.right_pane };
            pane.error = None;
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

    // Làm mới thông số bộ nhớ đám mây (bất đồng bộ)
    pub fn trigger_refresh_storage_info(&mut self) {
        if self.active_account.is_none() {
            self.storage_used = "0 B".to_string();
            self.storage_max = "0 B".to_string();
            return;
        }
        let tx = self.msg_tx.clone();
        let active_account = self.active_account.clone();
        tokio::spawn(async move {
            if let Ok((used, max)) = Operations::statfs(&active_account).await {
                if let Some(tx) = tx {
                    let _ = tx.send(AppEvent::StorageInfoRefreshed { used, max });
                }
            }
        });
    }

    // Thực chạy vòng lặp TUI chính
    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.msg_tx = Some(tx.clone());

        // Khởi chạy tác vụ nền để load danh sách tài khoản ban đầu (không chặn startup)
        let tx_accounts = tx.clone();
        tokio::spawn(async move {
            let mut loaded_accounts = Vec::new();
            let mut default_email = None;

            // Load tất cả các tài khoản lưu trong accounts.yaml
            let stored = load_stored_accounts();
            for acc in &stored {
                if !loaded_accounts.contains(&acc.email) {
                    loaded_accounts.push(acc.email.clone());
                }
            }

            // Kiểm tra xem tài khoản nào đang thực sự đăng nhập trên CLI mặc định
            let default_res = Operations::whoami(&None).await;
            if let Ok(email) = default_res {
                let email_clean = email.trim().to_string();
                if !email_clean.is_empty() 
                    && !email_clean.contains("Please enter") 
                    && !email_clean.contains("credentials") 
                    && email_clean != "anonymous@filen.io" 
                {
                    default_email = Some(email_clean);
                }
            }

            let _ = tx_accounts.send(AppEvent::AccountsRefreshed {
                accounts: loaded_accounts,
                default_email,
            });
        });

        // Nạp danh sách file local ban đầu (chạy ngay lập tức vì không gọi mạng)
        if self.left_pane.is_local {
            if let Ok(items) = Operations::list_local(&self.left_pane.path) {
                self.left_pane.items = items;
            }
        }
        if self.right_pane.is_local {
            if let Ok(items) = Operations::list_local(&self.right_pane.path) {
                self.right_pane.items = items;
            }
        } else {
            self.right_pane.loading = true; // Hiện trạng thái đang tải Cloud khi mới khởi động
        }

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
                                pane.error = None;
                                if pane.selected_idx >= pane.items.len() {
                                    pane.selected_idx = 0;
                                }
                            }
                            Err(e) => {
                                pane.items = Vec::new();
                                pane.error = Some(e);
                            }
                        }
                    }
                    AppEvent::AccountsRefreshed { accounts, default_email } => {
                        self.accounts = accounts;
                        self.default_email = default_email.clone();
                        self.active_account = default_email;

                        if let Some(ref active) = self.active_account {
                            if let Some(pos) = self.accounts.iter().position(|a| a == active) {
                                self.active_account_idx = pos;
                            } else {
                                self.active_account_idx = 0;
                            }
                        } else {
                            self.active_account_idx = 0;
                        }

                        // Sau khi nạp xong danh sách tài khoản, chúng ta mới bắt đầu load file Cloud!
                        self.trigger_refresh_storage_info();
                        self.refresh_active_pane().await;
                        self.active_pane_left = false;
                        self.refresh_active_pane().await;
                        self.active_pane_left = true;
                    }
                    AppEvent::StorageInfoRefreshed { used, max } => {
                        self.storage_used = used;
                        self.storage_max = max;
                    }
                    AppEvent::LoginFinished { email, password, keep_logged, result } => {
                        self.is_loading = false;
                        match result {
                            Ok(()) => {
                                // Lưu/Cập nhật vào tệp accounts.yaml để phục vụ Quick Login / chuyển tài khoản sau này
                                if keep_logged.trim().to_lowercase() == "y" {
                                    let mut stored = load_stored_accounts();
                                    if let Some(pos) = stored.iter().position(|acc| acc.email == email) {
                                        stored[pos].password = password.clone();
                                    } else {
                                        stored.push(StoredAccount {
                                            email: email.clone(),
                                            password: password.clone(),
                                        });
                                    }
                                    save_stored_accounts(&stored);
                                }

                                self.active_account = Some(email.clone());
                                self.refresh_accounts().await;
                                self.trigger_refresh_storage_info();
                                self.refresh_active_pane().await;
                                self.active_pane_left = false;
                                self.refresh_active_pane().await;
                                self.active_pane_left = true;

                                self.popup_state = PopupState::Message {
                                    title: "Đăng nhập thành công".to_string(),
                                    message: format!("Tài khoản {} đã được nạp và kích hoạt thành công trên TUI.", email),
                                };
                            }
                            Err(e) => {
                                let err_lower = e.to_lowercase();
                                if err_lower.contains("twofactorcode") || err_lower.contains("2fa") || err_lower.contains("two_factor") || err_lower.contains("xác thực") {
                                    self.popup_state = PopupState::TwoFAInput {
                                        email,
                                        password,
                                        keep_logged,
                                        twofa_buffer: String::new(),
                                    };
                                } else {
                                    self.popup_state = PopupState::LoginInput {
                                        email_buffer: email,
                                        pass_buffer: password,
                                        keep_logged,
                                        active_field: 1, // Di chuyển tiêu điểm lại ô Password
                                        error_msg: Some(e),
                                    };
                                }
                            }
                        }
                    }
                    AppEvent::LoginLog(msg) => {
                        self.login_logs.push(msg);
                    }
                    AppEvent::ExportFinished { is_api_key, result } => {
                        self.is_loading = false;
                        match result {
                            Ok(text) => {
                                let mut lines = Vec::new();
                                if is_api_key {
                                    let mut display_text = text.trim().to_string();
                                    if let Some(pos) = display_text.find("API Key for") {
                                        display_text = display_text[pos..].to_string();
                                    }
                                    lines.push(display_text.clone());
                                    lines.push("".to_string());

                                    // Extract hex key
                                    let parsed_key = if let Some(pos) = text.find("API Key for") {
                                        let slice = &text[pos..];
                                        if let Some(colon_pos) = slice.rfind(':') {
                                            slice[colon_pos + 1..].trim().to_string()
                                        } else {
                                            slice.to_string()
                                        }
                                    } else {
                                        text.trim().to_string()
                                    };

                                    match operations::Operations::copy_to_clipboard(&parsed_key) {
                                        Ok(_) => {
                                            lines.push("✨ Đã tự động sao chép API Key vào clipboard hệ thống!".to_string());
                                        }
                                        Err(e) => {
                                            lines.push(format!("⚠️ Lỗi sao chép clipboard: {}", e));
                                        }
                                    }
                                    lines.push("".to_string());
                                    lines.push("Bấm [C] để sao chép lại API Key vào clipboard.".to_string());

                                    self.popup_state = PopupState::ViewFile {
                                        name: "API Key cho Rclone".to_string(),
                                        content: lines,
                                        scroll: 0,
                                    };
                                } else {
                                    // Auth Config
                                    for line in text.lines() {
                                        lines.push(line.to_string());
                                    }
                                    lines.push("".to_string());

                                    match operations::Operations::copy_to_clipboard(&text) {
                                        Ok(_) => {
                                            lines.push("✨ Đã tự động sao chép Auth Config vào clipboard hệ thống!".to_string());
                                        }
                                        Err(e) => {
                                            lines.push(format!("⚠️ Lỗi sao chép clipboard: {}", e));
                                        }
                                    }
                                    lines.push("".to_string());
                                    lines.push("Bấm [C] để sao chép lại Auth Config vào clipboard.".to_string());

                                    self.popup_state = PopupState::ViewFile {
                                        name: "Cấu hình đăng nhập (Auth Config)".to_string(),
                                        content: lines,
                                        scroll: 0,
                                    };
                                }
                            }
                            Err(e) => {
                                self.popup_state = PopupState::Message {
                                    title: if is_api_key { "Lỗi xuất API Key".to_string() } else { "Lỗi xuất cấu hình".to_string() },
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
