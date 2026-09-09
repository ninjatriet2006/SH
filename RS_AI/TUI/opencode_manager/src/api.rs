use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// AI SDK adapters OpenCode dùng cho custom provider. `Auto` ưu tiên Responses,
/// rồi Chat Completions, cuối cùng Anthropic để chọn endpoint mới nhất nhận POST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    Responses,
    ChatCompletions,
    Anthropic,
}

impl ProviderProtocol {
    pub const AUTO_ORDER: [Self; 3] = [Self::Responses, Self::ChatCompletions, Self::Anthropic];

    pub const fn npm(self) -> &'static str {
        match self {
            Self::Responses => "@ai-sdk/openai",
            Self::ChatCompletions => "@ai-sdk/openai-compatible",
            Self::Anthropic => "@ai-sdk/anthropic",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Responses => "OpenAI Responses",
            Self::ChatCompletions => "OpenAI Chat Completions",
            Self::Anthropic => "Anthropic Messages",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum ApiStatus {
    Alive,
    InsufficientCredits(String),
    InvalidKey(String),
    Offline(String),
}

impl std::fmt::Display for ApiStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiStatus::Alive => write!(f, "Hoạt động"),
            ApiStatus::InsufficientCredits(msg) => write!(f, "Hết tiền: {}", msg),
            ApiStatus::InvalidKey(msg) => write!(f, "Sai API Key: {}", msg),
            ApiStatus::Offline(msg) => write!(f, "Offline: {}", msg),
        }
    }
}

#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    _error_type: Option<String>,
    _code: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

pub struct ApiClient {
    client: Client,
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient {
    pub fn new() -> Self {
        ApiClient {
            client: Client::builder()
                .timeout(Duration::from_secs(6))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Trả về url phù hợp để lấy models.
    pub(crate) fn get_models_url(base_url: &str) -> String {
        let clean_base = crate::config::normalize_base_url(base_url);
        let clean_base = clean_base.trim_end_matches('/');
        if clean_base.contains("opengateway.gitlawb.com") {
            format!("{}/openai/models", clean_base)
        } else {
            format!("{}/models", clean_base)
        }
    }

    fn endpoint_url(base_url: &str, path: &str) -> String {
        let clean_base = crate::config::normalize_base_url(base_url);
        let clean_base = clean_base.trim_end_matches('/');
        if clean_base.contains("opengateway.gitlawb.com") {
            format!("{clean_base}/openai/{path}")
        } else {
            format!("{clean_base}/{path}")
        }
    }

    fn probe_status_accepted(status: reqwest::StatusCode, body: &str) -> bool {
        // Lỗi 4xx không đủ chứng minh protocol: proxy/WAF cũng có thể trả JSON
        // cho path không tồn tại. Một số proxy còn bọc lỗi trong HTTP 200.
        if !status.is_success() {
            return false;
        }
        serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .is_none_or(|value| value.get("error").is_none())
    }

    fn probe_model(models: &[String]) -> Option<&str> {
        const NON_CHAT_MARKERS: [&str; 8] = [
            "embed",
            "rerank",
            "image",
            "whisper",
            "audio",
            "speech",
            "tts",
            "moderation",
        ];
        models
            .iter()
            .find(|model| {
                let lower = model.to_ascii_lowercase();
                !NON_CHAT_MARKERS.iter().any(|marker| lower.contains(marker))
            })
            .or_else(|| models.first())
            .map(String::as_str)
    }

    async fn probe_protocol(
        &self,
        base_url: &str,
        api_key: &str,
        model: &str,
        protocol: ProviderProtocol,
    ) -> Result<(), String> {
        let (url, body, anthropic) = match protocol {
            ProviderProtocol::Responses => (
                Self::endpoint_url(base_url, "responses"),
                serde_json::json!({"model": model, "input": "ping", "max_output_tokens": 1}),
                false,
            ),
            ProviderProtocol::ChatCompletions => (
                Self::endpoint_url(base_url, "chat/completions"),
                serde_json::json!({"model": model, "messages": [{"role": "user", "content": "ping"}], "max_tokens": 1}),
                false,
            ),
            ProviderProtocol::Anthropic => (
                Self::endpoint_url(base_url, "messages"),
                serde_json::json!({"model": model, "max_tokens": 1, "messages": [{"role": "user", "content": "ping"}]}),
                true,
            ),
        };

        let mut request = self.client.post(url).json(&body);
        if anthropic {
            request = request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
        } else {
            request = request.header("Authorization", format!("Bearer {api_key}"));
        }
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                "Kết nối quá hạn (Timeout)".to_string()
            } else {
                format!("Lỗi kết nối: {e}")
            }
        })?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Chỉ lưu adapter khi probe thành công; mọi lỗi đều được đưa vào thông
        // báo để người dùng chọn protocol thủ công thay vì đoán nhầm.
        if Self::probe_status_accepted(status, &body) {
            return Ok(());
        }
        let detail = body.lines().next().unwrap_or_default().trim();
        Err(if detail.is_empty() {
            format!("HTTP {status}")
        } else {
            format!("HTTP {status}: {detail}")
        })
    }

