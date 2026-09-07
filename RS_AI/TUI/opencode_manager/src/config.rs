use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelLimit {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<u64>,
    // Schema opencode còn có `limit.input`; giữ để round-trip không mất.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelModalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

/// `interleaved` theo schema opencode (`https://opencode.ai/config.json`,
/// `ProviderConfig.models[].interleaved`): bool | "reasoning_content" | { field }.
/// Model reasoning (vd `hy3` trả reasoning qua field `reasoning_content`) bắt buộc
/// khai báo dạng này, nếu không OpenCode parse sai stream chunk reasoning.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Interleaved {
    Flag(bool),
    Field(String),
    Object { field: String },
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ModelEntry {
    // Schema cho phép entry rỗng (`"model-id": {}`) — `default` để load được.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<ModelLimit>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modalities: Option<ModelModalities>,
    // Các trường khả năng (capability) theo schema opencode. Thiếu chúng,
    // OpenCode coi model là thường → request sai shape (mất tool_call/reasoning).
    // Option + skip_serializing: không set thì không ghi, file gọn như cũ.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temperature: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub attachment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub interleaved: Option<Interleaved>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProviderOptions {
    #[serde(rename = "baseURL", default)]
    pub base_url: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    // Header tuỳ biến (vd Helicone `Helicone-Cache-Enabled`) theo docs providers.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Provider {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub npm: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub options: ProviderOptions,
    #[serde(default)]
    pub models: HashMap<String, ModelEntry>,
    // Ẩn model khỏi picker mà không cần xoá (docs: blacklist/whitelist).
    // Trước đây struct không khai báo → load→save làm mất hai field này.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub whitelist: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blacklist: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpencodeConfig {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: HashMap<String, Provider>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthEntry {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub key: String,
}

pub type AuthConfig = HashMap<String, AuthEntry>;

/// Endpoint hợp lệ đứng sau segment `v<digit>` (whitelist). Chỉ cắt path khi toàn bộ
/// phần sau `v<digit>` nằm trong danh sách này — tránh cắt bừa Groq `/openai/v1`,
/// OpenRouter `/api/v1`, Google `/v1beta`, hoặc suffix lạ.
const BASE_URL_WHITELIST_ENDPOINTS: &[&str] = &[
    "chat/completions",
    "completions",
    "chat",
    "models",
    "embeddings",
    "responses",
    "messages",
    "generate",
    "predictions",
    "audio/transcriptions",
    "images/generations",
    "moderations",
    "fine-tunes",
    "runs",
    "assistants",
    "threads",
    "search",
    "edits",
];

/// Kiểm tra segment có dạng `v<digit>` (regex `^v\d+$`); `v1beta` KHÔNG khớp.
fn is_version_segment(seg: &str) -> bool {
    match seg.as_bytes().split_first() {
        Some((b'v', rest)) => !rest.is_empty() && rest.iter().all(|b| b.is_ascii_digit()),
        _ => false,
    }
}

/// Tự sửa base_url nhập thừa path (vd `https://api.inceptionlabs.ai/v1/chat/completions`
/// → `https://api.inceptionlabs.ai/v1`).
///
/// - trim + trim_end_matches('/'); không chứa `://`:
///   - chuỗi bắt đầu bằng `/` (path thuần tuý) → vẫn chuẩn hoá phần path;
///   - trường hợp khác (vd `api.example.com/v1/models`) → trả về nguyên.
/// - Cắt path về "/" + các segment đến hết segment `v<digit>` CHỈ KHI segment đó không
///   phải segment cuối VÀ phần sau (join "/") thuộc whitelist endpoint.
pub fn normalize_base_url(raw: &str) -> String {
    let raw = raw.trim().trim_end_matches('/');

    // Tách prefix (scheme://host) khỏi path
    let (prefix, path) = if let Some((scheme, rest)) = raw.split_once("://") {
        match rest.find('/') {
            Some(idx) => (format!("{}://{}", scheme, &rest[..idx]), &rest[idx..]),
            None => return raw.to_string(), // chỉ có host, không có path
        }
    } else if raw.starts_with('/') {
        // Path thuần tuý → chuẩn hoá phần path, không có prefix
        (String::new(), raw)
    } else {
        // Không có scheme và không phải path thuần tuý → không đủ thông tin để sửa
        return raw.to_string();
    };

    // Phân path thành segments, bỏ segment rỗng
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return raw.to_string();
    }

    // Tìm segment đầu tiên khớp v<digit>
    let Some(v_idx) = segments.iter().position(|s| is_version_segment(s)) else {
        return raw.to_string();
    };

    // Segment v<digit> phải KHÔNG phải segment cuối
    if v_idx + 1 >= segments.len() {
        return raw.to_string();
    }

    // Phần sau (join "/") phải thuộc whitelist endpoint
    let rest_path = segments[v_idx + 1..].join("/");
    if !BASE_URL_WHITELIST_ENDPOINTS.contains(&rest_path.as_str()) {
        return raw.to_string();
    }

    // path mới = "/" + các segment đến hết segment v<digit>
    let new_path = format!("/{}", segments[..=v_idx].join("/"));
    if prefix.is_empty() {
        new_path
    } else {
        format!("{}{}", prefix, new_path)
    }
}

