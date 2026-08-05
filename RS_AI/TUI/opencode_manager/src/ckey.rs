use reqwest::Client;
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================
// CKey API Client (https://ckey.vn/docs)
// - Base URL quản lý tài khoản: từ config (endpoint, vd https://ckey.vn)  (?key=<account_key>)
// - LLM OpenAI-compatible:      https://api.xah.io/v1 (AI key dạng ck-...) — preset/import
// ============================================================

pub const CKEY_LLM_BASE_URL: &str = "https://api.xah.io/v1";
pub const CKEY_MANAGE_API_BASE: &str = "https://ckey.vn";
pub const CKEY_PRESET_ID: &str = "ckey";

/// Tạo tên provider ngẫu nhiên 6 ký tự alnum (vd X7K2P9), unique so với `existing`.
/// Không dùng crate rand: hash DefaultHasher + SystemTime nanos + bộ đếm thử lại để
/// đảm bảo luôn khác nhau ngay cả khi gọi nhiều lần trong cùng nanosecond.
pub fn generate_provider_name(existing: &HashSet<String>) -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut attempt: u64 = 0;
    loop {
        let mut hasher = DefaultHasher::new();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        nanos.hash(&mut hasher);
        attempt.hash(&mut hasher);
        attempt += 1;
        let mut h = hasher.finish();
        let mut name = String::with_capacity(6);
        for _ in 0..6 {
            name.push(CHARS[(h % CHARS.len() as u64) as usize] as char);
            h = h.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        }
        if !existing.contains(&name) {
            return name;
        }
    }
}

// ---------- Structs dữ liệu (deserialize từ response) ----------

#[derive(Debug, Clone, Deserialize)]
pub struct CkeyProfile {
    pub username: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub balance: String,
    #[serde(default)]
    pub balance_raw: f64,
    #[serde(default)]
    pub created_at: String,
}

// Giữ đầy đủ field từ API (có thể hiển thị thêm sau); chưa dùng hết nên allow dead_code.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CkeyModel {
    pub public_name: String,
    pub display_name: String,
    pub model_name: String,
    pub provider_username: String,
    #[serde(default)]
    pub is_provider_model: bool,
    #[serde(default)]
    pub input_price_per_million_vnd: f64,
    #[serde(default)]
    pub output_price_per_million_vnd: f64,
    #[serde(default)]
    pub price_per_request_vnd: f64,
    #[serde(default)]
    pub min_charge_per_request_vnd: f64,
    #[serde(default)]
    pub cache_enabled: bool,
    #[serde(default)]
    pub cache_read_price_per_million_vnd: f64,
    #[serde(default)]
    pub cache_write_price_per_million_vnd: f64,
    #[serde(default)]
    pub request_rate_limit_per_minute: u64,
    #[serde(default)]
    pub max_output_tokens_limit: u64,
    #[serde(default)]
    pub context_tokens_limit: u64,
    #[serde(default)]
    pub supported_paths: Vec<String>,
}

// Giữ đầy đủ field từ API; chỉ api_key/is_active được dùng cho import, số còn lại giữ để parse.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CkeyAiKey {
    pub id: u64,
    #[serde(default)]
    pub key_name: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub key_prefix: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub created_at_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CkeyUsageStats {
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub success_requests: u64,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
    #[allow(dead_code)]
    #[serde(default)]
    pub charged_vnd: f64,
    #[serde(default)]
    pub charged_vnd_text: String,
}

