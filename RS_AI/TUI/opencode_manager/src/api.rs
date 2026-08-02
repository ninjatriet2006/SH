use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

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
    fn get_models_url(base_url: &str) -> String {
        let clean_base = base_url.trim_end_matches('/');
        if clean_base.contains("opengateway.gitlawb.com") {
            format!("{}/openai/models", clean_base)
        } else {
            format!("{}/models", clean_base)
        }
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
        let url = Self::get_models_url(base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
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
                    .header("Authorization", format!("Bearer {}", api_key))
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
