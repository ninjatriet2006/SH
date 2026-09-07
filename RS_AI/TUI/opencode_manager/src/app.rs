use crate::api::{ApiClient, ApiStatus};
use crate::ckey::{CkeyAiKey, CkeyModel, CkeyProfile, CkeyUsageItem, CkeyUsagePage, CkeyUsageStats};
use crate::config::{
    AuthConfig, CkeyConfig, Interleaved, ModelEntry, ModelModalities, OpencodeConfig, Provider, ProviderOptions,
    normalize_base_url,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    Main,
    AddProvider,
    EditProvider,
    ManageAuthKeys,
    ModelScanResult,
    ModelCaps,
    QuickClean,
    Confirmation,
    SelectPreset,
    CKeyDashboard,
    BulkAddProviders,
    CKeyImport,
    CKeyUsage,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DynamicPreset {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub id_prefix: String,
    pub npm: Option<String>,
}

/// Chế độ popup account key CKey (mở khi provider đang chọn chưa có account key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CkeyPickMode {
    Choose, // chọn từ account key đã lưu của provider khác
    New,    // nhập account key mới
}

/// Focus của màn hình thêm nhanh nhiều provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkFocus {
    Endpoint,
    Keys,
    Execute,
}

/// Khả năng (capability) của một model đang chỉnh trong màn hình ModelCaps.
///
/// Đây là bản nháp trên UI: `None` = "không đổi so với config" (để round-trip
/// không phá field người dùng đã đặt tay trong opencode.json), `Some` = ghi đè.
/// `interleaved` là tên field reasoning (vd "reasoning_content") — rỗng = bỏ.
#[derive(Debug, Clone, Default)]
pub struct ModelCapsDraft {
    pub tool_call: Option<bool>,
    pub reasoning: Option<bool>,
    pub interleaved_field: String,
    /// Người dùng đã vào ô nhập interleaved — từ đây dòng rỗng nghĩa là XOÁ
    /// field khỏi config (có đường gỡ capability đã đặt).
    pub interleaved_touched: bool,
}

impl ModelCapsDraft {
    /// Lấy draft ban đầu từ entry trong config (mở màn hình chỉnh).
    pub fn from_entry(entry: &ModelEntry) -> Self {
        let interleaved_field = match &entry.interleaved {
            Some(Interleaved::Object { field }) => field.clone(),
            Some(Interleaved::Field(f)) => f.clone(),
            _ => String::new(),
        };
        Self {
            tool_call: entry.tool_call,
            reasoning: entry.reasoning,
            interleaved_field,
            interleaved_touched: false,
        }
    }

    /// Ghi draft vào entry (chỉ đè field đã chỉnh).
    pub fn apply_to(&self, entry: &mut ModelEntry) {
        if let Some(v) = self.tool_call {
            entry.tool_call = Some(v);
        }
        if let Some(v) = self.reasoning {
            entry.reasoning = Some(v);
        }
        if self.interleaved_touched {
            let field = self.interleaved_field.trim();
            if field.is_empty() {
                entry.interleaved = None;
            } else {
                entry.interleaved = Some(Interleaved::Object {
                    field: field.to_string(),
                });
            }
        }
    }

    /// Mô tả ngắn để hiển thị ở footer/danh sách (vd "tool+reason").
    pub fn summary(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.tool_call == Some(true) {
            parts.push("tool");
        }
        if self.reasoning == Some(true) {
            parts.push("reason");
        }
        if !self.interleaved_field.trim().is_empty() {
            parts.push("interleaved");
        }
        if parts.is_empty() {
            "-".to_string()
        } else {
            parts.join("+")
        }
    }
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
    OverwriteDuplicate {
        duplicate_id: String,
        duplicate_name: String,
    },
}

pub enum AppMessage {
    Test {
        provider_id: String,
        status: ApiStatus,
    },
    Scan {
        provider_id: String,
        models: Result<Vec<String>, String>,
    },
    FormTest {
        status: ApiStatus,
    },
    Log(String),
    CkeyProfile {
        result: Result<CkeyProfile, String>,
    },
    CkeyKeys {
        result: Result<Vec<CkeyAiKey>, String>,
    },
    CkeyModels {
        result: Result<Vec<CkeyModel>, String>,
    },
    CkeyStats {
        result: Result<CkeyUsageStats, String>,
    },
    CkeyUsage {
        result: Result<CkeyUsagePage, String>,
    },
    CkeyDeposit {
        result: Result<crate::ckey::CkeyDepositInfo, String>,
    },
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
    // (model_id, is_checked, is_stale) — is_stale = model có trong config nhưng provider đã xoá
    pub scanned_models: Vec<(String, bool, bool)>,
    pub selected_model_idx: usize,
    pub scanning_provider_id: String,
    pub model_search_query: String,

    // Model capabilities editor (màn hình ModelCaps — case `hy3`)
    pub caps_draft: ModelCapsDraft,
    /// Id model đang chỉnh (snapshot — danh sách quét có thể được thay thế bởi
    /// background scan trong lúc người dùng đang ở màn hình này).
    pub caps_model_id: String,
    /// Ô đang focus: 0 = tool_call, 1 = reasoning, 2 = interleaved field.
    pub caps_focus: usize,
    /// Đang gõ chữ vào ô tên field interleaved.
    pub caps_field_editing: bool,

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

    // CKey account state
    pub ckey_config: CkeyConfig,
    pub ckey_profile: Option<CkeyProfile>, // profile của provider đang chọn
    pub ckey_stats: Option<CkeyUsageStats>,
    pub ckey_keys: Vec<CkeyAiKey>,   // keys của provider đang chọn (dùng cho import)
    pub ckey_models: Vec<CkeyModel>, // models của provider đang chọn (dùng cho import)
    pub ckey_usage: Vec<CkeyUsageItem>,
    pub ckey_usage_page: u64,
    pub ckey_usage_total_pages: u64,
    pub ckey_loading: bool,
    pub ckey_pending: u8, // số request CKey đang chờ (profile/keys/models/stats)
    pub ckey_error: Option<String>,
    // Popup account key CKey (provider đang chọn chưa có account key)
    pub ckey_need_key: bool,
    pub ckey_account_options: Vec<(String, String, String)>, // (profile_id, profile_name, account_key)
    pub ckey_pick_selected_idx: usize,
    pub ckey_pick_mode: CkeyPickMode,
    pub ckey_new_key_input: String,
    // Import model từ CKey: (model_id, checked, stale, input_price, output_price)
    pub ckey_import_list: Vec<(String, bool, bool, f64, f64)>,
    pub ckey_import_idx: usize,
    pub ckey_import_query: String,
    pub ckey_import_list_state: ratatui::widgets::ListState,
    pub ckey_usage_scroll: usize,
    /// Thông tin nạp tiền hiển thị trên dashboard ([D]).
    pub ckey_deposit: Option<crate::ckey::CkeyDepositInfo>,
    /// Số tiền nạp cho lần lấy thông tin tiếp theo (+/- đổi 10k mỗi bước).
    pub ckey_deposit_amount: u64,