    /// Thử endpoint POST thật, không chỉ `GET /models`: WAF có thể chặn riêng
    /// `/chat/completions` như case JustWoker dù catalog vẫn trả 200.
    pub async fn detect_protocol(&self, base_url: &str, api_key: &str) -> Result<ProviderProtocol, String> {
        // Probe bằng model thật: không coi 404 là tương thích vì nó thường là
        // path không tồn tại. GET /models từng sống ở case JustWoker, trong khi
        // POST Chat bị Cloudflare 403, nên danh sách này phân biệt được hai lỗi.
        let models = match self.fetch_models(base_url, api_key).await {
            Ok(models) => models,
            Err(bearer_error) => self
                .fetch_models_with_header(base_url, "x-api-key", api_key)
                .await
                .map_err(|anthropic_error| {
                    format!(
                        "Không lấy được danh sách model bằng Bearer ({bearer_error}) hoặc x-api-key ({anthropic_error})"
                    )
                })?,
        };
        let model = Self::probe_model(&models)
            .ok_or_else(|| "Provider không trả model nào để kiểm tra endpoint".to_string())?;
        let mut errors = Vec::new();
        for protocol in ProviderProtocol::AUTO_ORDER {
            match self.probe_protocol(base_url, api_key, model, protocol).await {
                Ok(()) => return Ok(protocol),
                Err(error) => errors.push(format!("{} ({model}): {error}", protocol.label())),
            }
        }
        Err(format!(
            "Không tìm thấy endpoint POST tương thích. {}",
            errors.join(" | ")
        ))
    }

