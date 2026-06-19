use std::collections::HashMap;
use crate::config::{OpencodeConfig, AuthConfig, Provider, ProviderOptions, ModelEntry, ModelLimit, ModelModalities};
use crate::api::{ApiClient, ApiStatus};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Main,
    AddProvider,
    EditProvider,
    ManageAuthKeys,
    ModelScanResult,
    QuickClean,
    Confirmation,
    SelectPreset,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DynamicPreset {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub id_prefix: String,
    pub npm: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderForm {
    pub id: String,
    pub preset_id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub focus_index: usize, // 0: Preset, 1: Name, 2: URL, 3: Key, 4: Test, 5: Save, 6: Cancel
    pub test_status: Option<ApiStatus>,
    pub is_testing: bool,
    pub is_editing_field: bool,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteProvider(String),
    DeleteAuthKey(String),
    CleanSelected,
    OverwriteDuplicate { duplicate_id: String, duplicate_name: String },
}

pub enum AppMessage {
    Test { provider_id: String, status: ApiStatus },
    Scan { provider_id: String, models: Result<Vec<String>, String> },
    FormTest { status: ApiStatus },
}

pub struct App {
    pub config: OpencodeConfig,
    pub auth_config: AuthConfig,
    pub providers_keys: Vec<String>,
    pub selected_provider_idx: usize,
    pub current_screen: Screen,
    
    // Forms & states
    pub form: ProviderForm,
    pub confirm_action: Option<ConfirmAction>,
    pub confirm_focus_yes: bool, // true: Yes, false: No
    
    // Status caching
    pub api_status_cache: HashMap<String, Option<ApiStatus>>,
    pub is_checking_all: bool,
    pub is_scanning: bool,
    
    // Models selection state
    pub scanned_models: Vec<(String, bool)>, // (model_id, is_checked)
    pub selected_model_idx: usize,
    pub scanning_provider_id: String,
    pub model_search_query: String,
    
    // Auth keys manager
    pub auth_keys: Vec<(String, String)>, // (key_name, key_value)
    pub selected_auth_idx: usize,
    
    // Quick clean list
    pub clean_list: Vec<(String, String, ApiStatus, bool)>, // (provider_id, name, status, is_checked)
    pub selected_clean_idx: usize,
    
    // Presets
    pub presets: Vec<DynamicPreset>,
    pub preset_search_query: String,
    pub selected_preset_search_idx: usize,

    // ListStates for scroll viewports
    pub provider_list_state: ratatui::widgets::ListState,
    pub preset_list_state: ratatui::widgets::ListState,
    pub models_list_state: ratatui::widgets::ListState,
    pub auth_keys_list_state: ratatui::widgets::ListState,
    pub clean_list_state: ratatui::widgets::ListState,
    
    // Message sender
    pub tx: tokio::sync::mpsc::UnboundedSender<AppMessage>,
    pub logs: Vec<String>,
}

impl App {
    pub fn new(
        config: OpencodeConfig, 
        auth_config: AuthConfig,
        tx: tokio::sync::mpsc::UnboundedSender<AppMessage>
    ) -> Self {
        let mut keys: Vec<String> = config.provider.keys().cloned().collect();
        keys.sort();

        let mut api_status_cache = HashMap::new();
        for key in &keys {
            api_status_cache.insert(key.clone(), None);
        }

        let presets = Self::load_dynamic_presets();
        let default_preset = presets.iter()
            .find(|p| p.id == "xiaomi-token-plan-sgp" || p.id_prefix == "mimo_sgp")
            .cloned()
            .unwrap_or_else(|| presets[0].clone());

        let mut provider_list_state = ratatui::widgets::ListState::default();
        provider_list_state.select(Some(0));

        let mut preset_list_state = ratatui::widgets::ListState::default();
        preset_list_state.select(Some(0));

        let mut models_list_state = ratatui::widgets::ListState::default();
        models_list_state.select(Some(0));

        let mut auth_keys_list_state = ratatui::widgets::ListState::default();
        auth_keys_list_state.select(Some(0));

        let mut clean_list_state = ratatui::widgets::ListState::default();
        clean_list_state.select(Some(0));

        let mut app = App {
            config,
            auth_config,
            providers_keys: keys,
            selected_provider_idx: 0,
            current_screen: Screen::Main,
            form: ProviderForm {
                id: String::new(),
                preset_id: default_preset.id.clone(),
                name: default_preset.name.clone(),
                base_url: default_preset.base_url.clone(),
                api_key: String::new(),
                focus_index: 0,
                test_status: None,
                is_testing: false,
                is_editing_field: false,
            },
            confirm_action: None,
            confirm_focus_yes: false,
            api_status_cache,
            is_checking_all: false,
            is_scanning: false,
            scanned_models: Vec::new(),
            selected_model_idx: 0,
            scanning_provider_id: String::new(),
            model_search_query: String::new(),
            auth_keys: Vec::new(),
            selected_auth_idx: 0,
            clean_list: Vec::new(),
            selected_clean_idx: 0,
            presets,
            preset_search_query: String::new(),
            selected_preset_search_idx: 0,
            provider_list_state,
            preset_list_state,
            models_list_state,
            auth_keys_list_state,
            clean_list_state,
            tx,
            logs: vec!["Sẵn sàng.".to_string()],
        };
        app.merge_auth_into_providers();
        app.update_provider_keys();
        app.reload_auth_keys();
        app
    }

    pub fn load_dynamic_presets() -> Vec<DynamicPreset> {
        let mut presets = Vec::new();
        
        // 1. Cố gắng đọc từ ~/.cache/opencode/models.json
        if let Some(home) = crate::config::get_home_dir() {
            let path = home.join(".cache").join("opencode").join("models.json");
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    #[derive(Deserialize)]
                    struct RawProvider {
                        id: String,
                        name: String,
                        api: Option<String>,
                        npm: Option<String>,
                    }
                    if let Ok(map) = serde_json::from_str::<HashMap<String, RawProvider>>(&content) {
                        for (_, raw) in map {
                            let base_url = raw.api.unwrap_or_default();
                            presets.push(DynamicPreset {
                                id: raw.id.clone(),
                                name: raw.name.clone(),
                                base_url,
                                id_prefix: raw.id.replace("-", "_").replace(" ", "_").to_lowercase(),
                                npm: raw.npm,
                            });
                        }
                    }
                }
            }
        }

        // Sắp xếp danh sách preset theo tên
        presets.sort_by_key(|a| a.name.to_lowercase());

        // 2. Nếu danh sách rỗng (do lỗi đọc file hoặc chưa có cache), thêm các fallbacks mặc định
        if presets.is_empty() {
            presets = vec![
                DynamicPreset {
                    id: "xiaomi-token-plan-sgp".to_string(),
                    name: "MiMo SGP".to_string(),
                    base_url: "https://token-plan-sgp.xiaomimimo.com/v1".to_string(),
                    id_prefix: "mimo_sgp".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "mimo-gateway".to_string(),
                    name: "MiMo Gateway".to_string(),
                    base_url: "https://opengateway.gitlawb.com/v1".to_string(),
                    id_prefix: "mimo_gate".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "opencode-zen".to_string(),
                    name: "OpenCode Zen".to_string(),
                    base_url: "https://api.opencode.ai/v1".to_string(),
                    id_prefix: "opencode_zen".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "openai".to_string(),
                    name: "OpenAI".to_string(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    id_prefix: "openai".to_string(),
                    npm: Some("@ai-sdk/openai".to_string()),
                },
                DynamicPreset {
                    id: "anthropic".to_string(),
                    name: "Anthropic".to_string(),
                    base_url: "https://api.anthropic.com/v1".to_string(),
                    id_prefix: "anthropic".to_string(),
                    npm: Some("@ai-sdk/anthropic".to_string()),
                },
                DynamicPreset {
                    id: "google".to_string(),
                    name: "Google Gemini".to_string(),
                    base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
                    id_prefix: "google".to_string(),
                    npm: Some("@ai-sdk/google".to_string()),
                },
                DynamicPreset {
                    id: "deepseek".to_string(),
                    name: "DeepSeek".to_string(),
                    base_url: "https://api.deepseek.com/v1".to_string(),
                    id_prefix: "deepseek".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "openrouter".to_string(),
                    name: "OpenRouter".to_string(),
                    base_url: "https://openrouter.ai/api/v1".to_string(),
                    id_prefix: "openrouter".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "groq".to_string(),
                    name: "Groq".to_string(),
                    base_url: "https://api.groq.com/openai/v1".to_string(),
                    id_prefix: "groq".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "ollama".to_string(),
                    name: "Ollama".to_string(),
                    base_url: "http://localhost:11434/v1".to_string(),
                    id_prefix: "ollama".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
                DynamicPreset {
                    id: "lmstudio".to_string(),
                    name: "LM Studio".to_string(),
                    base_url: "http://localhost:1234/v1".to_string(),
                    id_prefix: "lmstudio".to_string(),
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                },
            ];
        }

        // Luôn luôn chèn thêm "Custom" ở cuối cùng
        presets.push(DynamicPreset {
            id: "custom".to_string(),
            name: "Tự nhập (Custom)".to_string(),
            base_url: "".to_string(),
            id_prefix: "custom".to_string(),
            npm: None,
        });

        presets
    }

    pub fn filtered_presets(&self) -> Vec<&DynamicPreset> {
        let query = self.preset_search_query.to_lowercase();
        self.presets.iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query) || 
                p.id.to_lowercase().contains(&query) ||
                p.base_url.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn filtered_scanned_models(&self) -> Vec<(usize, String, bool)> {
        let query = self.model_search_query.to_lowercase();
        self.scanned_models.iter().enumerate()
            .filter(|(_, (name, _))| {
                name.to_lowercase().contains(&query)
            })
            .map(|(idx, (name, checked))| (idx, name.clone(), *checked))
            .collect()
    }

    pub fn detect_duplicate(&self) -> Option<(String, String)> {
        let id = self.form.id.trim();
        let base_url = self.form.base_url.trim();
        let api_key = self.form.api_key.trim();

        if base_url.is_empty() || api_key.is_empty() {
            return None;
        }

        let clean_new_url = base_url.trim_end_matches('/');

        // 1. Kiểm tra trùng lặp trong in-memory provider (đã gộp cả auth.json và opencode.json)
        for (prov_id, prov) in &self.config.provider {
            // Nếu ở chế độ Edit, bỏ qua so sánh với chính nó
            if (self.current_screen == Screen::EditProvider || !id.is_empty()) && prov_id == id {
                continue;
            }
            let clean_prov_url = prov.options.base_url.trim().trim_end_matches('/');
            if clean_prov_url == clean_new_url && prov.options.api_key.trim() == api_key.trim() {
                return Some((prov.name.clone(), prov_id.clone()));
            }
        }

        // 2. Kiểm tra trong auth_config gốc để chắc chắn
        for (auth_id, auth_entry) in &self.auth_config {
            if (self.current_screen == Screen::EditProvider || !id.is_empty()) && auth_id == id {
                continue;
            }
            if let Some(preset) = self.presets.iter().find(|p| p.id == *auth_id) {
                let clean_preset_url = preset.base_url.trim().trim_end_matches('/');
                if clean_preset_url == clean_new_url && auth_entry.key.trim() == api_key.trim() {
                    return Some((preset.name.clone(), auth_id.clone()));
                }
            }
        }

        None
    }

    pub fn execute_overwrite_duplicate(&mut self, duplicate_id: &str) -> Result<(), String> {
        let name = self.form.name.trim().to_string();
        let base_url = self.form.base_url.trim().to_string();
        let api_key = self.form.api_key.trim().to_string();

        // 1. Tìm npm từ preset nếu có
        let selected_preset = self.presets.iter().find(|p| p.id == self.form.preset_id);
        let npm = selected_preset.and_then(|p| p.npm.clone()).or(Some("@ai-sdk/openai-compatible".to_string()));

        // 2. Nếu đang sửa một provider khác (ví dụ: sửa B thành trùng với A)
        // và quyết định gộp vào A, thì ta phải xoá B đi để tránh trùng lặp
        let editing_id = self.form.id.trim().to_string();
        if !editing_id.is_empty() && editing_id != duplicate_id {
            self.config.provider.remove(&editing_id);
            self.log(format!("Xoá provider cũ đang sửa để tránh trùng lặp: {}", editing_id));
        }

        // 3. Cập nhật hoặc thêm provider trùng cũ (duplicate_id) bằng thông tin mới
        if let Some(provider) = self.config.provider.get_mut(duplicate_id) {
            provider.name = name;
            provider.options.base_url = base_url;
            provider.options.api_key = api_key;
            provider.npm = npm;
            // Giữ nguyên các models đã có của provider cũ
        } else {
            self.config.provider.insert(duplicate_id.to_string(), Provider {
                npm,
                name,
                options: ProviderOptions {
                    base_url,
                    api_key,
                },
                models: HashMap::new(),
            });
        }

        // 4. Lưu cấu hình cả hai file
        self.save_all_config()?;
        self.log(format!("Đã gộp/ghi đè cấu hình trùng lặp vào Provider: {}", duplicate_id));
        self.update_provider_keys();

        // 5. Thử check lại trạng thái ngầm ngay
        let tx = self.tx.clone();
        if let Some(p) = self.config.provider.get(duplicate_id) {
            let base_url_check = p.options.base_url.clone();
            let api_key_check = p.options.api_key.clone();
            let provider_id = duplicate_id.to_string();
            tokio::spawn(async move {
                let client = ApiClient::new();
                let status = client.test_api(&base_url_check, &api_key_check).await;
                let _ = tx.send(AppMessage::Test { provider_id, status });
            });
        }

        self.current_screen = Screen::Main;
        Ok(())
    }

    pub fn select_preset(&mut self, preset: DynamicPreset) {
        self.form.preset_id = preset.id.clone();
        if preset.id != "custom" {
            self.form.name = preset.name.clone();
            self.form.base_url = preset.base_url.clone();
        } else {
            self.form.name = "Custom Provider".to_string();
            self.form.base_url = String::new();
        }
        self.form.test_status = None;
        
        // Quay lại màn hình tương ứng
        self.current_screen = if self.form.id.is_empty() {
            Screen::AddProvider
        } else {
            Screen::EditProvider
        };
        // Focus Name
        self.form.focus_index = 1;
        self.log(format!("Đã chọn preset: {}", preset.name));
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(format!("[{}] {}", time, msg.into()));
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn reload_auth_keys(&mut self) {
        let mut keys: Vec<(String, String)> = self.auth_config.iter().map(|(k, v)| (k.clone(), v.key.clone())).collect();
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        self.auth_keys = keys;
        if self.selected_auth_idx >= self.auth_keys.len() && !self.auth_keys.is_empty() {
            self.selected_auth_idx = self.auth_keys.len() - 1;
        }
    }

    pub fn merge_auth_into_providers(&mut self) {
        let mut added = Vec::new();
        for (auth_id, auth_entry) in &self.auth_config {
            if auth_entry.auth_type != "api" || auth_entry.key.is_empty() {
                continue;
            }
            
            if let Some(preset) = self.presets.iter().find(|p| p.id == *auth_id) {
                let should_insert = match self.config.provider.get_mut(auth_id) {
                    Some(prov) => {
                        if prov.options.api_key.is_empty() {
                            prov.options.api_key = auth_entry.key.clone();
                        }
                        false
                    }
                    None => true,
                };
                
                if should_insert {
                    let npm = preset.npm.clone().or(Some("@ai-sdk/openai-compatible".to_string()));
                    self.config.provider.insert(auth_id.clone(), Provider {
                        npm,
                        name: preset.name.clone(),
                        options: ProviderOptions {
                            base_url: preset.base_url.clone(),
                            api_key: auth_entry.key.clone(),
                        },
                        models: HashMap::new(),
                    });
                    added.push(preset.name.clone());
                }
            }
        }
        if !added.is_empty() {
            self.log(format!("Nạp {} providers từ auth.json: {:?}", added.len(), added));
        }
    }

    pub fn reload_all_config_from_disk(&mut self) {
        let opencode_res = OpencodeConfig::load();
        let auth_res = crate::config::AuthEntry::load_config();
        
        match (opencode_res, auth_res) {
            (Ok(cfg), Ok(auth)) => {
                self.config = cfg;
                self.auth_config = auth;
                self.merge_auth_into_providers();
                self.reload_auth_keys();
                self.update_provider_keys();
            }
            (Err(e), _) | (_, Err(e)) => {
                self.log(format!("Không thể reload cấu hình: {}", e));
            }
        }
    }

    pub fn save_all_config(&mut self) -> Result<(), String> {
        // 1. Đồng bộ ngược từ memory vào self.auth_config
        for (id, provider) in &self.config.provider {
            if let Some(preset) = self.presets.iter().find(|p| p.id == *id) {
                let clean_prov_url = provider.options.base_url.trim().trim_end_matches('/');
                let clean_preset_url = preset.base_url.trim().trim_end_matches('/');
                if clean_prov_url == clean_preset_url {
                    self.auth_config.insert(id.clone(), crate::config::AuthEntry {
                        auth_type: "api".to_string(),
                        key: provider.options.api_key.clone(),
                    });
                }
            }
        }
        
        // Dọn dẹp auth_config: nếu key trong auth_config không còn trong config.provider
        let mut keys_to_remove = Vec::new();
        for auth_id in self.auth_config.keys() {
            if !self.config.provider.contains_key(auth_id) {
                keys_to_remove.push(auth_id.clone());
            }
        }
        for k in keys_to_remove {
            self.auth_config.remove(&k);
        }

        // 2. Tạo bản sao lọc để lưu vào opencode.json
        let mut file_config = self.config.clone();
        file_config.provider.retain(|id, provider| {
            let is_builtin = self.presets.iter().any(|preset| {
                let clean_prov_url = provider.options.base_url.trim().trim_end_matches('/');
                let clean_preset_url = preset.base_url.trim().trim_end_matches('/');
                preset.id == *id && clean_prov_url == clean_preset_url
            });
            !is_builtin
        });

        // 3. Lưu cả hai file
        file_config.save()?;
        crate::config::AuthEntry::save_config(&self.auth_config)?;
        
        self.reload_auth_keys();

        Ok(())
    }

    pub fn sync_providers_from_auth(&mut self, silent: bool) {
        let old_count = self.config.provider.len();
        self.reload_all_config_from_disk();
        let new_count = self.config.provider.len();
        
        if new_count > old_count {
            self.log(format!("Đã đồng bộ: Nhập mới {} providers từ auth.json.", new_count - old_count));
            self.check_all_providers();
        } else if !silent {
            self.log("Không tìm thấy API mới nào trong auth.json cần đồng bộ.");
        }
    }

    pub fn update_provider_keys(&mut self) {
        let mut keys: Vec<String> = self.config.provider.keys().cloned().collect();
        keys.sort();
        self.providers_keys = keys;
        
        // Cập nhật cache
        let mut new_cache = HashMap::new();
        for key in &self.providers_keys {
            new_cache.insert(
                key.clone(), 
                self.api_status_cache.get(key).cloned().flatten()
            );
        }
        self.api_status_cache = new_cache;

        if self.selected_provider_idx >= self.providers_keys.len() && !self.providers_keys.is_empty() {
            self.selected_provider_idx = self.providers_keys.len() - 1;
        }

        self.provider_list_state = ratatui::widgets::ListState::default();
    }

    pub fn selected_provider_id(&self) -> Option<&String> {
        self.providers_keys.get(self.selected_provider_idx)
    }

    pub fn check_selected_provider(&mut self) {
        if let Some(id) = self.selected_provider_id().cloned() {
            if let Some(provider) = self.config.provider.get(&id) {
                let tx = self.tx.clone();
                let provider_id = id.clone();
                let base_url = provider.options.base_url.clone();
                let api_key = provider.options.api_key.clone();

                self.api_status_cache.insert(id.clone(), None);
                self.log(format!("Bắt đầu kiểm tra kết nối cho: {}", provider.name));

                tokio::spawn(async move {
                    let client = ApiClient::new();
                    let status = client.test_api(&base_url, &api_key).await;
                    let _ = tx.send(AppMessage::Test { provider_id, status });
                });
            }
        }
    }

    pub fn check_all_providers(&mut self) {
        self.is_checking_all = true;
        self.log("Kiểm tra kết nối của tất cả các API...");
        for id in &self.providers_keys {
            if let Some(provider) = self.config.provider.get(id) {
                let tx = self.tx.clone();
                let provider_id = id.clone();
                let base_url = provider.options.base_url.clone();
                let api_key = provider.options.api_key.clone();

                self.api_status_cache.insert(id.clone(), None);

                tokio::spawn(async move {
                    let client = ApiClient::new();
                    let status = client.test_api(&base_url, &api_key).await;
                    let _ = tx.send(AppMessage::Test { provider_id, status });
                });
            }
        }
    }

    pub fn scan_models_selected(&mut self) {
        if let Some(id) = self.selected_provider_id().cloned() {
            if let Some(provider) = self.config.provider.get(&id) {
                let tx = self.tx.clone();
                let provider_id = id.clone();
                let base_url = provider.options.base_url.clone();
                let api_key = provider.options.api_key.clone();

                self.is_scanning = true;
                self.scanning_provider_id = id.clone();
                self.log(format!("Quét danh sách mô hình từ: {}", provider.name));

                tokio::spawn(async move {
                    let client = ApiClient::new();
                    let models = client.fetch_models(&base_url, &api_key).await;
                    let _ = tx.send(AppMessage::Scan { provider_id, models });
                });
            }
        }
    }

    pub fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::Test { provider_id, status } => {
                self.api_status_cache.insert(provider_id.clone(), Some(status.clone()));
                if let Some(p) = self.config.provider.get(&provider_id) {
                    self.log(format!("Kiểm tra {}: {}", p.name, status));
                }
                
                // Kiểm tra xem đã hoàn thành toàn bộ Check All chưa
                if self.is_checking_all {
                    let all_done = self.providers_keys.iter().all(|k| self.api_status_cache.get(k).unwrap_or(&None).is_some());
                    if all_done {
                        self.is_checking_all = false;
                        self.log("Hoàn thành kiểm tra tất cả.");
                    }
                }
            }
            AppMessage::Scan { provider_id, models } => {
                if self.is_scanning && self.scanning_provider_id == provider_id {
                    self.is_scanning = false;
                    match models {
                        Ok(list) => {
                            self.log(format!("Quét thành công {} mô hình từ: {}", list.len(), provider_id));
                            
                            // Lấy danh sách models hiện tại của provider để so sánh
                            let existing_models = if let Some(p) = self.config.provider.get(&provider_id) {
                                p.models.keys().cloned().collect::<Vec<String>>()
                            } else {
                                Vec::new()
                            };

                            self.scanned_models = list.into_iter()
                                .map(|m| {
                                    let exists = existing_models.contains(&m);
                                    // Mặc định check các mô hình đã tồn tại trong cấu hình từ trước
                                    (m, exists)
                                })
                                .collect();

                            self.selected_model_idx = 0;
                            self.model_search_query = String::new();
                            self.models_list_state = ratatui::widgets::ListState::default();
                            self.current_screen = Screen::ModelScanResult;
                        }
                        Err(e) => {
                            self.log(format!("Quét mô hình thất bại: {}", e));
                        }
                    }
                }
            }
            AppMessage::FormTest { status } => {
                self.form.is_testing = false;
                self.form.test_status = Some(status.clone());
                self.log(format!("Kết quả kiểm thử form: {}", status));
            }
        }
    }

    pub fn test_form_connection(&mut self) {
        if self.form.is_testing {
            return;
        }

        let base_url = self.form.base_url.clone();
        let api_key = self.form.api_key.clone();
        
        self.form.is_testing = true;
        self.form.test_status = None;
        self.log(format!("Bắt đầu kiểm thử form kết nối đến: {}", base_url));

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let client = ApiClient::new();
            let status = client.test_api(&base_url, &api_key).await;
            let _ = tx.send(AppMessage::FormTest { status });
        });
    }

    pub fn cycle_form_preset(&mut self, next: bool) {
        if self.presets.is_empty() {
            return;
        }
        let current_id = &self.form.preset_id;
        let current_idx = self.presets.iter().position(|p| p.id == *current_id).unwrap_or(0);
        let next_idx = if next {
            (current_idx + 1) % self.presets.len()
        } else {
            if current_idx == 0 { self.presets.len() - 1 } else { current_idx - 1 }
        };
        let new_preset = self.presets[next_idx].clone();
        self.form.preset_id = new_preset.id.clone();
        
        // Tự điền Name & URL nếu không phải Custom
        if new_preset.id != "custom" {
            self.form.name = new_preset.name.clone();
            self.form.base_url = new_preset.base_url.clone();
        } else {
            self.form.name = "Custom Provider".to_string();
            self.form.base_url = String::new();
        }
        self.form.test_status = None;
    }

    pub fn open_add_provider(&mut self) {
        let default_preset = self.presets.iter()
            .find(|p| p.id == "xiaomi-token-plan-sgp" || p.id_prefix == "mimo_sgp")
            .cloned()
            .unwrap_or_else(|| self.presets[0].clone());
            
        self.form = ProviderForm {
            id: String::new(),
            preset_id: default_preset.id.clone(),
            name: default_preset.name.clone(),
            base_url: default_preset.base_url.clone(),
            api_key: String::new(),
            focus_index: 0,
            test_status: None,
            is_testing: false,
            is_editing_field: false,
        };
        self.current_screen = Screen::AddProvider;
        self.log("Mở màn hình thêm Provider mới.");
    }

    pub fn open_edit_provider(&mut self) {
        if let Some(id) = self.selected_provider_id().cloned() {
            if let Some(provider) = self.config.provider.get(&id) {
                let preset_id = self.presets.iter()
                    .find(|p| p.base_url == provider.options.base_url)
                    .map(|p| p.id.clone())
                    .unwrap_or_else(|| "custom".to_string());
                    
                self.form = ProviderForm {
                    id: id.clone(),
                    preset_id,
                    name: provider.name.clone(),
                    base_url: provider.options.base_url.clone(),
                    api_key: provider.options.api_key.clone(),
                    focus_index: 0, // Bắt đầu ở Preset
                    test_status: self.api_status_cache.get(&id).cloned().flatten(),
                    is_testing: false,
                    is_editing_field: false,
                };
                self.current_screen = Screen::EditProvider;
                self.log(format!("Chỉnh sửa cấu hình Provider: {}", id));
            }
        }
    }

    pub fn save_form(&mut self) -> Result<(), String> {
        let mut id = self.form.id.trim().to_string();
        let name = self.form.name.trim().to_string();
        let base_url = self.form.base_url.trim().to_string();
        let api_key = self.form.api_key.trim().to_string();

        if name.is_empty() || base_url.is_empty() || api_key.is_empty() {
            return Err("Vui lòng nhập đầy đủ Name, Base URL và API Key!".to_string());
        }

        // Kiểm tra xem có trùng cả Base URL và API Key không (check cả auth.json và opencode.json)
        if let Some((dup_name, dup_id)) = self.detect_duplicate() {
            self.confirm_action = Some(ConfirmAction::OverwriteDuplicate {
                duplicate_id: dup_id,
                duplicate_name: dup_name,
            });
            self.confirm_focus_yes = false; // Mặc định No để an toàn
            self.current_screen = Screen::Confirmation;
            self.log("Phát hiện trùng lặp API và Base URL. Hiển thị xác nhận gộp/ghi đè.");
            return Ok(());
        }

        // Xác định xem đây là built-in native provider hay custom provider
        let mut is_builtin = false;
        let mut target_id = id.clone();

        if self.form.preset_id != "custom" {
            if let Some(preset) = self.presets.iter().find(|p| p.id == self.form.preset_id) {
                let clean_form_url = base_url.trim().trim_end_matches('/');
                let clean_preset_url = preset.base_url.trim().trim_end_matches('/');
                
                if clean_form_url == clean_preset_url {
                    if let Some(auth_entry) = self.auth_config.get(&preset.id) {
                        if auth_entry.key.trim() == api_key.trim() {
                            is_builtin = true;
                            target_id = preset.id.clone();
                        }
                    } else {
                        is_builtin = true;
                        target_id = preset.id.clone();
                    }
                }
            }
        }

        if is_builtin {
            id = target_id;
        } else {
            // Lưu vào opencode.json dưới dạng Custom provider
            if self.current_screen == Screen::AddProvider || id.is_empty() || id == self.form.preset_id {
                let prefix = self.presets.iter()
                    .find(|p| p.id == self.form.preset_id)
                    .map(|p| p.id_prefix.as_str())
                    .unwrap_or("custom");
                
                let mut candidate_id = prefix.to_string();
                if self.config.provider.contains_key(&candidate_id) {
                    let mut suffix = 2;
                    loop {
                        let candidate = format!("{}_{}", prefix, suffix);
                        if !self.config.provider.contains_key(&candidate) {
                            candidate_id = candidate;
                            break;
                        }
                        suffix += 1;
                    }
                }
                id = candidate_id;
            }
        }

        // Tìm npm từ preset nếu có
        let selected_preset = self.presets.iter().find(|p| p.id == self.form.preset_id);
        let npm = selected_preset.and_then(|p| p.npm.clone()).or(Some("@ai-sdk/openai-compatible".to_string()));

        // Tạo hoặc cập nhật provider
        let mut provider = if let Some(existing) = self.config.provider.get(&id) {
            let mut p = existing.clone();
            p.npm = npm;
            p
        } else {
            Provider {
                npm,
                name: name.clone(),
                options: ProviderOptions {
                    base_url: base_url.clone(),
                    api_key: api_key.clone(),
                },
                models: HashMap::new(),
            }
        };

        provider.name = name;
        provider.options.base_url = base_url;
        provider.options.api_key = api_key;

        self.config.provider.insert(id.clone(), provider);
        self.save_all_config()?;
        
        self.log(format!("Đã lưu cấu hình Provider: {}", id));
        self.update_provider_keys();
        
        // Thử check lại trạng thái ngầm ngay
        let tx = self.tx.clone();
        let base_url_check = self.config.provider.get(&id).unwrap().options.base_url.clone();
        let api_key_check = self.config.provider.get(&id).unwrap().options.api_key.clone();
        let provider_id = id.clone();
        tokio::spawn(async move {
            let client = ApiClient::new();
            let status = client.test_api(&base_url_check, &api_key_check).await;
            let _ = tx.send(AppMessage::Test { provider_id, status });
        });

        self.current_screen = Screen::Main;
        Ok(())
    }

    pub fn open_delete_provider_confirm(&mut self) {
        if let Some(id) = self.selected_provider_id().cloned() {
            self.confirm_action = Some(ConfirmAction::DeleteProvider(id.clone()));
            self.confirm_focus_yes = false; // Mặc định No để an toàn
            self.current_screen = Screen::Confirmation;
        }
    }

    pub fn execute_delete_provider(&mut self, id: &str) -> Result<(), String> {
        self.config.provider.remove(id);
        self.save_all_config()?;
        self.log(format!("Đã xoá Provider: {}", id));
        self.update_provider_keys();
        self.current_screen = Screen::Main;
        Ok(())
    }

    pub fn open_auth_keys_manager(&mut self) {
        self.reload_all_config_from_disk();
        self.selected_auth_idx = 0;
        self.auth_keys_list_state = ratatui::widgets::ListState::default();
        self.current_screen = Screen::ManageAuthKeys;
        self.log("Mở trình quản lý API Keys (auth.json).");
    }

    pub fn open_delete_auth_key_confirm(&mut self) {
        if let Some((name, _)) = self.auth_keys.get(self.selected_auth_idx) {
            self.confirm_action = Some(ConfirmAction::DeleteAuthKey(name.clone()));
            self.confirm_focus_yes = false;
            self.current_screen = Screen::Confirmation;
        }
    }

    pub fn execute_delete_auth_key(&mut self, name: &str) -> Result<(), String> {
        self.auth_config.remove(name);
        self.save_all_config()?;
        self.log(format!("Đã xoá Auth API Key: {}", name));
        self.reload_auth_keys();
        self.auth_keys_list_state = ratatui::widgets::ListState::default();
        self.current_screen = Screen::ManageAuthKeys;
        Ok(())
    }

    pub fn add_scanned_models(&mut self) -> Result<(), String> {
        let provider_id = self.scanning_provider_id.clone();
        
        if let Some(provider) = self.config.provider.get_mut(&provider_id) {
            let mut added_count = 0;
            let mut removed_count = 0;
            
            for (m_id, checked) in &self.scanned_models {
                if *checked {
                    // Nếu checked và chưa có trong config -> Thêm vào
                    if !provider.models.contains_key(m_id) {
                        let input_modalities = if m_id.contains("vision") || m_id.contains("omni") || m_id.contains("image") {
                            vec!["text".to_string(), "image".to_string()]
                        } else {
                            vec!["text".to_string()]
                        };

                        let model_entry = ModelEntry {
                            name: m_id.clone(),
                            limit: Some(ModelLimit {
                                context: Some(1048576), // mặc định 1M context
                                output: Some(131072),
                            }),
                            modalities: Some(ModelModalities {
                                input: input_modalities,
                                output: vec!["text".to_string()],
                            }),
                        };
                        provider.models.insert(m_id.clone(), model_entry);
                        added_count += 1;
                    }
                } else {
                    // Nếu unchecked và đang có trong config -> Xoá đi
                    if provider.models.contains_key(m_id) {
                        provider.models.remove(m_id);
                        removed_count += 1;
                    }
                }
            }
            
            self.save_all_config()?;
            self.log(format!(
                "Đồng bộ thành công cho {}: đã thêm {}, đã xoá {} mô hình.", 
                provider_id, added_count, removed_count
            ));
        }

        self.current_screen = Screen::Main;
        Ok(())
    }

    pub fn open_quick_clean(&mut self) {
        let mut clean_candidates = Vec::new();
        
        for id in &self.providers_keys {
            if let Some(status) = self.api_status_cache.get(id).cloned().flatten() {
                match status {
                    ApiStatus::InsufficientCredits(_) | ApiStatus::InvalidKey(_) => {
                        if let Some(p) = self.config.provider.get(id) {
                            clean_candidates.push((id.clone(), p.name.clone(), status.clone(), true));
                        }
                    }
                    _ => {}
                }
            }
        }

        if clean_candidates.is_empty() {
            self.log("Không phát hiện API nào hết hạn hoặc không hợp lệ cần dọn dẹp.");
            return;
        }

        self.clean_list = clean_candidates;
        self.selected_clean_idx = 0;
        self.clean_list_state = ratatui::widgets::ListState::default();
        self.current_screen = Screen::QuickClean;
        self.log(format!("Phát hiện {} API không khả dụng để dọn dẹp.", self.clean_list.len()));
    }

    pub fn execute_quick_clean(&mut self) -> Result<(), String> {
        let to_remove: Vec<String> = self.clean_list.iter()
            .filter(|(_, _, _, checked)| *checked)
            .map(|(id, _, _, _)| id.clone())
            .collect();

        if to_remove.is_empty() {
            self.current_screen = Screen::Main;
            return Ok(());
        }

        for id in &to_remove {
            self.config.provider.remove(id);
        }

        self.save_all_config()?;
        self.log(format!("Đã dọn dẹp nhanh {} providers hỏng.", to_remove.len()));
        self.update_provider_keys();
        self.current_screen = Screen::Main;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_builtin_and_custom_separation() {
        // Setup temporary HOME directory inside target to isolate the test
        let test_dir = std::env::current_dir().unwrap().join("target").join("test_home");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();
        
        // Override HOME, USERPROFILE and OPENCODE_TEST_HOME environment variables to isolate the test
        std::env::set_var("HOME", &test_dir);
        std::env::set_var("USERPROFILE", &test_dir);
        std::env::set_var("OPENCODE_TEST_HOME", &test_dir);

        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let opencode_json_path = opencode_dir.join("opencode.json");

        let share_dir = test_dir.join(".local").join("share").join("opencode");
        fs::create_dir_all(&share_dir).unwrap();
        let auth_json_path = share_dir.join("auth.json");

        // 1. Ghi dữ liệu mock ban đầu
        // auth.json có một built-in: xiaomi-token-plan-sgp
        let mock_auth = r#"{
            "xiaomi-token-plan-sgp": {
                "type": "api",
                "key": "tp-test-native-key"
            }
        }"#;
        fs::write(&auth_json_path, mock_auth).unwrap();

        // opencode.json có một custom: custom_mimo (trùng url nhưng khác key)
        let mock_opencode = r#"{
            "provider": {
                "custom_mimo": {
                    "npm": "@ai-sdk/openai-compatible",
                    "name": "MiMo Custom Override",
                    "options": {
                        "baseURL": "https://token-plan-sgp.xiaomimimo.com/v1",
                        "apiKey": "tp-custom-api-key"
                    },
                    "models": {}
                }
            }
        }"#;
        fs::write(&opencode_json_path, mock_opencode).unwrap();

        // 2. Load vào App
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Đảm bảo in-memory đã gộp cả 2
        assert!(app.config.provider.contains_key("xiaomi-token-plan-sgp"), "Native provider must be merged in-memory");
        assert!(app.config.provider.contains_key("custom_mimo"), "Custom provider must be loaded in-memory");

        // API Key phải tương ứng đúng
        assert_eq!(app.config.provider.get("xiaomi-token-plan-sgp").unwrap().options.api_key, "tp-test-native-key");
        assert_eq!(app.config.provider.get("custom_mimo").unwrap().options.api_key, "tp-custom-api-key");

        // 3. Thực hiện lưu
        app.save_all_config().unwrap();

        // Đọc lại file xem opencode.json có bị nhiễm native provider không
        let saved_opencode_content = fs::read_to_string(&opencode_json_path).unwrap();
        assert!(!saved_opencode_content.contains("xiaomi-token-plan-sgp"), "Native provider must not be saved in opencode.json");
        assert!(saved_opencode_content.contains("custom_mimo"), "Custom provider must be saved in opencode.json");

        // Đọc lại file auth.json xem có lưu đúng key
        let saved_auth_content = fs::read_to_string(&auth_json_path).unwrap();
        assert!(saved_auth_content.contains("xiaomi-token-plan-sgp"), "Native key must remain in auth.json");
        assert!(!saved_auth_content.contains("custom_mimo"), "Custom key must not be written to auth.json");

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_user_windows_config_format() {
        let json_data = r#"{
          "$schema": "https://opencode.ai/config.json",
          "model": "google/gemini-2.5-pro",
          "provider": {
            "google": {}
          }
        }"#;

        let config: Result<OpencodeConfig, _> = serde_json::from_str(json_data);
        assert!(config.is_ok(), "Failed to parse user Windows config format: {:?}", config.err());
        let config = config.unwrap();
        assert_eq!(config.model, Some("google/gemini-2.5-pro".to_string()));
        assert!(config.provider.contains_key("google"));
        let google_provider = config.provider.get("google").unwrap();
        assert_eq!(google_provider.name, "");
        assert_eq!(google_provider.options.base_url, "");
        assert_eq!(google_provider.options.api_key, "");
    }
}
