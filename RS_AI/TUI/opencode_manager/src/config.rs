use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelLimit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelModalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<ModelLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<ModelModalities>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProviderOptions {
    #[serde(rename = "baseURL", default)]
    pub base_url: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub options: ProviderOptions,
    #[serde(default)]
    pub models: HashMap<String, ModelEntry>,
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

/// Cấu hình tài khoản CKey (account key từ trang Profile ckey.vn).
/// Map: provider_id → account_key. Mỗi provider CKey dùng đúng 1 account key.
/// Lưu riêng tại ~/.config/opencode/ckey.json — KHÔNG nằm trong opencode.json/auth.json.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CkeyConfig {
    #[serde(default)]
    pub accounts: HashMap<String, String>,
}

impl CkeyConfig {
    pub fn file_path() -> PathBuf {
        let home = get_home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".config").join("opencode").join("ckey.json")
    }

    /// Đọc ckey.json. Hỗ trợ migration từ file cũ:
    /// - `{ account_key }` (rất cũ) hoặc `{ endpoint, accounts: [{name, key}] }` (cũ)
    ///   → gán account đầu tiên cho provider "ckey" (CKEY_PRESET_ID).
    /// - File mới: `{ accounts: { provider_id: account_key } }`.
    pub fn load() -> Result<Self, String> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(CkeyConfig::default());
        }

        let content =
            fs::read_to_string(&path).map_err(|e| format!("Không thể đọc file ckey.json: {}", e))?;

        // Raw đọc được cả field cũ (account_key) lẫn field mới (accounts map).
        #[derive(Deserialize)]
        struct CkeyFileRaw {
            #[serde(default)]
            account_key: String,
            #[serde(default)]
            accounts: serde_json::Value,
        }

        let raw: CkeyFileRaw =
            serde_json::from_str(&content).map_err(|e| format!("Lỗi parse JSON ckey.json: {}", e))?;

        let mut accounts = HashMap::new();

        // 1. Thử parse dạng MỚI: accounts = map { provider_id: account_key }.
        if let Ok(map) = serde_json::from_value::<HashMap<String, String>>(raw.accounts.clone()) {
            accounts = map;
        } else if raw.accounts.is_null() {
            // Không có field accounts → để trống (không mất dữ liệu).
        } else {
            // 2. Dạng cũ: accounts = mảng [{name, key}] → lấy account đầu tiên cho "ckey".
            #[derive(Deserialize)]
            struct OldCkeyAccount {
                #[serde(default)]
                key: String,
            }
            if let Ok(old_list) = serde_json::from_value::<Vec<OldCkeyAccount>>(raw.accounts.clone())
                && let Some(first) = old_list.into_iter().next()
                && !first.key.trim().is_empty()
            {
                accounts.insert(
                    crate::ckey::CKEY_PRESET_ID.to_string(),
                    first.key.trim().to_string(),
                );
            } else {
                return Err(
                    "Không nhận diện được định dạng ckey.json (field 'accounts') — không tự sửa để tránh mất dữ liệu.".to_string(),
                );
            }
        }

        // 3. Field account_key lẻ (file rất cũ) → migrate nếu chưa có "ckey".
        if !raw.account_key.trim().is_empty() && !accounts.contains_key(crate::ckey::CKEY_PRESET_ID) {
            accounts.insert(
                crate::ckey::CKEY_PRESET_ID.to_string(),
                raw.account_key.trim().to_string(),
            );
        }

        Ok(CkeyConfig { accounts })
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Không thể tạo thư mục ckey.json: {}", e))?;
        }

        // Tạo bản backup nếu đã tồn tại
        if path.exists() {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = path.with_extension(format!("json.bak_{}", timestamp));
            let _ = fs::copy(&path, &backup_path);
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Không thể serialize ckey.json: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Không thể ghi file ckey.json: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
}