pub fn get_home_dir() -> Option<PathBuf> {
    std::env::var("OPENCODE_TEST_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(dirs::home_dir)
}

impl OpencodeConfig {
    pub fn file_path() -> PathBuf {
        let home = get_home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".config").join("opencode").join("opencode.json")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(OpencodeConfig {
                schema: Some("https://opencode.ai/config.json".to_string()),
                model: None,
                provider: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&path).map_err(|e| format!("Không thể đọc file opencode.json: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON opencode.json: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path();

        // Tạo thư mục cha nếu chưa có
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Không thể tạo thư mục cấu hình: {}", e))?;
        }

        // Tạo bản backup
        if path.exists() {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = path.with_extension(format!("json.bak_{}", timestamp));
            let _ = fs::copy(&path, &backup_path);
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| format!("Không thể serialize cấu hình: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Không thể ghi file opencode.json: {}", e))?;

        Ok(())
    }
}

impl AuthEntry {
    pub fn file_path() -> PathBuf {
        let home = get_home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".local").join("share").join("opencode").join("auth.json")
    }

    pub fn load_config() -> Result<AuthConfig, String> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&path).map_err(|e| format!("Không thể đọc file auth.json: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON auth.json: {}", e))
    }

    pub fn save_config(config: &AuthConfig) -> Result<(), String> {
        let path = Self::file_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Không thể tạo thư mục chứa auth.json: {}", e))?;
        }

        // Tạo bản backup
        if path.exists() {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = path.with_extension(format!("json.bak_{}", timestamp));
            let _ = fs::copy(&path, &backup_path);
        }

        let content =
            serde_json::to_string_pretty(config).map_err(|e| format!("Không thể serialize auth.json: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Không thể ghi file auth.json: {}", e))?;

        Ok(())
    }
}

/// Một tài khoản CKey (account key lấy ở trang Profile ckey.vn).
/// Người dùng có thể lưu NHIỀU tài khoản (profile) và chuyển đổi nhanh.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CkeyProfileEntry {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub key: String,
}

/// Cấu hình tài khoản CKey: danh sách profile + profile đang dùng + gán
/// provider → profile.
///
/// File `~/.config/opencode-manager/ckey.json` (KHÔNG nằm trong
/// opencode.json/auth.json):
/// ```json
/// {
///   "profiles": [ { "id": "p1", "name": "Tài khoản chính", "key": "..." } ],
///   "active": "p1",
///   "bindings": { "ckey": "p1" }
/// }
/// ```
/// Account key là của TÀI KHOẢN ckey.vn (gọi API quản lý) — không gắn với
/// provider nào cả; provider chỉ được GẮN (binding) vào một profile khi import
/// để biết dùng AI key của tài khoản nào.
///
/// Migration tự động khi đọc (không ghi lại cho tới lần lưu đầu tiên):
/// - `{ account_key }` (rất cũ) → 1 profile, gán cho "ckey".
/// - `{ endpoint, accounts: [{name, key}] }` (cũ) → account đầu thành 1 profile.
/// - `{ accounts: { provider_id: key } }` → mỗi entry một profile (đặt tên theo
///   provider_id), giữ binding provider cũ, active = profile đầu.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct CkeyConfig {
    #[serde(default)]
    pub profiles: Vec<CkeyProfileEntry>,
    /// Id profile đang dùng (dashboard/usage/deposit xem tài khoản này).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    /// provider_id → profile id (provider import model bằng AI key của profile nào).
    #[serde(default)]
    pub bindings: HashMap<String, String>,
}