// Giữ đầy đủ field từ API; một số trường chưa hiển thị trên UI.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct CkeyUsageItem {
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub model_name: String,
    #[serde(default)]
    pub request_path: String,
    #[serde(default)]
    pub http_status: u64,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub charged_vnd: f64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub created_at_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CkeyPagination {
    #[serde(default)]
    pub page: u64,
    #[serde(default)]
    pub total_pages: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CkeyUsagePage {
    #[serde(default)]
    pub items: Vec<CkeyUsageItem>,
    #[serde(default)]
    pub pagination: Option<CkeyPagination>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CkeyModelList {
    #[serde(default)]
    pub models: Vec<CkeyModel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CkeyKeyList {
    #[serde(default)]
    pub items: Vec<CkeyAiKey>,
}

// ---------- Wrapper chung của CKey: { success, status, message, data } ----------

#[derive(Debug, Deserialize)]
struct CkeyWrapped<T> {
    #[serde(default)]
    message: Option<String>,
    data: Option<T>,
}

// Hàm parse thuần (không mạng) để unit test được từ fixture JSON.
pub fn parse_wrapped<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, String> {
    let wrapped: CkeyWrapped<T> =
        serde_json::from_str(body).map_err(|e| format!("Lỗi parse JSON từ CKey: {}", e))?;
    match wrapped.data {
        Some(data) => Ok(data),
        None => {
            let msg = wrapped.message.unwrap_or_else(|| "CKey trả về response rỗng".to_string());
            Err(format!("CKey lỗi: {}", msg))
        }
    }
}

/// Map HTTP status + body lỗi thành thông báo thân thiện (giống ApiStatus).
pub fn friendly_error(status: u16, body: &str) -> String {
    if status == 401 || status == 403 {
        return "Sai hoặc thiếu account key CKey (401/403). Kiểm tra lại key trong Profile.".to_string();
    }
    if status == 402 {
        return "Tài khoản CKey hết tiền/quota (402). Vui lòng nạp tiền.".to_string();
    }
    // Thử đọc message từ body lỗi dạng {success:false,message:...}
    if let Some(msg) = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from))
    {
        return format!("CKey lỗi HTTP {}: {}", status, msg);
    }
    format!("CKey lỗi HTTP {}", status)
}

// ---------- Client ----------

pub struct CkeyClient {
    client: Client,
    endpoint: String,
}

impl CkeyClient {
    pub fn new(endpoint: &str) -> Self {
        CkeyClient {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            endpoint: endpoint.trim().trim_end_matches('/').to_string(),
        }
    }

    /// Gọi GET tới API quản lý CKey với params dạng query (key, page, ...).
    async fn get_query<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, String> {
        if self.endpoint.is_empty() {
            return Err("Vui lòng nhập endpoint".to_string());
        }
        let url = format!("{}{}", self.endpoint, path);
        let mut req = self.client.get(&url);
        for (name, value) in params {
            req = req.query(&[(name, value.as_str())]);
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                "Kết nối CKey quá hạn (Timeout)".to_string()
            } else {
                format!("Lỗi kết nối CKey: {}", e)
            }
        })?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(friendly_error(status.as_u16(), &body));
        }
        parse_wrapped(&body)
    }

    pub async fn fetch_profile(&self, account_key: &str) -> Result<CkeyProfile, String> {
        // Response: data.profile = {...} → parse qua wrapper rồi trả profile.
        #[derive(Deserialize)]
        struct ProfileWrap {
            profile: CkeyProfile,
        }
        let wrap: ProfileWrap = self
            .get_query("/api/profile", &[("key", account_key.to_string())])
            .await?;
        Ok(wrap.profile)
    }

    pub async fn fetch_models(&self, account_key: &str) -> Result<Vec<CkeyModel>, String> {
        let list: CkeyModelList = self
            .get_query("/api/llm/models", &[("key", account_key.to_string())])
            .await?;
        Ok(list.models)
    }

    pub async fn fetch_keys(&self, account_key: &str) -> Result<Vec<CkeyAiKey>, String> {
        let list: CkeyKeyList = self
            .get_query("/api/llm/keys", &[("key", account_key.to_string())])
            .await?;
        Ok(list.items)
    }

    pub async fn fetch_usage_stats(&self, account_key: &str) -> Result<CkeyUsageStats, String> {
        self.get_query("/api/llm/usage-stats", &[("key", account_key.to_string())])
            .await
    }

    pub async fn fetch_usage(
        &self,
        account_key: &str,
        ai_key_hint: &str,
        page: u64,
        limit: u64,
    ) -> Result<CkeyUsagePage, String> {
        // API /api/llm/usage bắt buộc có param `api_key` (xác thực chỉ cần param hiện diện;
        // dùng prefix key của user làm hint để lọc đúng AI key của tài khoản).
        self.get_query(
            "/api/llm/usage",
            &[
                ("key", account_key.to_string()),
                ("api_key", ai_key_hint.to_string()),
                ("page", page.to_string()),
                ("limit", limit.to_string()),
            ],
        )
        .await
    }
}

impl Default for CkeyClient {
    fn default() -> Self {
        Self::new("")
    }
}