    // Bulk add providers (màn hình K)
    pub bulk_endpoint_input: String,
    pub bulk_keys_input: String,
    pub bulk_focus: BulkFocus,
    pub bulk_is_editing: bool,

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
        tx: tokio::sync::mpsc::UnboundedSender<AppMessage>,
    ) -> Self {
        let mut keys: Vec<String> = config.provider.keys().cloned().collect();
        keys.sort();

        let mut api_status_cache = HashMap::new();
        for key in &keys {
            api_status_cache.insert(key.clone(), None);
        }

        let presets = Self::load_dynamic_presets();
        let default_preset = presets
            .iter()
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

        let (ckey_config, ckey_load_warn) = match CkeyConfig::load() {
            Ok(c) => (c, None),
            Err(e) => (CkeyConfig::default(), Some(e)),
        };
        let mut ckey_import_list_state = ratatui::widgets::ListState::default();
        ckey_import_list_state.select(Some(0));

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
            caps_draft: ModelCapsDraft::default(),
            caps_model_id: String::new(),
            caps_focus: 0,
            caps_field_editing: false,
            auth_keys: Vec::new(),
            selected_auth_idx: 0,
            clean_list: Vec::new(),
            selected_clean_idx: 0,
            presets,
            preset_search_query: String::new(),
            selected_preset_search_idx: 0,
            ckey_config,
            ckey_profile: None,
            ckey_stats: None,
            ckey_keys: Vec::new(),
            ckey_models: Vec::new(),
            ckey_usage: Vec::new(),
            ckey_usage_page: 1,
            ckey_usage_total_pages: 1,
            ckey_loading: false,
            ckey_pending: 0,
            ckey_error: None,
            ckey_need_key: false,
            ckey_account_options: Vec::new(),
            ckey_pick_selected_idx: 0,
            ckey_pick_mode: CkeyPickMode::Choose,
            ckey_new_key_input: String::new(),
            ckey_import_list: Vec::new(),
            ckey_import_idx: 0,
            ckey_import_query: String::new(),
            ckey_import_list_state,
            ckey_usage_scroll: 0,
            ckey_deposit: None,
            ckey_deposit_amount: 50_000,
            bulk_endpoint_input: String::new(),
            bulk_keys_input: String::new(),
            bulk_focus: BulkFocus::Endpoint,
            bulk_is_editing: false,
            provider_list_state,
            preset_list_state,
            models_list_state,
            auth_keys_list_state,
            clean_list_state,
            tx,
            logs: vec!["Sẵn sàng.".to_string()],
        };
        if let Some(warn) = ckey_load_warn {
            app.log(&warn);
        }
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
            if path.exists()
                && let Ok(content) = std::fs::read_to_string(&path)
            {
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
                DynamicPreset {
                    id: crate::ckey::CKEY_PRESET_ID.to_string(),
                    name: "CKey (ckey.vn)".to_string(),
                    base_url: crate::ckey::CKEY_LLM_BASE_URL.to_string(),
                    id_prefix: crate::ckey::CKEY_PRESET_ID.to_string(),
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
        self.presets
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.id.to_lowercase().contains(&query)
                    || p.base_url.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn filtered_scanned_models(&self) -> Vec<(usize, String, bool, bool)> {
        let query = self.model_search_query.to_lowercase();
        self.scanned_models
            .iter()
            .enumerate()
            .filter(|(_, (name, _, _))| name.to_lowercase().contains(&query))
            .map(|(idx, (name, checked, stale))| (idx, name.clone(), *checked, *stale))
            .collect()
    }

    pub fn detect_duplicate(&self) -> Option<(String, String)> {
        let id = self.form.id.trim();
        let base_url = self.form.base_url.trim();
        let api_key = self.form.api_key.trim();

        if base_url.is_empty() || api_key.is_empty() {
            return None;
        }

        let clean_new_url = normalize_base_url(base_url);

        // 1. Kiểm tra trùng lặp trong in-memory provider (đã gộp cả auth.json và opencode.json)
        for (prov_id, prov) in &self.config.provider {
            // Nếu ở chế độ Edit, bỏ qua so sánh với chính nó
            if (self.current_screen == Screen::EditProvider || !id.is_empty()) && prov_id == id {
                continue;
            }
            let clean_prov_url = normalize_base_url(&prov.options.base_url);
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
                let clean_preset_url = normalize_base_url(&preset.base_url);
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
        let npm = selected_preset
            .and_then(|p| p.npm.clone())
            .or(Some("@ai-sdk/openai-compatible".to_string()));

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
            self.config.provider.insert(
                duplicate_id.to_string(),
                Provider {
                    npm,
                    name,
                    options: ProviderOptions {
                        base_url,
                        api_key,
                        ..Default::default()
                    },
                    models: HashMap::new(),
                    ..Default::default()
                },
            );
        }

        // 4. Lưu cấu hình cả hai file
        self.save_all_config()?;

        self.log(format!(
            "Đã gộp/ghi đè cấu hình trùng lặp vào Provider: {}",
            duplicate_id
        ));
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
        let mut keys: Vec<(String, String)> = self
            .auth_config
            .iter()
            .map(|(k, v)| (k.clone(), v.key.clone()))
            .collect();
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
                    self.config.provider.insert(
                        auth_id.clone(),
                        Provider {
                            npm,
                            name: preset.name.clone(),
                            options: ProviderOptions {
                                base_url: preset.base_url.clone(),
                                api_key: auth_entry.key.clone(),
                                ..Default::default()
                            },
                            models: HashMap::new(),
                            ..Default::default()
                        },
                    );
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
                let clean_prov_url = normalize_base_url(&provider.options.base_url);
                let clean_preset_url = normalize_base_url(&preset.base_url);
                if clean_prov_url == clean_preset_url {
                    self.auth_config.insert(
                        id.clone(),
                        crate::config::AuthEntry {
                            auth_type: "api".to_string(),
                            key: provider.options.api_key.clone(),
                        },
                    );
                }
            }
        }

        // Dọn dẹp auth_config. Chỉ đụng entry mà merge_auth_into_providers HỢP
        // LỆ dùng (khớp preset + type "api" + key không rỗng) — entry OAuth
        // hoặc id ngoài preset (app khác ghi) phải giữ nguyên, xoá bừa là mất
        // dữ liệu người dùng. Trong phần merge quản lý, xoá 2 dạng:
        //   - Mồ côi: provider đã bị xoá (giữ lại thì provider sống lại).
        //   - Treo: provider còn nhưng không còn built-in (URL trỏ đi chỗ
        //     khác) — entry giữ khoá cũ, dữ liệu lệch.
        let mut keys_to_remove = Vec::new();
        for (auth_id, entry) in &self.auth_config {
            let would_merge = entry.auth_type == "api"
                && !entry.key.trim().is_empty()
                && self.presets.iter().any(|p| p.id == *auth_id);
            if !would_merge {
                continue;
            }
            let stale = match self.config.provider.get(auth_id) {
                None => true,
                Some(p) => {
                    let clean_prov_url = normalize_base_url(&p.options.base_url);
                    !self
                        .presets
                        .iter()
                        .any(|preset| preset.id == *auth_id && clean_prov_url == normalize_base_url(&preset.base_url))
                }
            };
            if stale {
                keys_to_remove.push(auth_id.clone());
            }
        }
        for k in keys_to_remove {
            self.auth_config.remove(&k);
        }

        // 2. Bản sao lọc để lưu vào opencode.json: provider custom + builtin
        //    CÓ DANH SÁCH MODEL khai báo rõ (người dùng đã chọn — opencode dùng
        //    đúng danh sách thay vì quét toàn bộ catalogue; xoá nó là mất lựa
        //    chọn). Builtin giữ lại thì xoá KHOÁ khỏi file — khoá builtin sống
        //    ở auth.json (đã sync ở bước 1), entry file chỉ mô tả danh sách.
        let mut file_config = self.config.clone();
        file_config.provider.retain(|id, provider| {
            let preset_match = self.presets.iter().any(|preset| {
                let clean_prov_url = normalize_base_url(&provider.options.base_url);
                let clean_preset_url = normalize_base_url(&preset.base_url);
                preset.id == *id && clean_prov_url == clean_preset_url
            });
            let builtin_key_only = preset_match && provider.models.is_empty();
            if preset_match && !builtin_key_only {
                provider.options.api_key.clear();
            }
            !builtin_key_only
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
            self.log(format!(
                "Đã đồng bộ: Nhập mới {} providers từ auth.json.",
                new_count - old_count
            ));
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
            new_cache.insert(key.clone(), self.api_status_cache.get(key).cloned().flatten());
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
        if let Some(id) = self.selected_provider_id().cloned()
            && let Some(provider) = self.config.provider.get(&id)
        {
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
        if let Some(id) = self.selected_provider_id().cloned()
            && let Some(provider) = self.config.provider.get(&id)
        {
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

    pub fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::Test { provider_id, status } => {
                self.api_status_cache.insert(provider_id.clone(), Some(status.clone()));
                if let Some(p) = self.config.provider.get(&provider_id) {
                    self.log(format!("Kiểm tra {}: {}", p.name, status));
                }

                // Kiểm tra xem đã hoàn thành toàn bộ Check All chưa
                if self.is_checking_all {
                    let all_done = self
                        .providers_keys
                        .iter()
                        .all(|k| self.api_status_cache.get(k).unwrap_or(&None).is_some());
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

                            // Các model đang có trong config nhưng KHÔNG còn trên provider
                            // (provider đã xoá model) → đánh dấu stale, mặc định unchecked để
                            // khi đồng bộ (Enter) sẽ bị xoá khỏi config.
                            let stale_models: Vec<String> =
                                existing_models.iter().filter(|m| !list.contains(*m)).cloned().collect();

                            if !stale_models.is_empty() {
                                self.log(format!(
                                    "Phát hiện {} mô hình đã bị provider xoá, sẽ xoá khỏi cấu hình khi đồng bộ: {}",
                                    stale_models.len(),
                                    stale_models.join(", ")
                                ));
                            }

                            let mut scanned: Vec<(String, bool, bool)> = list
                                .into_iter()
                                .map(|m| {
                                    let exists = existing_models.contains(&m);
                                    // Mặc định check các mô hình đã tồn tại trong cấu hình từ trước
                                    (m, exists, false)
                                })
                                .collect();

                            // Model bị provider xoá: xếp cuối danh sách, unchecked (mặc định sẽ bị xoá)
                            for m in stale_models {
                                scanned.push((m, false, true));
                            }

                            self.scanned_models = scanned;

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
            AppMessage::Log(msg) => {
                self.log(msg);
            }
            AppMessage::CkeyProfile { result } => {
                match result {
                    Ok(p) => {
                        self.ckey_profile = Some(p.clone());
                        self.log(format!("CKey profile: {} ({}).", p.username, p.balance));
                    }
                    Err(e) => {
                        self.ckey_error = Some(e.clone());
                        self.log(format!("Lỗi tải profile CKey: {}", e));
                    }
                }
                self.ckey_fetch_done();
            }
            AppMessage::CkeyKeys { result } => {
                match result {
                    Ok(list) => {
                        self.ckey_keys = list.clone();
                        self.log(format!("CKey: {} API key AI.", list.len()));
                    }
                    Err(e) => {
                        self.ckey_error = Some(e.clone());
                        self.log(format!("Lỗi tải keys CKey: {}", e));
                    }
                }
                self.ckey_fetch_done();
            }
            AppMessage::CkeyModels { result } => {
                match result {
                    Ok(list) => {
                        self.ckey_models = list.clone();
                        self.log(format!("CKey: {} model AI kèm bảng giá.", list.len()));
                    }
                    Err(e) => {
                        self.ckey_error = Some(e.clone());
                        self.log(format!("Lỗi tải models CKey: {}", e));
                    }
                }
                self.ckey_fetch_done();
            }
            AppMessage::CkeyStats { result } => {
                match result {
                    Ok(s) => {
                        self.ckey_stats = Some(s.clone());
                        self.log(format!(
                            "CKey: {} request, {} token, {}.",
                            s.requests, s.total_tokens, s.charged_vnd_text
                        ));
                    }
                    Err(e) => {
                        self.ckey_error = Some(e.clone());
                        self.log(format!("Lỗi tải usage-stats CKey: {}", e));
                    }
                }
                self.ckey_fetch_done();
            }
            AppMessage::CkeyUsage { result } => {
                self.ckey_loading = false;
                match result {
                    Ok(page) => {
                        if let Some(p) = &page.pagination {
                            self.ckey_usage_page = p.page;
                            self.ckey_usage_total_pages = p.total_pages.max(1);
                        }
                        self.ckey_usage = page.items;
                        self.log(format!(
                            "CKey usage trang {}/{}: {} bản ghi.",
                            self.ckey_usage_page,
                            self.ckey_usage_total_pages,
                            self.ckey_usage.len()
                        ));
                    }
                    Err(e) => {
                        self.ckey_error = Some(e.clone());
                        self.log(format!("Lỗi tải CKey usage: {}", e));
                    }
                }
            }
            AppMessage::CkeyDeposit { result } => {
                self.ckey_loading = false;
                match result {
                    Ok(info) => {
                        self.ckey_deposit = Some(info);
                        self.log("CKey: đã tải thông tin nạp tiền (D tải lại, +/- đổi số tiền).");
                    }
                    Err(e) => {
                        self.ckey_error = Some(e.clone());
                        self.log(format!("Lỗi tải thông tin nạp tiền: {}", e));
                    }
                }
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
            if current_idx == 0 {
                self.presets.len() - 1
            } else {
                current_idx - 1
            }
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
        let default_preset = self
            .presets
            .iter()
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
        if let Some(id) = self.selected_provider_id().cloned()
            && let Some(provider) = self.config.provider.get(&id)
        {
            let preset_id = self
                .presets
                .iter()
                .find(|p| normalize_base_url(&p.base_url) == normalize_base_url(&provider.options.base_url))
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

    pub fn save_form(&mut self) -> Result<(), String> {
        let mut id = self.form.id.trim().to_string();
        let name = self.form.name.trim().to_string();
        let raw_base_url = self.form.base_url.trim().to_string();
        let base_url = normalize_base_url(&raw_base_url);
        let api_key = self.form.api_key.trim().to_string();

        if base_url != raw_base_url {
            self.log(format!("Đã tự sửa base URL: {} → {}", raw_base_url, base_url));
        }

        // Đồng bộ giá trị đã normalize vào form để các bước sau (detect_duplicate,...) đọc đúng
        self.form.base_url = base_url.clone();

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

        if self.form.preset_id != "custom"
            && let Some(preset) = self.presets.iter().find(|p| p.id == self.form.preset_id)
        {
            let clean_form_url = normalize_base_url(&base_url);
            let clean_preset_url = normalize_base_url(&preset.base_url);

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

        if is_builtin {
            id = target_id;
        } else {
            // Lưu vào opencode.json dưới dạng Custom provider
            if self.current_screen == Screen::AddProvider || id.is_empty() || id == self.form.preset_id {
                let prefix = self
                    .presets
                    .iter()
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
        let npm = selected_preset
            .and_then(|p| p.npm.clone())
            .or(Some("@ai-sdk/openai-compatible".to_string()));

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
                    ..Default::default()
                },
                models: HashMap::new(),
                ..Default::default()
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

    /// Mở màn hình chỉnh capability cho model đang chọn trong danh sách quét.
    ///
    /// Draft lấy từ entry trong config nếu model đã có (để người dùng chỉnh
    /// tiếp thay vì gõ lại từ đầu), rỗng nếu model mới.
    pub fn open_model_caps(&mut self) {
        let filtered = self.filtered_scanned_models();
        if filtered.is_empty() || self.selected_model_idx >= filtered.len() {
            return;
        }
        let (original_idx, _, _, _) = filtered[self.selected_model_idx];
        let Some((model_id, _, _)) = self.scanned_models.get(original_idx) else {
            return;
        };
        let model_id = model_id.clone();

        let entry = self
            .config
            .provider
            .get(&self.scanning_provider_id)
            .and_then(|p| p.models.get(&model_id));
        self.caps_draft = entry.map(ModelCapsDraft::from_entry).unwrap_or_default();
        // Snapshot theo ID, không theo vị trí: danh sách quét có thể bị thay
        // thế bởi background scan trong lúc người dùng đang ở màn hình này.
        self.caps_model_id = model_id;
        self.caps_focus = 0;
        self.caps_field_editing = false;
        self.current_screen = Screen::ModelCaps;
        self.log(format!(
            "Chỉnh khả năng model '{}': Enter/Space đổi từng ô, S lưu, Esc huỷ.",
            self.caps_model_id
        ));
    }

    /// Hạ cánh draft capability vào config rồi LƯU NGAY (phím S).
    pub fn apply_model_caps(&mut self) -> Result<(), String> {
        let model_id = self.caps_model_id.clone();
        if model_id.is_empty() {
            return Err("Không tìm thấy model đang chỉnh.".to_string());
        }

        let provider_id = self.scanning_provider_id.clone();
        let provider = self
            .config
            .provider
            .get_mut(&provider_id)
            .ok_or_else(|| format!("Không tìm thấy provider: {provider_id}"))?;

        // Model chưa có trong config: chỉ tạo entry khi nó đang ĐƯỢC CHỌN
        // (tự sinh model chỉ vì người dùng chỉnh caps là rác config). Người
        // dùng muốn chọn thì tick rồi Enter đồng bộ như thường.
        if !provider.models.contains_key(&model_id) {
            let checked = self.scanned_models.iter().any(|(id, c, _)| *id == model_id && *c);
            if !checked {
                return Err(format!(
                    "Model '{model_id}' chưa được chọn — tick chọn nó trước khi đặt khả năng."
                ));
            }
            let input_modalities =
                if model_id.contains("vision") || model_id.contains("omni") || model_id.contains("image") {
                    vec!["text".to_string(), "image".to_string()]
                } else {
                    vec!["text".to_string()]
                };
            provider.models.insert(
                model_id.clone(),
                ModelEntry {
                    name: model_id.clone(),
                    // Không đoán limit: để None cho OpenCode/models.dev tự suy.
                    limit: None,
                    modalities: Some(ModelModalities {
                        input: input_modalities,
                        output: vec!["text".to_string()],
                    }),
                    ..Default::default()
                },
            );
        }

        let entry = provider.models.get_mut(&model_id).expect("entry vừa chắc chắn có");
        self.caps_draft.apply_to(entry);
        self.save_all_config()?;
        self.log(format!(
            "Đã lưu khả năng model '{}' ({}).",
            model_id,
            self.caps_draft.summary()
        ));
        self.current_screen = Screen::ModelScanResult;
        Ok(())
    }

    pub fn add_scanned_models(&mut self) -> Result<(), String> {
        let provider_id = self.scanning_provider_id.clone();

        if let Some(provider) = self.config.provider.get_mut(&provider_id) {
            let mut added_count = 0;
            let mut removed_count = 0;

            for (m_id, checked, _stale) in self.scanned_models.clone() {
                if checked {
                    // Nếu checked và chưa có trong config -> Thêm vào
                    if !provider.models.contains_key(&m_id) {
                        let input_modalities =
                            if m_id.contains("vision") || m_id.contains("omni") || m_id.contains("image") {
                                vec!["text".to_string(), "image".to_string()]
                            } else {
                                vec!["text".to_string()]
                            };

                        let model_entry = ModelEntry {
                            name: m_id.clone(),
                            // Không đoán limit: để None cho OpenCode/models.dev tự suy.
                            // Số hardcode 1M/131k gây 400 context_length_exceeded
                            // trên router có quota thực nhỏ hơn.
                            limit: None,
                            modalities: Some(ModelModalities {
                                input: input_modalities,
                                output: vec!["text".to_string()],
                            }),
                            ..Default::default()
                        };
                        provider.models.insert(m_id.clone(), model_entry);
                        added_count += 1;
                    }
                    // Capability model chỉnh ở màn ModelCaps được ghi thẳng
                    // khi bấm Lưu (S) — không cần gì thêm ở đây; Esc = huỷ.
                } else {
                    // Nếu unchecked và đang có trong config -> Xoá đi
                    if provider.models.contains_key(&m_id) {
                        provider.models.remove(&m_id);
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
        self.log(format!(
            "Phát hiện {} API không khả dụng để dọn dẹp.",
            self.clean_list.len()
        ));
    }

    pub fn execute_quick_clean(&mut self) -> Result<(), String> {
        let to_remove: Vec<String> = self
            .clean_list
            .iter()
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

    // ==================== CKEY ====================

    /// Provider ĐANG CHỌN có hỗ trợ kiểm tra thông tin tài khoản CKey không?
    /// Đúng nếu provider đang chọn (providers_keys[selected_provider_idx]) có base_url
    /// khớp CKEY_LLM_BASE_URL (so bằng normalize_base_url 2 vế để tránh trượt trailing slash).
    pub fn has_ckey_support(&self) -> bool {
        let target = normalize_base_url(crate::ckey::CKEY_LLM_BASE_URL);
        let is_ckey = |p: &Provider| normalize_base_url(&p.options.base_url) == target;

        // Provider đang chọn là CKey → G mở dashboard cho provider đó.
        if let Some(id) = self.providers_keys.get(self.selected_provider_idx)
            && let Some(p) = self.config.provider.get(id)
            && is_ckey(p)
        {
            return true;
        }
        // Chưa có provider CKey nào → G vẫn phải mở được (ngõ vào NHẬP account
        // key + import; provider tự tạo khi import). G là ngõ cụt là bug.
        !self
            .providers_keys
            .iter()
            .any(|id| self.config.provider.get(id).map(is_ckey).unwrap_or(false))
    }

    /// Provider mà luồng CKey đang thao tác: provider đang chọn nếu là CKey,
    /// rồi đến provider CKey đầu tiên, cuối cùng là id ẢO chuẩn `ckey`
    /// (chưa có provider nào). Chỉ còn test dùng — nghiệp vụ xem dữ liệu theo
    /// PROFILE active (ckey_view_key), đích import theo binding (import).
    #[cfg(test)]
    pub fn ckey_active_id(&self) -> Option<String> {
        let target = normalize_base_url(crate::ckey::CKEY_LLM_BASE_URL);
        let is_ckey = |p: &Provider| normalize_base_url(&p.options.base_url) == target;

        if let Some(id) = self.providers_keys.get(self.selected_provider_idx)
            && let Some(p) = self.config.provider.get(id)
            && is_ckey(p)
        {
            return Some(id.clone());
        }
        if let Some(id) = self
            .providers_keys
            .iter()
            .find(|id| self.config.provider.get(*id).map(is_ckey).unwrap_or(false))
        {
            return Some(id.clone());
        }
        Some(crate::ckey::CKEY_PRESET_ID.to_string())
    }

    /// Tra cứu account key của provider (qua binding → profile trong ckey.json).
    /// Chỉ test dùng — binding giờ chỉ được đọc/ghi lúc import.
    #[cfg(test)]
    pub fn ckey_account_key(&self, provider_id: &str) -> Option<String> {
        self.ckey_config.account_key(provider_id)
    }

    /// Account key của TÀI KHOẢN đang xem (profile active) — dashboard/usage/
    /// deposit đều theo tài khoản này, KHÔNG phụ thuộc provider nào.
    pub fn ckey_view_key(&self) -> Option<String> {
        self.ckey_config
            .active_profile()
            .map(|p| p.key.trim().to_string())
            .filter(|k| !k.is_empty())
    }

    /// Mở popup chọn tài khoản CKey (P): chọn profile đã lưu hoặc nhập key mới.
    /// Chỉ đổi tài khoản ĐANG XEM (active) — binding provider chỉ đổi khi import.
    pub fn open_ckey_profile_picker(&mut self) {
        self.ckey_error = None;
        self.ckey_need_key = true;
        self.ckey_pick_mode = CkeyPickMode::Choose;
        self.ckey_pick_selected_idx = 0;
        self.ckey_new_key_input = String::new();
        self.ckey_account_options = self
            .ckey_config
            .profiles
            .iter()
            .map(|p| (p.id.clone(), p.name.clone(), p.key.clone()))
            .collect();
        self.log("Chọn tài khoản CKey để xem (chỉ đổi tài khoản đang xem; import mới đổi provider).");
    }

    pub fn open_ckey_dashboard(&mut self) {
        self.ckey_error = None;
        self.current_screen = Screen::CKeyDashboard;

        if self.ckey_view_key().is_some() {
            self.ckey_fetch_all();
        } else {
            // Chưa có tài khoản nào → popup chọn profile đã lưu hoặc nhập key mới.
            self.open_ckey_profile_picker();
        }
    }

    /// Fetch song song cho tài khoản đang xem (active): profile + AI keys +
    /// models + usage-stats. Endpoint quản lý CỐ ĐỊNH CKEY_MANAGE_API_BASE.
    pub fn ckey_fetch_all(&mut self) {
        self.ckey_need_key = false;
        let Some(account_key) = self.ckey_view_key() else {
            self.ckey_error = Some("Chưa có tài khoản CKey nào.".to_string());
            self.open_ckey_profile_picker();
            return;
        };

        self.ckey_loading = true;
        self.ckey_pending = 4;
        self.ckey_error = None;
        let name = self
            .ckey_config
            .active_profile()
            .map(|p| p.name.clone())
            .unwrap_or_default();
        self.log(format!("Đang tải dữ liệu CKey cho tài khoản '{name}'..."));
        let endpoint = crate::ckey::CKEY_MANAGE_API_BASE.to_string();

        let tx = self.tx.clone();
        let ep = endpoint.clone();
        let key = account_key.clone();
        tokio::spawn(async move {
            let client = crate::ckey::CkeyClient::new(&ep);
            let result = client.fetch_profile(&key).await;
            let _ = tx.send(AppMessage::CkeyProfile { result });
        });
        let tx = self.tx.clone();
        let ep = endpoint.clone();
        let key = account_key.clone();
        tokio::spawn(async move {
            let client = crate::ckey::CkeyClient::new(&ep);
            let result = client.fetch_keys(&key).await;
            let _ = tx.send(AppMessage::CkeyKeys { result });
        });
        let tx = self.tx.clone();
        let ep = endpoint.clone();
        let key = account_key.clone();
        tokio::spawn(async move {
            let client = crate::ckey::CkeyClient::new(&ep);
            let result = client.fetch_models(&key).await;
            let _ = tx.send(AppMessage::CkeyModels { result });
        });
        let tx = self.tx.clone();
        let key = account_key;
        tokio::spawn(async move {
            let client = crate::ckey::CkeyClient::new(&endpoint);
            let result = client.fetch_usage_stats(&key, None).await;
            let _ = tx.send(AppMessage::CkeyStats { result });
        });
    }

    /// Popup: chuyển sang XEM một PROFILE đã lưu (chỉ đổi active — binding
    /// provider chỉ đổi khi import, nên không ảnh hưởng provider đang chạy).
    pub fn ckey_pick_account_key(&mut self, profile_id: &str) {
        if self.ckey_config.profile(profile_id).is_none() {
            self.log("CKey: tài khoản không tồn tại.");
            return;
        }
        self.ckey_config.active = Some(profile_id.to_string());
        if let Err(e) = self.ckey_config.save() {
            self.log(format!("Không thể lưu ckey.json: {}", e));
            return;
        }
        let name = self
            .ckey_config
            .profile(profile_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| profile_id.to_string());
        self.log(format!("Đã chuyển sang tài khoản '{}'.", name));
        self.ckey_need_key = false;
        self.ckey_fetch_all();
    }

    /// Popup: lưu account key mới (tự nhập) — tạo profile mới và XEM nó luôn;
    /// rỗng → Err. Binding provider chỉ được set khi import.
    pub fn ckey_save_new_account_key(&mut self) -> Result<(), String> {
        let new_key = self.ckey_new_key_input.trim().to_string();
        if new_key.is_empty() {
            return Err("Account key không được để trống.".to_string());
        }

        let id = self.ckey_config.next_profile_id();
        let name = format!("Tài khoản {}", self.ckey_config.profiles.len() + 1);
        self.ckey_config.profiles.push(crate::config::CkeyProfileEntry {
            id: id.clone(),
            name: name.clone(),
            key: new_key,
        });
        self.ckey_config.active = Some(id);
        self.ckey_config.save()?;
        self.log(format!("Đã tạo tài khoản CKey '{}'.", name));
        self.ckey_new_key_input.clear();
        self.ckey_need_key = false;
        self.ckey_fetch_all();
        Ok(())
    }

    /// Mở màn hình thêm nhanh nhiều provider (phím K).
    pub fn open_bulk_add(&mut self) {
        self.bulk_endpoint_input = String::new();
        self.bulk_keys_input = String::new();
        self.bulk_focus = BulkFocus::Endpoint;
        self.bulk_is_editing = false;
        self.current_screen = Screen::BulkAddProviders;
        self.log("Mở màn hình thêm nhanh nhiều provider.");
    }

    /// Thực hiện thêm nhanh: 1 endpoint + N key → N provider.
    /// Bỏ key trùng cặp (endpoint normalize + key) với provider đã có; id ngẫu nhiên unique.
    pub fn execute_bulk_add(&mut self) -> Result<usize, String> {
        let endpoint = normalize_base_url(self.bulk_endpoint_input.trim());
        if endpoint.is_empty() {
            return Err("Vui lòng nhập endpoint.".to_string());
        }
        // Validate: phải có scheme://host (không chấp nhận chuỗi thiếu scheme hoặc host rỗng).
        let host = endpoint.split_once("://").map(|(_, rest)| rest);
        let host_ok = host.map(|h| !h.is_empty() && !h.starts_with('/')).unwrap_or(false);
        if !host_ok {
            return Err("Endpoint không hợp lệ: phải có dạng https://host...".to_string());
        }

        // Parse từng dòng: trim + lọc rỗng + lọc trùng trong lần nhập.
        let mut seen = HashSet::new();
        let mut keys: Vec<String> = Vec::new();
        for line in self.bulk_keys_input.lines() {
            let k = line.trim().to_string();
            if k.is_empty() || !seen.insert(k.clone()) {
                continue;
            }
            keys.push(k);
        }
        if keys.is_empty() {
            return Err("Vui lòng dán ít nhất 1 AI key (mỗi dòng 1 key).".to_string());
        }

        let mut added = 0usize;
        let mut skipped = 0usize;
        let mut existing_ids: HashSet<String> = self.config.provider.keys().cloned().collect();

        for key in keys {
            // Bỏ cặp (endpoint, key) đã tồn tại trong provider
            if let Some((pid, p)) =
                self.config.provider.iter().find(|(_, p)| {
                    normalize_base_url(&p.options.base_url) == endpoint && p.options.api_key.trim() == key
                })
            {
                self.log(format!(
                    "Bỏ key trùng provider {} ({}).",
                    if p.name.is_empty() {
                        pid.as_str()
                    } else {
                        p.name.as_str()
                    },
                    pid
                ));
                skipped += 1;
                continue;
            }

            let id = crate::ckey::generate_provider_name(&existing_ids);
            existing_ids.insert(id.clone());
            self.config.provider.insert(
                id.clone(),
                Provider {
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                    name: id.clone(),
                    options: ProviderOptions {
                        base_url: endpoint.clone(),
                        api_key: key,
                        ..Default::default()
                    },
                    models: HashMap::new(),
                    ..Default::default()
                },
            );
            added += 1;
        }

        if added == 0 {
            self.log("Thêm nhanh: không có key mới nào được thêm (tất cả đã trùng).");
            return Ok(0);
        }

        self.save_all_config()?;
        self.update_provider_keys();
        self.log(format!("Đã thêm {} provider (bỏ {} key trùng).", added, skipped));
        self.bulk_endpoint_input.clear();
        self.bulk_keys_input.clear();
        self.bulk_is_editing = false;
        self.current_screen = Screen::Main;
        Ok(added)
    }

    /// Build danh sách import từ catalogue CKey + models hiện có của provider
    /// ĐÍCH (suy từ binding của tài khoản đang xem — không cần chọn).
    /// Model đã có trong config → checked; model mới → unchecked; model còn
    /// trong config nhưng KHÔNG còn trên CKey → stale (unchecked, sẽ bị xoá khi đồng bộ).
    pub fn open_ckey_import(&mut self) {
        if self.ckey_models.is_empty() {
            self.log("Catalogue model đang trống. Nhấn R để tải lại.");
            return;
        }
        let Some(active) = self.ckey_config.active_profile().map(|p| p.id.clone()) else {
            self.log("CKey: chưa có tài khoản nào. Nhấn P để chọn/nhập.");
            return;
        };
        let target = crate::ckey::resolve_import_target(&self.ckey_config, &active);
        let existing_models: Vec<String> = self
            .config
            .provider
            .get(&target)
            .map(|p| p.models.keys().cloned().collect())
            .unwrap_or_default();
        self.log(format!("Import model vào provider '{}'.", target));

        let mut list: Vec<(String, bool, bool, f64, f64)> = self
            .ckey_models
            .iter()
            .map(|m| {
                let exists = existing_models.contains(&m.public_name);
                (
                    m.public_name.clone(),
                    exists,
                    false,
                    m.input_price_per_million_vnd,
                    m.output_price_per_million_vnd,
                )
            })
            .collect();

        // Model bị CKey xoá nhưng còn trong config → stale, xếp cuối
        for stale in existing_models
            .iter()
            .filter(|m| !self.ckey_models.iter().any(|cm| &cm.public_name == *m))
        {
            list.push((stale.clone(), false, true, 0.0, 0.0));
        }

        let stale_count = list.iter().filter(|(_, _, s, _, _)| *s).count();
        if stale_count > 0 {
            self.log(format!(
                "CKey: {} mô hình đã bị xoá khỏi CKey, sẽ xoá khỏi config khi đồng bộ.",
                stale_count
            ));
        }

        self.ckey_import_list = list;
        self.ckey_import_idx = 0;
        self.ckey_import_query = String::new();
        self.ckey_import_list_state = ratatui::widgets::ListState::default();
        self.current_screen = Screen::CKeyImport;
    }

    pub fn filtered_ckey_import(&self) -> Vec<(usize, String, bool, bool, f64, f64)> {
        let q = self.ckey_import_query.to_lowercase();
        self.ckey_import_list
            .iter()
            .enumerate()
            .filter(|(_, (id, _, _, _, _))| id.to_lowercase().contains(&q))
            .map(|(i, (id, checked, stale, ip, op))| (i, id.clone(), *checked, *stale, *ip, *op))
            .collect()
    }

    /// Đồng bộ model CKey vào provider ĐÍCH (suy từ binding của tài khoản đang
    /// xem, mặc định id chuẩn "ckey") trong opencode.json.
    pub fn execute_ckey_import(&mut self) -> Result<(), String> {
        if self.ckey_import_list.is_empty() {
            return Err("Danh sách import rỗng.".to_string());
        }
        let Some(active) = self.ckey_config.active_profile().map(|p| p.id.clone()) else {
            return Err("CKey: chưa có tài khoản nào.".to_string());
        };
        let provider_id = crate::ckey::resolve_import_target(&self.ckey_config, &active);
        // Đổi tài khoản import = đổi nguồn AI key của provider (binding phải
        // khớp khoá thật, nếu không provider "dùng" tài khoản A mà trả tiền A).
        let rebinding = self.ckey_config.bindings.get(&provider_id) != Some(&active);

        // Tạo provider đích nếu chưa có; api_key = AI key active đầu tiên.
        if !self.config.provider.contains_key(&provider_id) {
            let api_key = self
                .ckey_keys
                .iter()
                .find(|k| k.is_active)
                .map(|k| k.api_key.clone())
                .unwrap_or_default();
            self.config.provider.insert(
                provider_id.clone(),
                Provider {
                    npm: Some("@ai-sdk/openai-compatible".to_string()),
                    name: "CKey (ckey.vn)".to_string(),
                    options: ProviderOptions {
                        base_url: crate::ckey::CKEY_LLM_BASE_URL.to_string(),
                        api_key,
                        ..Default::default()
                    },
                    models: HashMap::new(),
                    ..Default::default()
                },
            );
        }

        let provider = self
            .config
            .provider
            .get_mut(&provider_id)
            .expect("provider đích tồn tại hoặc vừa được tạo");

        let mut added = 0usize;
        let mut removed = 0usize;
        for (m_id, checked, _stale, _, _) in &self.ckey_import_list {
            if *checked {
                if !provider.models.contains_key(m_id) {
                    let input_modalities = if m_id.contains("vision") || m_id.contains("omni") || m_id.contains("image")
                    {
                        vec!["text".to_string(), "image".to_string()]
                    } else {
                        vec!["text".to_string()]
                    };
                    provider.models.insert(
                        m_id.clone(),
                        ModelEntry {
                            name: m_id.clone(),
                            // Không đoán limit (xem chú thích ở add_scanned_models).
                            limit: None,
                            modalities: Some(ModelModalities {
                                input: input_modalities,
                                output: vec!["text".to_string()],
                            }),
                            ..Default::default()
                        },
                    );
                    added += 1;
                }
            } else if provider.models.contains_key(m_id) {
                provider.models.remove(m_id);
                removed += 1;
            }
        }

        // Đảm bảo api_key không trống; đổi tài khoản (rebinding) thì cập nhật
        // khoá theo AI key active của tài khoản mới.
        if (provider.options.api_key.is_empty() || rebinding)
            && let Some(k) = self.ckey_keys.iter().find(|k| k.is_active)
        {
            provider.options.api_key = k.api_key.clone();
        }

        self.save_all_config()?;

        // Gắn binding provider → tài khoản import (ghi SAU save_all_config:
        // ckey.json và opencode.json là hai file độc lập).
        self.ckey_config.bindings.insert(provider_id.clone(), active);
        if let Err(e) = self.ckey_config.save() {
            self.log(format!("Không thể lưu ckey.json: {}", e));
        }

        self.log(format!(
            "Đồng bộ CKey vào '{}' thành công: đã thêm {}, đã xoá {} mô hình.",
            provider_id, added, removed
        ));
        self.update_provider_keys();
        self.current_screen = Screen::CKeyDashboard;
        Ok(())
    }

    pub fn open_ckey_usage(&mut self) {
        if self.ckey_view_key().is_none() {
            self.log("CKey: chưa có tài khoản nào để tra cứu. Nhấn P để chọn/nhập.");
            return;
        }
        self.current_screen = Screen::CKeyUsage;
        self.ckey_fetch_usage_page(1);
    }

    pub fn ckey_fetch_usage_page(&mut self, page: u64) {
        let Some(account_key) = self.ckey_view_key() else {
            self.log("CKey: chưa có tài khoản nào để tra cứu.");
            return;
        };
        let endpoint = crate::ckey::CKEY_MANAGE_API_BASE.to_string();
        self.ckey_loading = true;
        self.ckey_error = None;
        self.ckey_usage_scroll = 0;
        self.log(format!("Đang tải lịch sử dùng AI trang {}", page));

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let client = crate::ckey::CkeyClient::new(&endpoint);
            let result = client.fetch_usage(&account_key, None, page, 30).await;
            let _ = tx.send(AppMessage::CkeyUsage { result });
        });
    }

    fn ckey_fetch_done(&mut self) {
        self.ckey_pending = self.ckey_pending.saturating_sub(1);
        if self.ckey_pending == 0 {
            self.ckey_loading = false;
        }
    }

    /// Lấy thông tin nạp tiền (docs: /api/deposit-info) cho TÀI KHOẢN đang xem
    /// với số tiền hiện tại (`ckey_deposit_amount`, đổi bằng +/-).
    pub fn ckey_fetch_deposit(&mut self) {
        let Some(account_key) = self.ckey_view_key() else {
            self.log("CKey: chưa có tài khoản nào. Nhấn P để chọn/nhập.");
            return;
        };
        let amount = self.ckey_deposit_amount;
        self.ckey_loading = true;
        self.log(format!("Đang lấy thông tin nạp {}₫...", amount));
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let client = crate::ckey::CkeyClient::new(crate::ckey::CKEY_MANAGE_API_BASE);
            let result = client.fetch_deposit_info(&account_key, amount).await;
            let _ = tx.send(AppMessage::CkeyDeposit { result });
        });
    }

    /// Đổi số tiền nạp theo bước 10.000₫ (floor 10.000, ceiling 100 triệu).
    pub fn ckey_deposit_amount_step(&mut self, up: bool) {
        if up {
            self.ckey_deposit_amount = (self.ckey_deposit_amount + 10_000).min(100_000_000);
        } else {
            self.ckey_deposit_amount = self.ckey_deposit_amount.saturating_sub(10_000).max(10_000);
        }
        self.log(format!(
            "Số tiền nạp: {}₫ (nhấn D để lấy thông tin).",
            self.ckey_deposit_amount
        ));
    }

    // ==================== END CKEY ====================

    pub fn launch_opencode(&mut self) {
        self.log("Đang tìm kiếm opencode trên máy...");
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let path_opt = tokio::task::spawn_blocking(|| {
                let temp_dir = std::env::temp_dir();
                let script_path = temp_dir.join("find_opencode.ps1");
                let script_content = r#"
$exe = Get-Command opencode -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if ($exe -and (Test-Path $exe)) { Write-Output $exe; exit }

$paths = @(
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*"
)
$regItems = Get-ItemProperty -Path $paths -ErrorAction SilentlyContinue | Where-Object {
    $_.DisplayName -match "opencode" -or
    $_.PSChildName -match "opencode" -or
    $_.InstallLocation -match "opencode" -or
    $_.UninstallString -match "opencode"
}

foreach ($item in $regItems) {
    if ($item.InstallLocation -and (Test-Path $item.InstallLocation)) {
        $p = Join-Path $item.InstallLocation "opencode.exe"
        if (Test-Path $p) { Write-Output $p; exit }
    }
    if ($item.UninstallString -match '"([^"]+)"') {
        $unpath = $Matches[1]
        $dir = Split-Path $unpath -Parent
        $p = Join-Path $dir "opencode.exe"
        if (Test-Path $p) { Write-Output $p; exit }
    }
}

$fallbacks = @(
    "$env:LOCALAPPDATA\Programs\opencode\opencode.exe",
    "$env:ProgramFiles\opencode\opencode.exe",
    "$env:SystemDrive\Program Files (x86)\opencode\opencode.exe"
)
foreach ($fb in $fallbacks) {
    if (Test-Path $fb) { Write-Output $fb; exit }
}

$wingetPath = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\SST.opencode*" -ErrorAction SilentlyContinue | 
    ForEach-Object { Join-Path $_.FullName "opencode.exe" } | 
    Where-Object { Test-Path $_ } | 
    Select-Object -First 1
if ($wingetPath) { Write-Output $wingetPath; exit }
"#;

                if std::fs::write(&script_path, script_content).is_err() {
                    return None;
                }

                let output = std::process::Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-File")
                    .arg(&script_path)
                    .output();

                let _ = std::fs::remove_file(&script_path);

                if let Ok(out) = output
                    && out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let path = stdout.trim().to_string();
                        if !path.is_empty() {
                            return Some(path);
                        }
                    }
                None
            }).await.unwrap_or(None);

            if let Some(path) = path_opt {
                let _ = tx.send(AppMessage::Log(format!("Tìm thấy opencode tại: {}", path)));

                let status_res = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("cmd")
                        .arg("/c")
                        .arg("start")
                        .arg("")
                        .arg(&path)
                        .status()
                })
                .await
                .unwrap_or_else(|e| Err(std::io::Error::other(e.to_string())));

                match status_res {
                    Ok(status) if status.success() => {
                        let _ = tx.send(AppMessage::Log("Đã khởi chạy opencode thành công.".to_string()));
                    }
                    Ok(status) => {
                        let _ = tx.send(AppMessage::Log(format!(
                            "Khởi chạy opencode lỗi (Exit code: {}).",
                            status
                        )));
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::Log(format!("Không thể khởi chạy opencode: {}", e)));
                    }
                }
            } else {
                let _ = tx.send(AppMessage::Log(
                    "Không tìm thấy opencode trên máy. Hãy cài đặt opencode trước.".to_string(),
                ));
            }
        });
    }
}

#[cfg(test)]
pub(crate) use tests::TEST_ENV_LOCK;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    // Các test dưới đây ghi đè biến môi trường HOME/OPENCODE_TEST_HOME (global process-wide).
    // Phải tuần tự hoá chúng để không cướp env của nhau khi chạy song song (race condition).
    pub(crate) static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_builtin_and_custom_separation() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        // Setup temporary HOME directory inside target to isolate the test
        let test_dir = std::env::current_dir().unwrap().join("target").join("test_home");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // Override HOME, USERPROFILE and OPENCODE_TEST_HOME environment variables to isolate the test
        // SAFETY: This is a test function running in a controlled environment
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

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
        assert!(
            app.config.provider.contains_key("xiaomi-token-plan-sgp"),
            "Native provider must be merged in-memory"
        );
        assert!(
            app.config.provider.contains_key("custom_mimo"),
            "Custom provider must be loaded in-memory"
        );

        // API Key phải tương ứng đúng
        assert_eq!(
            app.config
                .provider
                .get("xiaomi-token-plan-sgp")
                .unwrap()
                .options
                .api_key,
            "tp-test-native-key"
        );
        assert_eq!(
            app.config.provider.get("custom_mimo").unwrap().options.api_key,
            "tp-custom-api-key"
        );

        // 3. Thực hiện lưu
        app.save_all_config().unwrap();

        // Đọc lại file xem opencode.json có bị nhiễm native provider không
        let saved_opencode_content = fs::read_to_string(&opencode_json_path).unwrap();
        assert!(
            !saved_opencode_content.contains("xiaomi-token-plan-sgp"),
            "Native provider must not be saved in opencode.json"
        );
        assert!(
            saved_opencode_content.contains("custom_mimo"),
            "Custom provider must be saved in opencode.json"
        );

        // Đọc lại file auth.json xem có lưu đúng key
        let saved_auth_content = fs::read_to_string(&auth_json_path).unwrap();
        assert!(
            saved_auth_content.contains("xiaomi-token-plan-sgp"),
            "Native key must remain in auth.json"
        );
        assert!(
            !saved_auth_content.contains("custom_mimo"),
            "Custom key must not be written to auth.json"
        );

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_builtin_broken_url_normalized_still_auth_json() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_builtin_broken_url");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let opencode_json_path = opencode_dir.join("opencode.json");
        let share_dir = test_dir.join(".local").join("share").join("opencode");
        fs::create_dir_all(&share_dir).unwrap();
        let auth_json_path = share_dir.join("auth.json");

        // Provider built-in "ckey" có base_url HỎNG (nhập thừa path) — do lần lưu cũ chưa normalize
        let mock_opencode = r#"{
            "provider": {
                "ckey": {
                    "npm": "@ai-sdk/openai-compatible",
                    "name": "CKey (ckey.vn)",
                    "options": {
                        "baseURL": "https://api.xah.io/v1/chat/completions",
                        "apiKey": "ck-broken-old"
                    },
                    "models": {}
                }
            }
        }"#;
        fs::write(&opencode_json_path, mock_opencode).unwrap();
        fs::write(&auth_json_path, "{}").unwrap();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // URL hỏng vẫn khớp built-in sau normalize → ckey nằm auth.json, không vào opencode.json
        assert!(app.config.provider.contains_key("ckey"));

        app.save_all_config().unwrap();

        let saved_opencode = fs::read_to_string(&opencode_json_path).unwrap();
        assert!(
            !saved_opencode.contains("ckey"),
            "Built-in ckey với base_url hỏng vẫn phải bị loại khỏi opencode.json"
        );
        let saved_auth = fs::read_to_string(&auth_json_path).unwrap();
        assert!(saved_auth.contains("ckey"), "Built-in ckey phải nằm trong auth.json");

        // Backward compat: get_models_url chuẩn hoá URL hỏng trước khi append
        assert_eq!(
            ApiClient::get_models_url("https://api.xah.io/v1/chat/completions"),
            "https://api.xah.io/v1/models"
        );

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
        assert!(
            config.is_ok(),
            "Failed to parse user Windows config format: {:?}",
            config.err()
        );
        let config = config.unwrap();
        assert_eq!(config.model, Some("google/gemini-2.5-pro".to_string()));
        assert!(config.provider.contains_key("google"));
        let google_provider = config.provider.get("google").unwrap();
        assert_eq!(google_provider.name, "");
        assert_eq!(google_provider.options.base_url, "");
        assert_eq!(google_provider.options.api_key, "");
    }

    /// Helper: parse model fixtures từ JSON CKey (không cần mạng).
    fn parse_ckey_models_fixture() -> Vec<crate::ckey::CkeyModel> {
        let body = r#"{
            "success": true, "status": 200, "message": "OK",
            "data": { "models": [
                { "public_name": "provider/gpt-demo", "display_name": "GPT Demo",
                  "model_name": "gpt-demo", "provider_username": "provider",
                  "is_provider_model": true,
                  "input_price_per_million_vnd": 5000, "output_price_per_million_vnd": 15000,
                  "price_per_request_vnd": 0, "min_charge_per_request_vnd": 1,
                  "cache_enabled": false, "cache_read_price_per_million_vnd": 0,
                  "cache_write_price_per_million_vnd": 0, "request_rate_limit_per_minute": 0,
                  "max_output_tokens_limit": 0, "context_tokens_limit": 0,
                  "supported_paths": ["chat/completions"] },
                { "public_name": "provider/gpt-removed", "display_name": "GPT Removed",
                  "model_name": "gpt-removed", "provider_username": "provider",
                  "is_provider_model": true,
                  "input_price_per_million_vnd": 1000, "output_price_per_million_vnd": 3000,
                  "price_per_request_vnd": 0, "min_charge_per_request_vnd": 0,
                  "cache_enabled": false, "cache_read_price_per_million_vnd": 0,
                  "cache_write_price_per_million_vnd": 0, "request_rate_limit_per_minute": 0,
                  "max_output_tokens_limit": 0, "context_tokens_limit": 0,
                  "supported_paths": ["chat/completions"] }
            ] }
        }"#;
        crate::ckey::parse_wrapped::<crate::ckey::CkeyModelList>(body)
            .unwrap()
            .models
    }

    #[test]
    fn test_ckey_config_roundtrip_profiles() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_ckey_config_map");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let cfg = crate::config::CkeyConfig {
            profiles: vec![
                crate::config::CkeyProfileEntry {
                    id: "p1".to_string(),
                    name: "ckey".to_string(),
                    key: "ck-account-1".to_string(),
                },
                crate::config::CkeyProfileEntry {
                    id: "p2".to_string(),
                    name: "X7K2P9".to_string(),
                    key: "ck-account-2".to_string(),
                },
            ],
            active: Some("p2".to_string()),
            bindings: std::collections::HashMap::from([
                ("ckey".to_string(), "p1".to_string()),
                ("X7K2P9".to_string(), "p2".to_string()),
            ]),
        };
        cfg.save().unwrap();
        let loaded = crate::config::CkeyConfig::load().unwrap();
        assert_eq!(loaded, cfg);
        // Binding → đúng key của profile tương ứng.
        assert_eq!(loaded.account_key("ckey").as_deref(), Some("ck-account-1"));
        assert_eq!(loaded.account_key("X7K2P9").as_deref(), Some("ck-account-2"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_ckey_config_migration_old_forms() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_ckey_migration");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        // File rất cũ: { account_key } → 1 profile gắn cho "ckey"
        // Vị trí file giờ là .config/opencode-manager/ (đã tách khỏi opencode);
        // test format-migration ghi thẳng vào vị trí hiện hành.
        let ckey_path = crate::config::CkeyConfig::file_path();
        fs::create_dir_all(ckey_path.parent().unwrap()).unwrap();
        fs::write(&ckey_path, r#"{"account_key":"ck-xxx"}"#).unwrap();
        let loaded = crate::config::CkeyConfig::load().unwrap();
        assert_eq!(loaded.profiles.len(), 1, "migration phải tạo 1 profile");
        assert_eq!(loaded.account_key("ckey").as_deref(), Some("ck-xxx"));

        // File cũ: { endpoint, accounts: [{name, key}] } → account đầu thành 1 profile
        fs::write(
            &ckey_path,
            r#"{"endpoint":"https://ckey.vn","accounts":[{"name":"CKey-a1b2c3","key":"ck-account-1"},{"name":"CKey-d4e5f6","key":"ck-account-2"}]}"#,
        )
        .unwrap();
        let loaded2 = crate::config::CkeyConfig::load().unwrap();
        assert_eq!(loaded2.profiles.len(), 1, "chỉ lấy account đầu tiên");
        assert_eq!(loaded2.account_key("ckey").as_deref(), Some("ck-account-1"));

        // File cũ map nhiều provider → MỘT profile mỗi provider, binding giữ nguyên.
        fs::write(&ckey_path, r#"{"accounts":{"ckey":"k1","X7K2P9":"k2"}}"#).unwrap();
        let loaded3 = crate::config::CkeyConfig::load().unwrap();
        assert_eq!(loaded3.profiles.len(), 2);
        assert_eq!(loaded3.account_key("ckey").as_deref(), Some("k1"));
        assert_eq!(loaded3.account_key("X7K2P9").as_deref(), Some("k2"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_generate_provider_name_unique() {
        let existing: std::collections::HashSet<String> =
            ["ckey".to_string(), "X7K2P9".to_string(), "abc123".to_string()]
                .into_iter()
                .collect();
        for _ in 0..200 {
            let name = crate::ckey::generate_provider_name(&existing);
            assert!(!existing.contains(&name), "tên phải unique so với existing");
            assert_eq!(name.len(), 6, "phải đúng 6 ký tự");
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric()), "phải là ký tự alnum");
        }
    }

    #[test]
    fn test_bulk_add_creates_providers_and_skips_duplicates() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_bulk_add");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();
        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        fs::write(opencode_dir.join("opencode.json"), "{}").unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Endpoint rỗng → Err
        app.bulk_endpoint_input = "  ".to_string();
        app.bulk_keys_input = "key-1\nkey-2".to_string();
        let err = app.execute_bulk_add().unwrap_err();
        assert!(err.contains("endpoint"), "err = {}", err);

        // Keys rỗng → Err
        app.bulk_endpoint_input = "https://api.example.com/v1".to_string();
        app.bulk_keys_input = "\n  \n".to_string();
        let err = app.execute_bulk_add().unwrap_err();
        assert!(err.contains("ít nhất 1"), "err = {}", err);

        // 2 key khác nhau → tạo 2 provider id random (6 ký tự alnum), không trùng builtin ckey
        app.bulk_endpoint_input = "https://api.example.com/v1".to_string();
        app.bulk_keys_input = "key-1\nkey-2".to_string();
        let added = app.execute_bulk_add().unwrap();
        assert_eq!(added, 2, "phải thêm được 2 provider");

        let ep = normalize_base_url("https://api.example.com/v1");
        let matched: Vec<_> = app
            .config
            .provider
            .iter()
            .filter(|(_, p)| normalize_base_url(&p.options.base_url) == ep)
            .collect();
        assert_eq!(matched.len(), 2, "phải có 2 provider cùng endpoint");
        assert!(
            matched
                .iter()
                .all(|(id, _)| id.len() == 6 && id.chars().all(|c| c.is_ascii_alphanumeric())),
            "id phải là 6 ký tự alnum ngẫu nhiên"
        );
        assert!(
            matched.iter().all(|(id, _)| *id != crate::ckey::CKEY_PRESET_ID),
            "không được trùng id builtin ckey"
        );
        assert!(
            matched
                .iter()
                .all(|(_, p)| p.npm.as_deref() == Some("@ai-sdk/openai-compatible"))
        );

        // Chạy lại cùng endpoint + key → cặp trùng bị bỏ, không thêm mới
        app.bulk_endpoint_input = "https://api.example.com/v1".to_string();
        app.bulk_keys_input = "key-1\nkey-2".to_string();
        let added = app.execute_bulk_add().unwrap();
        assert_eq!(added, 0, "cặp trùng phải được bỏ");

        // Provider thêm nhanh (không builtin) phải được ghi vào opencode.json
        let saved = fs::read_to_string(opencode_dir.join("opencode.json")).unwrap();
        assert!(
            saved.contains("api.example.com"),
            "provider thêm nhanh phải nằm trong opencode.json"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_bulk_add_skips_existing_pair() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_bulk_add_skip");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        // Provider "myprov" có base_url + apiKey = "secret-key-1" (cặp trùng sẽ bị bỏ khi bulk add)
        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        fs::write(
            opencode_dir.join("opencode.json"),
            r#"{
                "provider": {
                    "myprov": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "My Provider",
                        "options": { "baseURL": "https://api.example.com/v1", "apiKey": "secret-key-1" },
                        "models": {}
                    }
                }
            }"#,
        )
        .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Key trùng CẢ endpoint (trailing slash vẫn khớp) + key với provider có sẵn → bỏ key đó
        app.bulk_endpoint_input = "https://api.example.com/v1/".to_string();
        app.bulk_keys_input = "secret-key-1\nsecret-key-2".to_string();
        let added = app.execute_bulk_add().unwrap();
        assert_eq!(added, 1, "chỉ thêm key mới, key trùng cặp phải bị bỏ");

        assert!(
            app.config.provider.contains_key("myprov"),
            "provider có sẵn phải giữ nguyên"
        );
        assert_eq!(app.config.provider.len(), 2, "myprov + 1 provider mới");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_has_ckey_support() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_ckey_support");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();
        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        fs::write(opencode_dir.join("opencode.json"), "{}").unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // 1. Không có provider nào → G VẪN mở được (ngõ vào nhập account key
        //    + import — provider tự tạo khi import). Trước đây trả false khiến
        //    G là ngõ cụt: không thể thêm account CKey khi chưa có provider.
        assert!(app.has_ckey_support(), "không provider → G phải mở được");
        assert_eq!(
            app.ckey_active_id().as_deref(),
            Some(crate::ckey::CKEY_PRESET_ID),
            "chưa có provider → active id là id ảo chuẩn ckey"
        );

        // 2. Có provider CKey nhưng CHƯA chọn nó → has_ckey_support false
        app.config.provider.insert(
            "p1".to_string(),
            Provider {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "P1".to_string(),
                options: ProviderOptions {
                    base_url: "https://api.xah.io/v1".to_string(),
                    api_key: "k1".to_string(),
                    ..Default::default()
                },
                models: HashMap::new(),
                ..Default::default()
            },
        );
        app.update_provider_keys();
        let p1_idx = app.providers_keys.iter().position(|id| id == "p1").unwrap();
        app.selected_provider_idx = p1_idx;
        assert!(app.has_ckey_support(), "provider đang chọn api.xah.io/v1 → true");

        // 3. trailing slash vẫn khớp
        app.config.provider.get_mut("p1").unwrap().options.base_url = "https://api.xah.io/v1/".to_string();
        assert!(app.has_ckey_support(), "trailing slash phải khớp → true");

        // 4. Có CKey nhưng chuyển sang chọn provider khác → false
        app.config.provider.insert(
            "p3".to_string(),
            Provider {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "P3".to_string(),
                options: ProviderOptions {
                    base_url: "https://api.deepseek.com/v1".to_string(),
                    api_key: "k3".to_string(),
                    ..Default::default()
                },
                models: HashMap::new(),
                ..Default::default()
            },
        );
        app.update_provider_keys();
        let p3_idx = app.providers_keys.iter().position(|id| id == "p3").unwrap();
        app.selected_provider_idx = p3_idx;
        assert!(!app.has_ckey_support(), "provider đang chọn khác → false");

        // 5. Xoá hết provider → G lại mở được (điểm bắt đầu mới)
        app.config.provider.clear();
        app.update_provider_keys();
        assert!(app.has_ckey_support(), "rỗng → G phải mở được để thêm account");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ckey_dashboard_need_key_popup_and_pick() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_ckey_account_key_popup");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();
        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        fs::write(opencode_dir.join("opencode.json"), "{}").unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Provider CKey (base_url api.xah.io/v1) + provider khác đang chọn
        app.config.provider.insert(
            "p-ckey".to_string(),
            Provider {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "P CKey".to_string(),
                options: ProviderOptions {
                    base_url: crate::ckey::CKEY_LLM_BASE_URL.to_string(),
                    api_key: "k1".to_string(),
                    ..Default::default()
                },
                models: HashMap::new(),
                ..Default::default()
            },
        );
        app.config.provider.insert(
            "p-other".to_string(),
            Provider {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "P Other".to_string(),
                options: ProviderOptions {
                    base_url: crate::ckey::CKEY_LLM_BASE_URL.to_string(),
                    api_key: "k2".to_string(),
                    ..Default::default()
                },
                models: HashMap::new(),
                ..Default::default()
            },
        );
        app.update_provider_keys();
        let p_idx = app.providers_keys.iter().position(|id| id == "p-ckey").unwrap();
        app.selected_provider_idx = p_idx;

        // Chưa có account key → mở dashboard phải bật popup cần key
        assert!(app.ckey_account_key("p-ckey").is_none());
        app.open_ckey_dashboard();
        assert!(app.ckey_need_key, "chưa có account key → phải bật popup");

        // Lưu account key mới (rỗng → Err)
        app.ckey_new_key_input = "  ".to_string();
        let err = app.ckey_save_new_account_key().unwrap_err();
        assert!(err.contains("không được để trống"), "err = {}", err);

        // Lưu account key mới hợp lệ → tạo profile + đặt làm tài khoản ĐANG XEM
        // (binding provider chỉ được set khi import).
        app.ckey_new_key_input = "ck-account-1".to_string();
        app.ckey_save_new_account_key().unwrap();
        assert_eq!(
            app.ckey_view_key().as_deref(),
            Some("ck-account-1"),
            "tài khoản đang xem phải là profile vừa tạo"
        );
        assert!(
            app.ckey_account_key("p-ckey").is_none(),
            "chưa import → chưa có binding"
        );
        assert!(!app.ckey_need_key, "sau khi lưu key mới phải tắt popup");
        let saved_profile_id = app
            .ckey_config
            .active_profile()
            .map(|p| p.id.clone())
            .expect("profile active phải có sau khi lưu");

        // Mở lại popup bằng P (chuyển tài khoản đang xem) → liệt kê profile đã lưu
        app.open_ckey_profile_picker();
        assert!(app.ckey_need_key, "P phải mở popup chọn tài khoản");
        assert!(
            app.ckey_account_options
                .iter()
                .any(|(pid, _, _)| *pid == saved_profile_id),
            "danh sách chọn phải chứa profile vừa tạo"
        );

        // Chuyển sang profile đó → chỉ đổi tài khoản ĐANG XEM, không binding
        app.ckey_pick_account_key(&saved_profile_id);
        assert_eq!(app.ckey_view_key().as_deref(), Some("ck-account-1"));
        assert!(app.ckey_account_key("p-other").is_none(), "pick không được tạo binding");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_ckey_import_creates_provider_and_builtin_not_saved() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_ckey_import");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let opencode_json_path = opencode_dir.join("opencode.json");
        fs::write(&opencode_json_path, "{}").unwrap();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Tài khoản đang xem (import dùng AI key + binding của tài khoản này).
        app.ckey_config.profiles.push(crate::config::CkeyProfileEntry {
            id: "p1".to_string(),
            name: "Tài khoản 1".to_string(),
            key: "ck-account-1".to_string(),
        });
        app.ckey_config.active = Some("p1".to_string());

        // Giả lập dữ liệu CKey: 1 AI key active + 2 models (1 model sẽ bị xoá sau khi import)
        app.ckey_keys = vec![crate::ckey::CkeyAiKey {
            id: 4804,
            key_name: "Production".to_string(),
            api_key: "ck-prod-xxxxxxxxxxxxxxxx".to_string(),
            key_prefix: "ck-prod-xxxx".to_string(),
            is_active: true,
            created_at: 1778032800,
            created_at_text: "06/05/2026".to_string(),
        }];
        app.ckey_models = parse_ckey_models_fixture();
        app.open_ckey_import();

        // Chưa có provider "ckey" → cả 2 model đều unchecked (model mới)
        assert_eq!(app.ckey_import_list.len(), 2);
        assert!(app.ckey_import_list.iter().all(|(_, c, s, _, _)| !c && !s));

        // Chọn model đầu tiên để thêm
        if let Some(item) = app.ckey_import_list.get_mut(0) {
            item.1 = true;
        }
        app.execute_ckey_import().unwrap();

        // Provider "ckey" được tạo với model đã chọn + api_key từ key active
        let provider = app
            .config
            .provider
            .get(crate::ckey::CKEY_PRESET_ID)
            .expect("provider ckey");
        assert!(provider.models.contains_key("provider/gpt-demo"));
        assert!(!provider.models.contains_key("provider/gpt-removed"));
        assert_eq!(provider.options.api_key, "ck-prod-xxxxxxxxxxxxxxxx");
        assert_eq!(provider.options.base_url, crate::ckey::CKEY_LLM_BASE_URL);
        // Import phải gắn binding provider → tài khoản đang xem.
        assert_eq!(
            app.ckey_config
                .bindings
                .get(crate::ckey::CKEY_PRESET_ID)
                .map(String::as_str),
            Some("p1")
        );

        // Built-in ckey SAU IMPORT (có danh sách model khai báo rõ) phải được
        // giữ trong opencode.json — đó là cơ chế "cung cấp danh sách rõ ràng"
        // thay vì để opencode quét toàn bộ catalogue; nhưng KHOÁ phải bị xoá
        // khỏi file (khoá builtin sống ở auth.json).
        let saved_cfg: OpencodeConfig =
            serde_json::from_str(&fs::read_to_string(&opencode_json_path).unwrap()).unwrap();
        let saved_ckey = saved_cfg
            .provider
            .get(crate::ckey::CKEY_PRESET_ID)
            .expect("builtin ckey chọn model phải nằm trong opencode.json");
        assert!(
            saved_ckey.models.contains_key("provider/gpt-demo"),
            "danh sách model import phải được giữ"
        );
        assert!(
            saved_ckey.options.api_key.trim().is_empty(),
            "khoá builtin phải ở auth.json, không nằm trong opencode.json"
        );
        let share_dir = test_dir.join(".local").join("share").join("opencode");
        let auth_json_path = share_dir.join("auth.json");
        let saved_auth = fs::read_to_string(&auth_json_path).unwrap();
        assert!(saved_auth.contains("ckey"), "Built-in ckey phải nằm trong auth.json");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_ckey_import_removes_stale_models() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_ckey_stale");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let opencode_json_path = opencode_dir.join("opencode.json");

        // Provider "ckey" có sẵn model-a (sắp bị CKey xoá) + provider/gpt-demo (còn tồn tại)
        let mock_opencode = format!(
            r#"{{
                "provider": {{
                    "{}": {{
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "CKey",
                        "options": {{ "baseURL": "{}", "apiKey": "ck-prod-old" }},
                        "models": {{
                            "model-a": {{ "name": "model-a" }},
                            "provider/gpt-demo": {{ "name": "provider/gpt-demo" }}
                        }}
                    }}
                }}
            }}"#,
            crate::ckey::CKEY_PRESET_ID,
            crate::ckey::CKEY_LLM_BASE_URL
        );
        fs::write(&opencode_json_path, mock_opencode).unwrap();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Tài khoản đang xem (import cần active profile).
        app.ckey_config.profiles.push(crate::config::CkeyProfileEntry {
            id: "p1".to_string(),
            name: "Tài khoản 1".to_string(),
            key: "ck-account-1".to_string(),
        });
        app.ckey_config.active = Some("p1".to_string());

        // CKey hiện chỉ còn 1 model: provider/gpt-demo (model-a đã bị CKey xoá)
        let models = parse_ckey_models_fixture();
        app.ckey_models = vec![models[0].clone()];
        app.open_ckey_import();

        // Import list: provider/gpt-demo (checked, tồn tại) + model-a (stale, unchecked)
        assert_eq!(app.ckey_import_list.len(), 2);
        let stale = app.ckey_import_list.iter().find(|(_id, _, s, _, _)| *s).unwrap();
        assert_eq!(stale.0, "model-a");
        assert!(!stale.1, "model stale phải unchecked");

        app.execute_ckey_import().unwrap();

        let provider = app.config.provider.get(crate::ckey::CKEY_PRESET_ID).unwrap();
        assert!(
            !provider.models.contains_key("model-a"),
            "model-a bị CKey xoá phải bị xoá khỏi config sau khi đồng bộ"
        );
        assert!(
            provider.models.contains_key("provider/gpt-demo"),
            "model còn tồn tại phải được giữ"
        );
        assert!(
            !provider.models.contains_key("provider/gpt-removed"),
            "model mới unchecked không được thêm"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_scan_detects_and_removes_stale_models() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        let test_dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_home_stale_models");
        if test_dir.exists() {
            let _ = fs::remove_dir_all(&test_dir);
        }
        fs::create_dir_all(&test_dir).unwrap();

        // Override HOME, USERPROFILE and OPENCODE_TEST_HOME environment variables to isolate the test
        // SAFETY: This is a test function running in a controlled environment
        unsafe {
            std::env::set_var("HOME", &test_dir);
            std::env::set_var("USERPROFILE", &test_dir);
            std::env::set_var("OPENCODE_TEST_HOME", &test_dir);
        }

        let opencode_dir = test_dir.join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let opencode_json_path = opencode_dir.join("opencode.json");

        // Provider có 2 model: model-a (sắp bị provider xoá) và model-b (còn tồn tại)
        let mock_opencode = r#"{
            "provider": {
                "test_provider": {
                    "npm": "@ai-sdk/openai-compatible",
                    "name": "Test Provider",
                    "options": {
                        "baseURL": "https://example.com/v1",
                        "apiKey": "test-key"
                    },
                    "models": {
                        "model-a": { "name": "model-a" },
                        "model-b": { "name": "model-b" }
                    }
                }
            }
        }"#;
        fs::write(&opencode_json_path, mock_opencode).unwrap();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();
        let cfg = OpencodeConfig::load().unwrap();
        let auth = crate::config::AuthEntry::load_config().unwrap();
        let mut app = App::new(cfg, auth, tx);

        // Provider chỉ còn trả về model-b và model-c (model-a đã bị provider xoá)
        app.is_scanning = true;
        app.scanning_provider_id = "test_provider".to_string();
        app.handle_message(AppMessage::Scan {
            provider_id: "test_provider".to_string(),
            models: Ok(vec!["model-b".to_string(), "model-c".to_string()]),
        });

        // Danh sách quét phải chứa: model-b (checked), model-c (unchecked), model-a (stale, unchecked)
        assert_eq!(
            app.scanned_models.len(),
            3,
            "scanned_models phải gồm model-b, model-c và model-a stale"
        );
        let model_b = app.scanned_models.iter().find(|(id, _, _)| id == "model-b").unwrap();
        assert!(
            model_b.1,
            "model-b còn trên provider và đã có trong config → phải checked"
        );
        assert!(!model_b.2, "model-b không phải stale");
        let model_c = app.scanned_models.iter().find(|(id, _, _)| id == "model-c").unwrap();
        assert!(!model_c.1, "model-c chưa có trong config → unchecked");
        assert!(!model_c.2, "model-c không phải stale");
        let model_a = app.scanned_models.iter().find(|(id, _, _)| id == "model-a").unwrap();
        assert!(
            !model_a.1,
            "model-a bị provider xoá → mặc định unchecked để bị xoá khi đồng bộ"
        );
        assert!(model_a.2, "model-a phải được đánh dấu stale");

        // Đồng bộ: model-a bị xoá khỏi config, model-b giữ nguyên, model-c không thêm (unchecked)
        app.add_scanned_models().unwrap();
        let provider = app.config.provider.get("test_provider").unwrap();
        assert!(
            !provider.models.contains_key("model-a"),
            "model-a phải bị xoá khỏi config sau khi đồng bộ (provider đã xoá)"
        );
        assert!(provider.models.contains_key("model-b"), "model-b phải được giữ lại");
        assert!(
            !provider.models.contains_key("model-c"),
            "model-c unchecked nên không được thêm vào config"
        );

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);
    }
}

