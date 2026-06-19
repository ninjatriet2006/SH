use crate::config::{AppEntry, AppStatus};
use crate::remover;
use crate::manager;
use std::collections::HashSet;
use ratatui::widgets::TableState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    AppList,
    AppOperations,
    UninstallList,
    AutostartManager,
}

pub struct App {
    pub current_screen: Screen,
    pub menu_index: usize,
    pub apps_with_status: Vec<(AppEntry, AppStatus)>,
    pub selected_index: usize,
    pub checked_app_ids: HashSet<String>,
    pub operations_index: usize,
    pub checked_operations: HashSet<usize>, // 0: Start, 1: Stop, 2: Restart, 3: Toggle Autostart
    pub status_message: Option<String>,
    pub process_snapshot: std::sync::Arc<std::sync::Mutex<manager::ProcessSnapshot>>,
    pub running_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[allow(dead_code)]
    pub worker_thread: Option<std::thread::JoinHandle<()>>,
    pub app_list_state: TableState,
    pub uninstall_list_state: TableState,
    pub autostart_entries: Vec<manager::AutostartEntry>,
    pub autostart_index: usize,
    pub autostart_state: TableState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterResult {
    None,
    RunWizard,
    RunCheckUpdates,
    RunCleanLeftovers,
    RunUninstalls,
    Exit,
}

impl App {
    pub fn new() -> Self {
        let initial_snapshot = std::sync::Arc::new(std::sync::Mutex::new(manager::ProcessSnapshot::collect()));
        let snapshot_clone = std::sync::Arc::clone(&initial_snapshot);
        let running_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let flag_clone = std::sync::Arc::clone(&running_flag);

        let worker_thread = std::thread::spawn(move || {
            while flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if !flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                let new_snapshot = manager::ProcessSnapshot::collect();
                if let Ok(mut guard) = snapshot_clone.lock() {
                    *guard = new_snapshot;
                }
            }
        });

        let mut app = App {
            current_screen: Screen::MainMenu,
            menu_index: 0,
            apps_with_status: Vec::new(),
            selected_index: 0,
            checked_app_ids: HashSet::new(),
            operations_index: 0,
            checked_operations: HashSet::new(),
            status_message: None,
            process_snapshot: initial_snapshot,
            worker_thread: Some(worker_thread),
            running_flag,
            app_list_state: TableState::default(),
            uninstall_list_state: TableState::default(),
            autostart_entries: Vec::new(),
            autostart_index: 0,
            autostart_state: TableState::default(),
        };
        app.reload_apps();
        app
    }

    pub fn get_process_snapshot(&self) -> manager::ProcessSnapshot {
        if let Ok(guard) = self.process_snapshot.lock() {
            guard.clone()
        } else {
            manager::ProcessSnapshot {
                canonical_exes: HashSet::new(),
                names: HashSet::new(),
                cmdlines: Vec::new(),
            }
        }
    }

    pub fn trigger_process_scan(&self) {
        let snapshot_clone = std::sync::Arc::clone(&self.process_snapshot);
        std::thread::spawn(move || {
            let new_snapshot = manager::ProcessSnapshot::collect();
            if let Ok(mut guard) = snapshot_clone.lock() {
                *guard = new_snapshot;
            }
        });
    }

    pub fn reload_apps(&mut self) {
        let system_apps = manager::scan_all_system_apps();
        
        // Scan status for each app
        self.apps_with_status = system_apps.into_iter()
            .map(|entry| {
                let status = entry.check_status();
                (entry, status)
            })
            .collect();
        
        // Adjust selected index if it exceeds list size
        if self.selected_index >= self.apps_with_status.len() && !self.apps_with_status.is_empty() {
            self.selected_index = self.apps_with_status.len() - 1;
        }
    }