impl CkeyConfig {
    /// Vị trí MỚI: `~/.config/opencode-manager/ckey.json` (dữ liệu riêng của
    /// manager — tách khỏi thư mục của opencode).
    pub fn file_path() -> PathBuf {
        crate::storage::manager_data_path("ckey.json")
    }

    /// Vị trí CŨ (trước khi tách thư mục) — chỉ còn dùng cho migration.
    fn legacy_file_path() -> PathBuf {
        let home = get_home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".config").join("opencode").join("ckey.json")
    }

    /// Đọc ckey.json (kèm migration vị trí + định dạng cũ — xem doc struct).
    pub fn load() -> Result<Self, String> {
        // Migration không xoá: copy bản cũ sang vị trí mới, bản cũ thành .legacy.
        crate::storage::migrate_legacy(&Self::legacy_file_path(), &Self::file_path());

        let path = Self::file_path();
        if !path.exists() {
            return Ok(CkeyConfig::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| format!("Không thể đọc file ckey.json: {}", e))?;

        let raw: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON ckey.json: {}", e))?;

        Self::from_json(raw)
    }

    /// Parse + migration từ JSON đã đọc (tách riêng để unit test được).
    pub fn from_json(raw: serde_json::Value) -> Result<Self, String> {
        // Định dạng MỚI: có mảng `profiles`.
        if raw.get("profiles").and_then(|v| v.as_array()).is_some() {
            return serde_json::from_value(raw).map_err(|e| format!("Lỗi parse ckey.json (profiles): {}", e));
        }

        // ----- Migration từ định dạng cũ -----
        let mut cfg = CkeyConfig::default();

        // 1. `accounts` dạng map { provider_id: key } → mỗi entry một profile.
        if let Some(accounts) = raw.get("accounts") {
            if let Ok(map) = serde_json::from_value::<std::collections::BTreeMap<String, String>>(accounts.clone()) {
                for (pid, key) in map {
                    let key = key.trim().to_string();
                    if key.is_empty() {
                        continue;
                    }
                    let id = cfg.next_profile_id();
                    cfg.bindings.insert(pid.clone(), id.clone());
                    cfg.profiles.push(CkeyProfileEntry { id, name: pid, key });
                }
                cfg.active = cfg.profiles.first().map(|p| p.id.clone());
                return Ok(cfg);
            }

            // 2. `accounts` dạng mảng [{name, key}] (rất cũ) → account đầu cho "ckey".
            #[derive(Deserialize)]
            struct OldAccount {
                #[serde(default)]
                key: String,
            }
            if let Ok(list) = serde_json::from_value::<Vec<OldAccount>>(accounts.clone())
                && let Some(first) = list.into_iter().find(|a| !a.key.trim().is_empty())
            {
                let id = cfg.next_profile_id();
                cfg.bindings.insert(crate::ckey::CKEY_PRESET_ID.to_string(), id.clone());
                cfg.profiles.push(CkeyProfileEntry {
                    id,
                    name: "CKey".to_string(),
                    key: first.key.trim().to_string(),
                });
                cfg.active = cfg.profiles.first().map(|p| p.id.clone());
                return Ok(cfg);
            }

            return Err(
                "Không nhận diện được định dạng ckey.json (field 'accounts') — không tự sửa để tránh mất dữ liệu."
                    .to_string(),
            );
        }

        // 3. Field `account_key` lẻ (file rất cũ).
        if let Some(key) = raw.get("account_key").and_then(|v| v.as_str()) {
            let key = key.trim().to_string();
            if !key.is_empty() {
                let id = cfg.next_profile_id();
                cfg.bindings.insert(crate::ckey::CKEY_PRESET_ID.to_string(), id.clone());
                cfg.profiles.push(CkeyProfileEntry {
                    id,
                    name: "CKey".to_string(),
                    key,
                });
                cfg.active = cfg.profiles.first().map(|p| p.id.clone());
                return Ok(cfg);
            }
        }

        Ok(cfg)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path();

        // Backup xoay vòng (giữ N bản) + ghi atomic — crash không để lại
        // file rỗng/nửa vời, và luôn lùi được vài bước.
        crate::storage::backup_rotate(&path, crate::storage::BACKUP_KEEP);

        let content =
            serde_json::to_string_pretty(self).map_err(|e| format!("Không thể serialize ckey.json: {}", e))?;

        crate::storage::atomic_write(&path, &content).map_err(|e| format!("Không thể ghi file ckey.json: {}", e))
    }

    /// Tìm profile theo id.
    pub fn profile(&self, id: &str) -> Option<&CkeyProfileEntry> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Profile đang dùng (active; nếu id active hỏng thì rơi về profile đầu).
    /// TUI tự set active theo binding nên chưa gọi; crate GUI dùng cho màn
    /// quản lý profile trực tiếp.
    #[allow(dead_code)]
    pub fn active_profile(&self) -> Option<&CkeyProfileEntry> {
        self.active
            .as_deref()
            .and_then(|id| self.profile(id))
            .or_else(|| self.profiles.first())
    }

    /// Account key của một profile. Crate GUI gọi cho các lệnh theo profile;
    /// TUI dùng active_profile().key trực tiếp.
    #[allow(dead_code)]
    pub fn key_of(&self, profile_id: &str) -> Option<String> {
        self.profile(profile_id)
            .map(|p| p.key.clone())
            .filter(|k| !k.trim().is_empty())
    }

    /// Account key của TÀI KHOẢN mà provider này được gắn (binding).
    /// Crate GUI gọi (account_key_of); TUI chỉ đọc binding lúc import.
    #[allow(dead_code)]
    pub fn account_key(&self, provider_id: &str) -> Option<String> {
        let profile_id = self.bindings.get(provider_id)?;
        self.key_of(profile_id)
    }

    /// Sinh id profile mới chưa dùng: p1, p2, …
    pub fn next_profile_id(&self) -> String {
        let mut n = 1usize;
        loop {
            let candidate = format!("p{n}");
            if self.profile(&candidate).is_none() {
                return candidate;
            }
            n += 1;
        }
    }

    /// Dọn dữ liệu sau khi xoá profile: bỏ binding trỏ tới nó, nếu nó đang
    /// active thì chuyển active sang profile đầu còn lại. Crate GUI gọi (TUI
    /// chưa có nút xoá profile).
    #[allow(dead_code)]
    pub fn remove_profile(&mut self, profile_id: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.id != profile_id);
        if self.profiles.len() == before {
            return false;
        }
        self.bindings.retain(|_, v| v != profile_id);
        if self.active.as_deref() == Some(profile_id) {
            self.active = self.profiles.first().map(|p| p.id.clone());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::OpencodeConfig;
    use super::normalize_base_url;

    #[test]
    fn test_normalize_base_url() {
        // Có scheme + v<digit> + suffix whitelist → cắt về đến segment v<digit>
        assert_eq!(
            normalize_base_url("https://api.inceptionlabs.ai/v1/chat/completions"),
            "https://api.inceptionlabs.ai/v1"
        );
        assert_eq!(
            normalize_base_url("https://x.ai/v2/chat/completions"),
            "https://x.ai/v2"
        );

        // Path thuần tuý → chuẩn hoá phần path
        assert_eq!(normalize_base_url("/v1/models"), "/v1");
        assert_eq!(normalize_base_url("/v1/embeddings"), "/v1");

        // v<digit> là segment cuối → giữ nguyên
        assert_eq!(normalize_base_url("https://api.x.com/v1"), "https://api.x.com/v1");
        assert_eq!(
            normalize_base_url("https://api.groq.com/openai/v1"),
            "https://api.groq.com/openai/v1"
        );
        assert_eq!(
            normalize_base_url("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1"
        );

        // v1beta không khớp v<digit> → giữ nguyên
        assert_eq!(
            normalize_base_url("https://api.google.com/v1beta"),
            "https://api.google.com/v1beta"
        );

        // Không scheme + có host → giữ nguyên
        assert_eq!(
            normalize_base_url("api.example.com/v1/models"),
            "api.example.com/v1/models"
        );

        // Suffix lạ (không thuộc whitelist) → giữ nguyên
        assert_eq!(normalize_base_url("https://x.ai/v1/xyz"), "https://x.ai/v1/xyz");

        // trim + trailing slash được xử lý trước khi so sánh
        assert_eq!(
            normalize_base_url("  https://api.inceptionlabs.ai/v1/chat/completions/  "),
            "https://api.inceptionlabs.ai/v1"
        );
    }

    /// Hồi quy case `hy3` (case-opencode-custom4-hy3.md): load→save không được
    /// làm mất trường khả năng model (tool_call/reasoning/interleaved/id) và
    /// whitelist/blacklist của provider.
    #[test]
    fn round_trip_giu_capability_model_va_list_provider() {
        let raw = r#"{
            "provider": {
                "custom_4": {
                    "npm": "@ai-sdk/openai-compatible",
                    "name": "TEMP",
                    "options": { "baseURL": "https://router.nexaworks.web.id/v1", "apiKey": "sk-x" },
                    "blacklist": ["old-model"],
                    "models": {
                        "hy3": {
                            "id": "hy3",
                            "name": "hy3",
                            "tool_call": true,
                            "reasoning": true,
                            "temperature": true,
                            "attachment": false,
                            "interleaved": { "field": "reasoning_content" },
                            "limit": { "context": 64000, "output": 8000 },
                            "modalities": { "input": ["text"], "output": ["text"] }
                        },
                        "plain": {}
                    }
                }
            }
        }"#;

        let cfg: OpencodeConfig = serde_json::from_str(raw).expect("parse config mẫu hy3");
        let hy3 = &cfg.provider["custom_4"].models["hy3"];
        assert_eq!(hy3.tool_call, Some(true));
        assert_eq!(hy3.reasoning, Some(true));
        assert_eq!(hy3.id.as_deref(), Some("hy3"));
        // Entry rỗng `{}` theo docs phải load được (name mặc định "").
        assert_eq!(cfg.provider["custom_4"].models["plain"].name, "");

        let again: OpencodeConfig = serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        let hy3b = &again.provider["custom_4"].models["hy3"];
        assert_eq!(hy3b.tool_call, Some(true), "tool_call mất sau round-trip");
        assert_eq!(hy3b.reasoning, Some(true), "reasoning mất sau round-trip");
        // interleaved dạng object { field } phải giữ nguyên.
        let field = match &hy3b.interleaved {
            Some(super::Interleaved::Object { field }) => field.clone(),
            other => panic!("interleaved sai sau round-trip: {:?}", other),
        };
        assert_eq!(field, "reasoning_content");
        assert_eq!(
            again.provider["custom_4"].blacklist.as_deref(),
            Some(&["old-model".to_string()][..]),
            "blacklist mất sau round-trip"
        );
    }

    /// `interleaved` chấp nhận cả 3 dạng của schema: bool, chuỗi, object.
    #[test]
    fn parse_duoc_ca_3_dang_interleaved() {
        let b: super::ModelEntry = serde_json::from_str(r#"{"name":"a","interleaved":true}"#).unwrap();
        assert!(matches!(b.interleaved, Some(super::Interleaved::Flag(true))));
        let s: super::ModelEntry = serde_json::from_str(r#"{"name":"a","interleaved":"reasoning_content"}"#).unwrap();
        assert!(matches!(s.interleaved, Some(super::Interleaved::Field(_))));
        let o: super::ModelEntry = serde_json::from_str(r#"{"name":"a","interleaved":{"field":"reasoning"}}"#).unwrap();
        assert!(matches!(o.interleaved, Some(super::Interleaved::Object { .. })));
    }
}

#[cfg(test)]
mod ckey_profile_tests {
    use super::*;

    /// File mới (profiles/active/bindings) round-trip nguyên vẹn.
    #[test]
    fn roundtrip_dinh_dang_profile() {
        let cfg = CkeyConfig {
            profiles: vec![
                CkeyProfileEntry {
                    id: "p1".into(),
                    name: "Chính".into(),
                    key: "k1".into(),
                },
                CkeyProfileEntry {
                    id: "p2".into(),
                    name: "Phụ".into(),
                    key: "k2".into(),
                },
            ],
            active: Some("p2".into()),
            bindings: HashMap::from([("ckey".to_string(), "p1".to_string())]),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: CkeyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
        // active_profile tôn trọng lựa chọn, không phải profile đầu.
        assert_eq!(back.active_profile().map(|p| p.id.as_str()), Some("p2"));
    }

    /// Migration map cũ { accounts: {pid: key} } → profile + binding tương ứng.
    #[test]
    fn migration_map_cu_sang_profile() {
        let raw = r#"{ "accounts": { "ckey": "k1", "X7K2P9": "k2" } }"#;
        let cfg: CkeyConfig = CkeyConfig::load_from_str_for_test(raw).expect("phải migrate được");
        assert_eq!(cfg.profiles.len(), 2);
        assert_eq!(cfg.account_key("ckey").as_deref(), Some("k1"));
        assert_eq!(cfg.account_key("X7K2P9").as_deref(), Some("k2"));
        // active = profile đầu (theo thứ tự tên khi migrate).
        assert_eq!(cfg.active_profile().map(|p| p.name.as_str()), Some("X7K2P9"));
        // next id không đụng profile đang có.
        assert_eq!(cfg.next_profile_id(), "p3");
    }

    /// Migration { account_key } (rất cũ) và mảng [{name,key}] (cũ).
    #[test]
    fn migration_dang_rat_cu() {
        let cfg = CkeyConfig::load_from_str_for_test(r#"{"account_key":"ck-xxx"}"#).unwrap();
        assert_eq!(cfg.profiles.len(), 1);
        assert_eq!(cfg.account_key("ckey").as_deref(), Some("ck-xxx"));

        let cfg2 =
            CkeyConfig::load_from_str_for_test(r#"{"accounts":[{"name":"a","key":"k1"},{"name":"b","key":"k2"}]}"#)
                .unwrap();
        assert_eq!(cfg2.profiles.len(), 1, "dạng cũ chỉ lấy account đầu");
        assert_eq!(cfg2.account_key("ckey").as_deref(), Some("k1"));
    }

    /// File rỗng/lạc định dạng accounts → lỗi rõ ràng, không mất dữ liệu.
    #[test]
    fn file_hoac_dang_le_bao_loi() {
        assert!(CkeyConfig::load_from_str_for_test(r#"{"accounts":"khong_phai_map"}"#).is_err());
        // Không có gì (object rỗng) → config rỗng hợp lệ.
        let cfg = CkeyConfig::load_from_str_for_test("{}").unwrap();
        assert!(cfg.profiles.is_empty());
        assert!(cfg.active_profile().is_none());
    }

    /// remove_profile dọn binding + đổi active.
    #[test]
    fn remove_profile_don_binding_va_active() {
        let mut cfg = CkeyConfig {
            profiles: vec![
                CkeyProfileEntry {
                    id: "p1".into(),
                    name: "A".into(),
                    key: "k1".into(),
                },
                CkeyProfileEntry {
                    id: "p2".into(),
                    name: "B".into(),
                    key: "k2".into(),
                },
            ],
            active: Some("p2".into()),
            bindings: HashMap::from([
                ("ckey".to_string(), "p2".to_string()),
                ("other".to_string(), "p1".to_string()),
            ]),
        };
        assert!(cfg.remove_profile("p2"));
        // Binding trỏ tới p2 bị bỏ; p1 giữ nguyên.
        assert!(!cfg.bindings.contains_key("ckey"));
        assert_eq!(cfg.bindings["other"], "p1");
        // active rơi về profile đầu còn lại.
        assert_eq!(cfg.active_profile().map(|p| p.id.as_str()), Some("p1"));
        assert!(!cfg.remove_profile("khong-ton-tai"), "xoá id lạ → false");
    }
}

#[cfg(test)]
impl CkeyConfig {
    /// Parse từ chuỗi JSON cho test — đi QUA from_json để test đúng luồng
    /// migration (deserialize trực tiếp sẽ bỏ hết các nhánh migrate).
    fn load_from_str_for_test(s: &str) -> Result<Self, String> {
        let raw: serde_json::Value = serde_json::from_str(s).map_err(|e| format!("parse lỗi: {e}"))?;
        Self::from_json(raw)
    }
}