/// Test cơ chế ModelCaps — bản nháp capability của màn hình ModelCaps (case hy3).
#[cfg(test)]
mod caps_tests {
    use super::*;

    fn entry_with_caps() -> ModelEntry {
        ModelEntry {
            name: "hy3".to_string(),
            tool_call: Some(true),
            reasoning: Some(true),
            interleaved: Some(Interleaved::Object {
                field: "reasoning_content".to_string(),
            }),
            ..Default::default()
        }
    }

    /// Draft mở từ entry config phải mang đúng capability đang lưu.
    #[test]
    fn draft_doc_du_tu_entry() {
        let d = ModelCapsDraft::from_entry(&entry_with_caps());
        assert_eq!(d.tool_call, Some(true));
        assert_eq!(d.reasoning, Some(true));
        assert_eq!(d.interleaved_field, "reasoning_content");
        assert_eq!(d.summary(), "tool+reason+interleaved");
    }

    /// Ghi draft vào entry: chỉ field đã đặt mới bị đè — field None giữ nguyên
    /// giá trị cũ (round-trip an toàn).
    #[test]
    fn apply_khong_dong_bo_field_none() {
        let mut entry = entry_with_caps();
        // Draft chỉ đặt reasoning, còn lại để None và chưa chạm ô interleaved.
        let d = ModelCapsDraft {
            reasoning: Some(false),
            ..Default::default()
        };
        d.apply_to(&mut entry);
        assert_eq!(entry.reasoning, Some(false), "reasoning phải bị đè");
        assert_eq!(entry.tool_call, Some(true), "tool_call giữ nguyên");
        assert!(matches!(entry.interleaved, Some(Interleaved::Object { .. })));
    }