    pub fn next(&mut self) {
        match self.current_screen {
            Screen::MainMenu => {
                self.menu_index = (self.menu_index + 1) % 7;
            }
            Screen::AppList | Screen::UninstallList => {
                if !self.apps_with_status.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.apps_with_status.len();
                }
            }
            Screen::AppOperations => {
                self.operations_index = (self.operations_index + 1) % 4;
            }
            Screen::AutostartManager => {
                if !self.autostart_entries.is_empty() {
                    self.autostart_index = (self.autostart_index + 1) % self.autostart_entries.len();
                }
            }
        }
        self.status_message = None;
    }

    pub fn previous(&mut self) {
        match self.current_screen {
            Screen::MainMenu => {
                if self.menu_index == 0 {
                    self.menu_index = 6;
                } else {
                    self.menu_index -= 1;
                }
            }
            Screen::AppList | Screen::UninstallList => {
                if !self.apps_with_status.is_empty() {
                    if self.selected_index == 0 {
                        self.selected_index = self.apps_with_status.len() - 1;
                    } else {
                        self.selected_index -= 1;
                    }
                }
            }
            Screen::AppOperations => {
                if self.operations_index == 0 {
                    self.operations_index = 3;
                } else {
                    self.operations_index -= 1;
                }
            }
            Screen::AutostartManager => {
                if !self.autostart_entries.is_empty() {
                    if self.autostart_index == 0 {
                        self.autostart_index = self.autostart_entries.len() - 1;
                    } else {
                        self.autostart_index -= 1;
                    }
                }
            }
        }
        self.status_message = None;
    }

    pub fn toggle_checked(&mut self) {
        match self.current_screen {
            Screen::AppList | Screen::UninstallList => {
                if let Some((entry, _)) = self.apps_with_status.get(self.selected_index) {
                    let id = entry.id.clone();
                    if self.checked_app_ids.contains(&id) {
                        self.checked_app_ids.remove(&id);
                    } else {
                        self.checked_app_ids.insert(id);
                    }
                }
            }
            Screen::AppOperations => {
                if self.checked_operations.contains(&self.operations_index) {
                    self.checked_operations.remove(&self.operations_index);
                } else {
                    self.checked_operations.insert(self.operations_index);
                }
            }
            _ => {}
        }
    }

    pub fn handle_enter(&mut self) -> EnterResult {
        match self.current_screen {
            Screen::MainMenu => {
                match self.menu_index {
                    0 => {
                        self.current_screen = Screen::AppList;
                        self.selected_index = 0;
                        self.checked_app_ids.clear();
                        EnterResult::None
                    }
                    1 => {
                        EnterResult::RunWizard
                    }
                    2 => {
                        self.current_screen = Screen::UninstallList;
                        self.selected_index = 0;
                        self.checked_app_ids.clear();
                        EnterResult::None
                    }
                    3 => {
                        EnterResult::RunCheckUpdates
                    }
                    4 => {
                        EnterResult::RunCleanLeftovers
                    }
                    5 => {
                        self.current_screen = Screen::AutostartManager;
                        self.autostart_entries = manager::scan_global_autostart();
                        self.autostart_index = 0;
                        EnterResult::None
                    }
                    6 => {
                        EnterResult::Exit
                    }
                    _ => EnterResult::None,
                }
            }
            Screen::AppList => {
                self.go_to_operations();
                EnterResult::None
            }
            Screen::AppOperations => {
                self.execute_operations();
                EnterResult::None
            }
            Screen::UninstallList => {
                EnterResult::RunUninstalls
            }
            Screen::AutostartManager => {
                self.delete_selected_autostart();
                EnterResult::None
            }
        }
    }

    pub fn delete_selected_autostart(&mut self) {
        if self.autostart_entries.is_empty() {
            return;
        }
        let entry = &self.autostart_entries[self.autostart_index];
        match manager::remove_autostart_entry(entry) {
            Ok(_) => {
                self.status_message = Some(format!("Đã gỡ bỏ '{}' khỏi khởi động cùng hệ thống!", entry.name));
                self.autostart_entries = manager::scan_global_autostart();
                if self.autostart_index >= self.autostart_entries.len() && !self.autostart_entries.is_empty() {
                    self.autostart_index = self.autostart_entries.len() - 1;
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Lỗi khi gỡ autostart: {}", e));
            }
        }
    }

    pub fn go_to_operations(&mut self) {
        if self.checked_app_ids.is_empty() {
            // Auto check currently highlighted app if none checked
            if let Some((entry, _)) = self.apps_with_status.get(self.selected_index) {
                self.checked_app_ids.insert(entry.id.clone());
            }
        }
        
        if !self.checked_app_ids.is_empty() {
            self.current_screen = Screen::AppOperations;
            self.operations_index = 0;
            self.checked_operations.clear();
            self.status_message = None;
        } else {
            self.status_message = Some("Danh sách ứng dụng trống!".to_string());
        }
    }

    pub fn handle_back(&mut self) {
        match self.current_screen {
            Screen::MainMenu => {}
            Screen::AppList | Screen::UninstallList | Screen::AutostartManager => {
                self.current_screen = Screen::MainMenu;
                self.status_message = None;
            }
            Screen::AppOperations => {
                self.current_screen = Screen::AppList;
                self.status_message = None;
            }
        }
    }

    pub fn execute_operations(&mut self) {
        if self.checked_operations.is_empty() {
            self.status_message = Some("Chưa chọn hành động nào để thực thi!".to_string());
            return;
        }

        let mut success_count = 0;
        let mut error_messages = Vec::new();

        // Get the list of selected apps
        let selected_apps: Vec<AppEntry> = self.apps_with_status.iter()
            .filter(|(entry, _)| self.checked_app_ids.contains(&entry.id))
            .map(|(entry, _)| entry.clone())
            .collect();

        for app in selected_apps {
            let mut app_actions = Vec::new();
            
            // 1. Stop
            if self.checked_operations.contains(&1) {
                app_actions.push(("Tắt", manager::stop_app(&app)));
            }
            // 2. Start (or Restart handles stop+start)
            if self.checked_operations.contains(&2) {
                app_actions.push(("Khởi động lại", manager::restart_app(&app)));
            } else if self.checked_operations.contains(&0) {
                app_actions.push(("Khởi chạy", manager::start_app(&app)));
            }
            // 3. Toggle Autostart
            if self.checked_operations.contains(&3) {
                let autostart_enabled = manager::is_autostart_enabled(&app);
                if autostart_enabled {
                    app_actions.push(("Tắt Autostart", manager::disable_autostart(&app)));
                } else {
                    app_actions.push(("Bật Autostart", manager::enable_autostart(&app)));
                }
            }

            let mut app_ok = true;
            for (action_name, result) in app_actions {
                if let Err(e) = result {
                    app_ok = false;
                    error_messages.push(format!("App {}: {} thất bại: {}", app.name, action_name, e));
                }
            }

            if app_ok {
                success_count += 1;
            }
        }

        if error_messages.is_empty() {
            self.status_message = Some(format!("Thực thi thành công trên {} ứng dụng!", success_count));
        } else {
            self.status_message = Some(format!(
                "Thành công {}/{} app. Lỗi đầu tiên: {}", 
                success_count, 
                self.checked_app_ids.len(),
                error_messages[0]
            ));
        }

        // Return to AppList
        self.current_screen = Screen::AppList;
        self.checked_app_ids.clear();
        self.trigger_process_scan();
        self.reload_apps();
    }

    pub fn execute_uninstalls(&mut self) {
        if self.checked_app_ids.is_empty() {
            // Auto check currently highlighted if none checked
            if let Some((entry, _)) = self.apps_with_status.get(self.selected_index) {
                self.checked_app_ids.insert(entry.id.clone());
            }
        }

        if self.checked_app_ids.is_empty() {
            self.status_message = Some("Chưa chọn ứng dụng nào để gỡ!".to_string());
            return;
        }

        let mut success_count = 0;
        let mut error_messages = Vec::new();

        let ids_to_uninstall: Vec<String> = self.checked_app_ids.iter().cloned().collect();
        for id in ids_to_uninstall {
            match remover::uninstall(&id) {
                Ok(_) => success_count += 1,
                Err(e) => error_messages.push(format!("Lỗi gỡ app ID {}: {}", id, e)),
            }
        }

        if error_messages.is_empty() {
            self.status_message = Some(format!("Đã gỡ cài đặt thành công {} ứng dụng!", success_count));
        } else {
            self.status_message = Some(format!(
                "Gỡ thành công {}/{} ứng dụng. Lỗi: {}", 
                success_count, 
                self.checked_app_ids.len(),
                error_messages[0]
            ));
        }

        self.current_screen = Screen::MainMenu;
        self.checked_app_ids.clear();
        self.trigger_process_scan();
        self.reload_apps();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.running_flag.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Dispatches calls to the interactive wizard located in src/wizard.rs.
pub fn run_integration_wizard_inline() -> Result<bool, String> {
    crate::wizard::run_wizard(None)
}