    pub async fn test_api(&self, base_url: &str, api_key: &str) -> ApiStatus {
        let url = Self::get_models_url(base_url);

        let req = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .build();

        let response = match req {
            Ok(r) => match self.client.execute(r).await {
                Ok(resp) => resp,
                Err(e) => {
                    let err_str = e.to_string();
                    if e.is_timeout() {
                        return ApiStatus::Offline("Kết nối quá hạn (Timeout)".to_string());
                    }
                    return ApiStatus::Offline(format!("Lỗi kết nối: {}", err_str));
                }
            },
            Err(e) => return ApiStatus::Offline(format!("Lỗi cấu hình request: {}", e)),
        };

        let status = response.status();
        if status.is_success() {
            return ApiStatus::Alive;
        }

        // Đọc body lỗi để phân tích chi tiết hơn
        let body_text = response.text().await.unwrap_or_default();
        if status.as_u16() == 402 {
            let parsed: Result<ErrorResponse, _> = serde_json::from_str(&body_text);
            let msg = if let Ok(err_resp) = parsed {
                err_resp
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Insufficient credits".to_string())
            } else {
                "Tài khoản hết hạn/hết tiền (402)".to_string()
            };
            return ApiStatus::InsufficientCredits(msg);
        }

        if status.as_u16() == 401 || status.as_u16() == 403 {
            let parsed: Result<ErrorResponse, _> = serde_json::from_str(&body_text);
            let msg = if let Ok(err_resp) = parsed {
                err_resp
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unauthorized".to_string())
            } else {
                "API Key không hợp lệ (401/403)".to_string()
            };
            return ApiStatus::InvalidKey(msg);
        }

        // Hỗ trợ fallback cho gitlawb nếu nó báo 404 cho url thường và yêu cầu /v1/<provider>/<path>
        if status.as_u16() == 404
            && body_text.contains("Use /v1/<provider>/<path>")
            && !base_url.contains("opengateway.gitlawb.com")
        {
            // Thử gọi lại với /openai/models
            let clean_base = base_url.trim_end_matches('/');
            let retry_url = format!("{}/openai/models", clean_base);

            let retry_resp = self
                .client
                .get(&retry_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await;

            if let Ok(resp) = retry_resp {
                let retry_status = resp.status();
                if retry_status.is_success() {
                    return ApiStatus::Alive;
                }
                if retry_status.as_u16() == 402 {
                    return ApiStatus::InsufficientCredits("Hết tiền (402)".to_string());
                }
                if retry_status.as_u16() == 401 || retry_status.as_u16() == 403 {
                    return ApiStatus::InvalidKey("API Key không hợp lệ".to_string());
                }
                return ApiStatus::InvalidKey(format!("Lỗi HTTP retry: {}", retry_status));
            }
        }

        ApiStatus::InvalidKey(format!("HTTP {}", status))
    }

    pub async fn fetch_models(&self, base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
        self.fetch_models_with_header(base_url, "Authorization", &format!("Bearer {api_key}"))
            .await
    }

    async fn fetch_models_with_header(
        &self,
        base_url: &str,
        header_name: &str,
        header_value: &str,
    ) -> Result<Vec<String>, String> {
        let url = Self::get_models_url(base_url);

        let response = self
            .client
            .get(&url)
            .header(header_name, header_value)
            .send()
            .await
            .map_err(|e| format!("Lỗi gọi API quét models: {}", e))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| format!("Không thể đọc body: {}", e))?;

        if !status.is_success() {
            // Thử fallback cho gitlawb
            if status.as_u16() == 404
                && body_text.contains("Use /v1/<provider>/<path>")
                && !base_url.contains("opengateway.gitlawb.com")
            {
                let clean_base = base_url.trim_end_matches('/');
                let retry_url = format!("{}/openai/models", clean_base);
                let retry_response = self
                    .client
                    .get(&retry_url)
                    .header(header_name, header_value)
                    .send()
                    .await
                    .map_err(|e| format!("Lỗi gọi API fallback quét models: {}", e))?;

                if retry_response.status().is_success() {
                    let retry_body = retry_response
                        .text()
                        .await
                        .map_err(|e| format!("Không thể đọc body: {}", e))?;
                    let res: OpenAIModelsResponse =
                        serde_json::from_str(&retry_body).map_err(|e| format!("Lỗi parse JSON: {}", e))?;
                    return Ok(res.data.into_iter().map(|m| m.id).collect());
                }
            }

            let parsed: Result<ErrorResponse, _> = serde_json::from_str(&body_text);
            let msg = if let Ok(err_resp) = parsed {
                err_resp
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| format!("HTTP {}", status))
            } else {
                format!("HTTP {}", status)
            };
            return Err(msg);
        }

        let res: OpenAIModelsResponse =
            serde_json::from_str(&body_text).map_err(|e| format!("Lỗi parse JSON: {}", e))?;

        Ok(res.data.into_iter().map(|m| m.id).collect())
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::ApiClient;
    use reqwest::StatusCode;

    #[test]
    fn endpoint_url_ho_tro_gitlawb() {
        assert_eq!(
            ApiClient::endpoint_url("https://opengateway.gitlawb.com/v1", "responses"),
            "https://opengateway.gitlawb.com/v1/openai/responses"
        );
        assert_eq!(
            ApiClient::endpoint_url("https://example.com/v1/chat/completions", "responses"),
            "https://example.com/v1/responses"
        );
    }

    #[test]
    fn probe_chi_nhan_response_thanh_cong() {
        assert!(ApiClient::probe_status_accepted(
            StatusCode::OK,
            r#"{"id":"response_1"}"#
        ));
        assert!(!ApiClient::probe_status_accepted(
            StatusCode::OK,
            r#"{"error":{"message":"blocked"}}"#
        ));
        assert!(!ApiClient::probe_status_accepted(StatusCode::BAD_REQUEST, ""));
        assert!(!ApiClient::probe_status_accepted(StatusCode::UNPROCESSABLE_ENTITY, ""));
        assert!(!ApiClient::probe_status_accepted(StatusCode::UNAUTHORIZED, ""));
        assert!(!ApiClient::probe_status_accepted(StatusCode::NOT_FOUND, ""));
    }

    #[test]
    fn probe_bo_qua_model_khong_phai_chat() {
        let models = vec!["text-embedding-3-small".to_string(), "gpt-4.1-mini".to_string()];
        assert_eq!(ApiClient::probe_model(&models), Some("gpt-4.1-mini"));

        let only_embedding = vec!["embedding-v1".to_string()];
        assert_eq!(ApiClient::probe_model(&only_embedding), Some("embedding-v1"));
        assert_eq!(ApiClient::probe_model(&[]), None);
    }
}