    /// Chưa chạm ô interleaved thì field cũ giữ nguyên; CHẠM rồi xoá trắng thì
    /// field bị gỡ khỏi config (phải có đường gỡ capability đã đặt).
    #[test]
    fn apply_touched_rong_goi_interleaved() {
        let mut entry = entry_with_caps();

        // Chưa touched: giữ nguyên.
        let d = ModelCapsDraft {
            interleaved_field: String::new(),
            interleaved_touched: false,
            ..Default::default()
        };
        d.apply_to(&mut entry);
        assert!(matches!(entry.interleaved, Some(Interleaved::Object { .. })));

        // Touched + rỗng: gỡ field.
        let d = ModelCapsDraft {
            interleaved_field: String::new(),
            interleaved_touched: true,
            ..Default::default()
        };
        d.apply_to(&mut entry);
        assert!(entry.interleaved.is_none(), "field phải bị gỡ");

        // Touched + giá trị mới: đè.
        let d = ModelCapsDraft {
            interleaved_field: "reasoning".to_string(),
            interleaved_touched: true,
            ..Default::default()
        };
        d.apply_to(&mut entry);
        match entry.interleaved {
            Some(Interleaved::Object { field }) => assert_eq!(field, "reasoning"),
            other => panic!("interleaved sai: {:?}", other),
        }
    }

    /// Entry trống (model mới) + draft đầy đủ = config hoàn chỉnh cho hy3.
    #[test]
    fn apply_len_model_moi() {
        let mut entry = ModelEntry::default();
        let d = ModelCapsDraft {
            tool_call: Some(true),
            reasoning: Some(true),
            interleaved_field: "reasoning_content".to_string(),
            interleaved_touched: true,
        };
        d.apply_to(&mut entry);
        assert_eq!(entry.tool_call, Some(true));
        assert_eq!(entry.reasoning, Some(true));
        match entry.interleaved {
            Some(Interleaved::Object { field }) => assert_eq!(field, "reasoning_content"),
            other => panic!("interleaved sai: {:?}", other),
        }
    }

    /// Interleaved dạng chuỗi ("reasoning_content") của schema cũ cũng đọc được
    /// vào draft (from_entry chấp nhận cả 3 dạng).
    #[test]
    fn draft_doc_duoc_interleaved_dang_chuoi() {
        let mut entry = ModelEntry::default();
        entry.interleaved = Some(Interleaved::Field("reasoning".to_string()));
        let d = ModelCapsDraft::from_entry(&entry);
        assert_eq!(d.interleaved_field, "reasoning");
    }
}