// ---------- Tests: parse thuần từ fixture JSON ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_profile_fixture() {
        let body = r#"{
            "success": true, "status": 200, "message": "OK",
            "data": { "profile": {
                "username": "demo", "name": "Demo User", "email": "demo@example.com",
                "balance": "100.000 VND", "balance_raw": 100000,
                "created_at": "06/05/2026 - 09:00:00", "created_at_timestamp": 1778032800,
                "api_key_masked": "abcd********wxyz"
            } }
        }"#;
        // profile nằm trong data.profile → parse wrapper theo kiểu profile
        #[derive(Deserialize)]
        struct ProfileWrap {
            profile: CkeyProfile,
        }
        let data: ProfileWrap = parse_wrapped(body).unwrap();
        assert_eq!(data.profile.username, "demo");
        assert_eq!(data.profile.balance_raw, 100000.0);
    }

    #[test]
    fn parse_models_fixture() {
        let body = r#"{
            "success": true, "status": 200, "message": "OK",
            "data": { "count": 1, "models": [ {
                "public_name": "provider/gpt-demo", "display_name": "GPT Demo",
                "model_name": "gpt-demo", "provider_username": "provider",
                "is_provider_model": true,
                "input_price_per_million_vnd": 5000, "output_price_per_million_vnd": 15000,
                "price_per_request_vnd": 0, "min_charge_per_request_vnd": 1,
                "cache_enabled": false, "cache_read_price_per_million_vnd": 0,
                "cache_write_price_per_million_vnd": 0, "request_rate_limit_per_minute": 0,
                "max_output_tokens_limit": 0, "context_tokens_limit": 0,
                "supported_paths": ["chat/completions", "images/edits"]
            } ] }
        }"#;
        let list: CkeyModelList = parse_wrapped(body).unwrap();
        assert_eq!(list.models.len(), 1);
        assert_eq!(list.models[0].public_name, "provider/gpt-demo");
        assert_eq!(list.models[0].input_price_per_million_vnd, 5000.0);
    }

    #[test]
    fn parse_keys_fixture() {
        let body = r#"{
            "success": true, "status": 200, "message": "OK",
            "data": { "items": [ {
                "id": 4804, "key_name": "Production", "api_key": "ck-prod-xxxx",
                "key_prefix": "ck-prod-xxxx", "is_active": true,
                "created_at": 1778032800, "created_at_text": "06/05/2026 - 09:00:00"
            } ] }
        }"#;
        let list: CkeyKeyList = parse_wrapped(body).unwrap();
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].api_key, "ck-prod-xxxx");
        assert!(list.items[0].is_active);
    }

    #[test]
    fn parse_usage_stats_fixture() {
        let body = r#"{
            "success": true, "status": 200, "message": "OK",
            "data": { "requests": 42, "success_requests": 40,
                "prompt_tokens": 5000, "completion_tokens": 3000, "total_tokens": 8000,
                "cache_read_tokens": 0, "cache_write_tokens": 0,
                "charged_vnd": 75.5, "charged_vnd_text": "75,5 VND", "since": 0 }
        }"#;
        let stats: CkeyUsageStats = parse_wrapped(body).unwrap();
        assert_eq!(stats.requests, 42);
        assert_eq!(stats.charged_vnd, 75.5);
    }

    #[test]
    fn parse_usage_page_fixture() {
        let body = r#"{
            "success": true, "status": 200, "message": "OK",
            "data": { "items": [ {
                "request_id": "req_demo", "model_name": "gpt-demo",
                "request_path": "chat/completions", "http_status": 200,
                "prompt_tokens": 120, "completion_tokens": 80, "total_tokens": 200,
                "charged_vnd": 1.8, "status": "success", "latency_ms": 950,
                "stream": true, "created_at": 1778032800,
                "created_at_text": "06/05/2026 - 09:00:00"
            } ],
            "pagination": { "page": 1, "limit": 20, "total": 1, "total_pages": 1 } }
        }"#;
        let page: CkeyUsagePage = parse_wrapped(body).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].total_tokens, 200);
        assert_eq!(page.pagination.as_ref().unwrap().total_pages, 1);
    }

    #[test]
    fn parse_wrapped_error_message() {
        let body = r#"{"success": false, "status": 400, "message": "Key không hợp lệ", "data": null}"#;
        let err = parse_wrapped::<CkeyProfile>(body).unwrap_err();
        assert!(err.contains("Key không hợp lệ"), "err = {}", err);
    }

    #[test]
    fn friendly_error_mapping() {
        assert!(friendly_error(401, "").contains("401/403"));
        assert!(friendly_error(402, "").contains("hết tiền"));
        let body = r#"{"success":false,"message":"Rate limit"}"#;
        assert!(friendly_error(429, body).contains("Rate limit"));
    }
}
