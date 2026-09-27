//! 1min.AI Adapter: Chuyển đổi 2 chiều giữa chuẩn OpenAI Protocol (Local)
//! và 1min.AI Aggregator API (Upstream).

use serde_json::{json, Value};
use uuid::Uuid;

/// Nhận diện xem URL đích có phải là endpoint của 1min.ai hay không.
pub fn is_1min_target(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("1min.ai") || lower.contains("api.1min.ai")
}

/// Ánh xạ tên model phổ thông ở Local sang modelId thực tế của 1min.AI.
pub fn resolve_model_id(input_model: &str) -> String {
    let m = input_model.trim().to_lowercase();
    match m.as_str() {
        // OpenAI models
        "gpt-4o" => "gpt-4o".to_string(),
        "gpt-4o-mini" => "gpt-4o-mini".to_string(),
        "gpt-4-turbo" => "gpt-4-turbo".to_string(),
        "gpt-3.5-turbo" => "gpt-3.5-turbo".to_string(),
        "gpt-4.1" => "gpt-4.1".to_string(),
        "gpt-4.1-mini" => "gpt-4.1-mini".to_string(),
        "o3-mini" => "o3-mini".to_string(),
        "o3" => "o3".to_string(),
        "gpt-5" => "gpt-5".to_string(),
        "gpt-5-mini" => "gpt-5-mini".to_string(),

        // Anthropic Claude qua AWS Bedrock
        "claude-3-5-sonnet" | "claude-3.5-sonnet" | "claude-3-5-sonnet-latest" => {
            "us.anthropic.claude-3-5-sonnet-20241022-v2:0".to_string()
        }
        "claude-3-7-sonnet" | "claude-3.7-sonnet" => {
            "us.anthropic.claude-3-7-sonnet-20250219-v1:0".to_string()
        }
        "claude-3-5-haiku" | "claude-3.5-haiku" => {
            "us.anthropic.claude-3-5-haiku-20241022-v1:0".to_string()
        }

        // Google Gemini
        "gemini-1.5-flash" | "gemini-2.5-flash" => "gemini-2.5-flash".to_string(),
        "gemini-1.5-pro" | "gemini-2.5-pro" => "gemini-2.5-pro".to_string(),
        "gemini-3.7-flash" => "gemini-3.7-flash".to_string(),

        // DeepSeek
        "deepseek-chat" | "deepseek-v3" | "deepseek-v4" => "deepseek-v4-pro".to_string(),
        "deepseek-flash" => "deepseek-flash".to_string(),

        // Alibaba Qwen
        "qwen-max" | "qwen3-max" | "qwen3.7-max" => "qwen3.7-max".to_string(),
        "qwen-plus" | "qwen3.7-plus" => "qwen3.7-plus".to_string(),
        "qwen-flash" | "qwen3.7-flash" => "qwen3.7-flash".to_string(),

        // Giữ nguyên model ID nếu đã đúng format
        _ => input_model.trim().to_string(),
    }
}

/// Dự đoán mục đích dịch thuật và suy ra tone/writingStyle từ temperature.
pub fn detect_translation_intent(
    temp: f64,
    system_text: &str,
    user_text: &str,
) -> (bool, &'static str, &'static str, String, String) {
    let sys_lower = system_text.to_lowercase();
    let user_lower = user_text.to_lowercase();

    // Điều kiện nhận diện dịch thuật:
    // 1. temperature thấp (<= 0.35) đặc trưng cho deterministic translation
    // 2. Hoặc prompt chứa từ khóa dịch
    let is_translation = temp <= 0.35
        || sys_lower.contains("translate")
        || sys_lower.contains("dịch")
        || sys_lower.contains("translation")
        || user_lower.starts_with("translate")
        || user_lower.starts_with("dịch");

    // Ánh xạ temperature sang Tone (độ biểu cảm)
    let tone = if temp <= 0.2 {
        "clinical"
    } else if temp <= 0.35 {
        "formal"
    } else if temp <= 0.75 {
        "friendly"
    } else {
        "playful"
    };

    // Ánh xạ temperature sang Writing Style (văn phong)
    let style = if temp <= 0.2 {
        "Academic"
    } else if temp <= 0.35 {
        "Analytical"
    } else if temp <= 0.75 {
        "Conversational"
    } else {
        "Creative"
    };

    // Suy đoán ngôn ngữ đích
    let target_lang = if sys_lower.contains("vietnamese")
        || sys_lower.contains("tiếng việt")
        || user_lower.contains("sang tiếng việt")
        || user_lower.contains("into vietnamese")
    {
        "vi".to_string()
    } else if sys_lower.contains("english")
        || sys_lower.contains("tiếng anh")
        || user_lower.contains("into english")
    {
        "en".to_string()
    } else if sys_lower.contains("japanese")
        || sys_lower.contains("tiếng nhật")
        || user_lower.contains("into japanese")
    {
        "ja".to_string()
    } else if sys_lower.contains("chinese")
        || sys_lower.contains("tiếng trung")
        || user_lower.contains("into chinese")
    {
        "zh".to_string()
    } else {
        "vi".to_string() // Mặc định đích là Tiếng Việt
    };

    let original_lang = if sys_lower.contains("from english") || user_lower.contains("từ tiếng anh") {
        "en".to_string()
    } else if sys_lower.contains("from japanese") || user_lower.contains("từ tiếng nhật") {
        "ja".to_string()
    } else if sys_lower.contains("from chinese") || user_lower.contains("từ tiếng trung") {
        "zh".to_string()
    } else {
        "auto".to_string()
    };

    (is_translation, tone, style, original_lang, target_lang)
}

/// Chuyển đổi OpenAI Request Payload sang 1min.AI format & endpoint tương ứng.
/// Trả về: (new_target_url, new_body_bytes, is_streaming, requested_model)
pub fn transform_request(
    _original_target_url: &str,
    raw_bytes: &[u8],
) -> Result<(String, Vec<u8>, bool, String), String> {
    let json_val: Value = serde_json::from_slice(raw_bytes)
        .map_err(|e| format!("Invalid JSON request body: {}", e))?;

    let model_raw = json_val["model"].as_str().unwrap_or("gpt-4o");
    let resolved_model = resolve_model_id(model_raw);
    let temp = json_val["temperature"].as_f64().unwrap_or(0.7);
    let stream = json_val["stream"].as_bool().unwrap_or(false);

    let messages = json_val["messages"].as_array();
    let mut system_text = String::new();
    let mut user_text = String::new();
    let mut history_turns = Vec::new();

    if let Some(msgs) = messages {
        for m in msgs {
            let role = m["role"].as_str().unwrap_or("");
            let content = match &m["content"] {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    let mut text_acc = String::new();
                    for item in arr {
                        if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                            text_acc.push_str(t);
                        }
                    }
                    text_acc
                }
                other => other.to_string(),
            };

            if role == "system" {
                if !system_text.is_empty() {
                    system_text.push('\n');
                }
                system_text.push_str(&content);
            } else if role == "user" {
                user_text = content.clone();
                history_turns.push(format!("User: {}", content));
            } else if role == "assistant" {
                history_turns.push(format!("Assistant: {}", content));
            }
        }
    }

    let (is_translation, tone, style, orig_lang, target_lang) =
        detect_translation_intent(temp, &system_text, &user_text);

    if is_translation {
        // Nhánh CONTENT_TRANSLATOR
        let base_url = "https://api.1min.ai/api/features";
        let target_url = if stream {
            format!("{}?isStreaming=true", base_url)
        } else {
            base_url.to_string()
        };

        let payload = json!({
            "type": "CONTENT_TRANSLATOR",
            "model": resolved_model,
            "conversationId": "CONTENT_TRANSLATOR",
            "promptObject": {
                "originalLanguage": orig_lang,
                "targetLanguage": target_lang,
                "tone": tone,
                "domain": "Technology & Computing",
                "writingStyle": style,
                "prompt": user_text
            }
        });

        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| format!("Failed to serialize 1min.ai translator payload: {}", e))?;
        Ok((target_url, bytes, stream, model_raw.to_string()))
    } else {
        // Nhánh UNIFY_CHAT_WITH_AI
        let base_url = "https://api.1min.ai/api/chat-with-ai";
        let target_url = if stream {
            format!("{}?isStreaming=true", base_url)
        } else {
            base_url.to_string()
        };

        // Gộp system prompt vào bối cảnh nếu có
        let combined_prompt = if !system_text.is_empty() {
            format!("[Instruction: {}]\n{}", system_text, user_text)
        } else {
            user_text
        };

        let payload = json!({
            "type": "UNIFY_CHAT_WITH_AI",
            "model": resolved_model,
            "promptObject": {
                "prompt": combined_prompt,
                "settings": {
                    "historySettings": {
                        "isMixed": false,
                        "historyMessageLimit": 10
                    },
                    "webSearchSettings": {
                        "webSearch": false
                    },
                    "withMemories": false
                }
            }
        });

        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| format!("Failed to serialize 1min.ai chat payload: {}", e))?;
        Ok((target_url, bytes, stream, model_raw.to_string()))
    }
}

/// Trả về danh sách models chuẩn OpenAI tương thích 1min.AI
pub fn get_models_list_json() -> Value {
    json!({
        "object": "list",
        "data": [
            {"id": "gpt-4o", "object": "model", "owned_by": "1min.ai"},
            {"id": "gpt-4o-mini", "object": "model", "owned_by": "1min.ai"},
            {"id": "gpt-4-turbo", "object": "model", "owned_by": "1min.ai"},
            {"id": "gpt-3.5-turbo", "object": "model", "owned_by": "1min.ai"},
            {"id": "gpt-4.1", "object": "model", "owned_by": "1min.ai"},
            {"id": "gpt-4.1-mini", "object": "model", "owned_by": "1min.ai"},
            {"id": "o3-mini", "object": "model", "owned_by": "1min.ai"},
            {"id": "claude-3-5-sonnet", "object": "model", "owned_by": "1min.ai"},
            {"id": "claude-3-7-sonnet", "object": "model", "owned_by": "1min.ai"},
            {"id": "claude-3-5-haiku", "object": "model", "owned_by": "1min.ai"},
            {"id": "gemini-2.5-flash", "object": "model", "owned_by": "1min.ai"},
            {"id": "gemini-2.5-pro", "object": "model", "owned_by": "1min.ai"},
            {"id": "gemini-3.7-flash", "object": "model", "owned_by": "1min.ai"},
            {"id": "deepseek-chat", "object": "model", "owned_by": "1min.ai"},
            {"id": "deepseek-flash", "object": "model", "owned_by": "1min.ai"},
            {"id": "qwen3.7-max", "object": "model", "owned_by": "1min.ai"},
            {"id": "qwen3.7-plus", "object": "model", "owned_by": "1min.ai"},
            {"id": "qwen3.7-flash", "object": "model", "owned_by": "1min.ai"}
        ]
    })
}

/// Chuyển đổi Non-Streaming JSON Response từ 1min.AI sang OpenAI ChatCompletion format.
pub fn transform_response_json(one_min_bytes: &[u8], requested_model: &str) -> Result<Vec<u8>, String> {
    let v: Value = serde_json::from_slice(one_min_bytes)
        .map_err(|e| format!("Invalid 1min.ai response JSON: {}", e))?;

    // Trích xuất text từ các cấu trúc phổ biến của 1min.ai:
    // 1. v["aiRecord"]["aiRecordDetail"]["resultObject"] (Array of strings hoặc array of objects)
    // 2. v["aiRecord"]["aiRecordDetail"]["result"]
    // 3. v["resultObject"]
    let record = &v["aiRecord"];
    let uuid = record["uuid"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut content = String::new();
    if let Some(arr) = record["aiRecordDetail"]["resultObject"].as_array() {
        if let Some(first) = arr.first() {
            if let Some(s) = first.as_str() {
                content = s.to_string();
            } else if let Some(c) = first.get("content").and_then(|v| v.as_str()) {
                content = c.to_string();
            }
        }
    } else if let Some(s) = record["aiRecordDetail"]["resultObject"].as_str() {
        content = s.to_string();
    } else if let Some(s) = record["aiRecordDetail"]["result"].as_str() {
        content = s.to_string();
    } else if let Some(arr) = v["resultObject"].as_array() {
        if let Some(first) = arr.first().and_then(|i| i.as_str()) {
            content = first.to_string();
        }
    } else if let Some(s) = v["resultObject"].as_str() {
        content = s.to_string();
    }

    if content.is_empty() && (v.get("error").is_some() || v.get("message").is_some()) {
        return Err(format!("1min.AI error response: {}", String::from_utf8_lossy(one_min_bytes)));
    }

    let openai_resp = json!({
        "id": format!("chatcmpl-{}", uuid),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": requested_model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        }
    });

    serde_json::to_vec(&openai_resp).map_err(|e| format!("Failed to serialize OpenAI response: {}", e))
}

/// Trạng thái phân tích SSE Stream từ 1min.AI
#[derive(Debug, Clone)]
pub struct OneMinSseState {
    pub stream_id: String,
    pub current_event: String,
}

impl Default for OneMinSseState {
    fn default() -> Self {
        Self::new()
    }
}

impl OneMinSseState {
    pub fn new() -> Self {
        Self {
            stream_id: format!("chatcmpl-{}", Uuid::new_v4()),
            current_event: String::new(),
        }
    }

    /// Nhận 1 dòng text SSE từ 1min.AI và chuyển đổi sang chuẩn OpenAI SSE chunk
    pub fn process_line(&mut self, line: &str, model: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(event_name) = trimmed.strip_prefix("event:") {
            self.current_event = event_name.trim().to_string();
            return None;
        }

        if let Some(data_str) = trimmed.strip_prefix("data:") {
            let data_trimmed = data_str.trim();
            if data_trimmed.is_empty() {
                return None;
            }

            if data_trimmed == "[DONE]" {
                return Some("data: [DONE]\n\n".to_string());
            }

            match self.current_event.as_str() {
                "content" | "message" | "" => {
                    if let Ok(v) = serde_json::from_str::<Value>(data_trimmed) {
                        if let Some(chunk_text) = v["content"]
                            .as_str()
                            .or_else(|| v["delta"]["content"].as_str())
                            .or_else(|| v["text"].as_str())
                        {
                            let chunk = json!({
                                "id": self.stream_id,
                                "object": "chat.completion.chunk",
                                "created": chrono::Utc::now().timestamp(),
                                "model": model,
                                "choices": [{
                                    "index": 0,
                                    "delta": {
                                        "content": chunk_text
                                    },
                                    "finish_reason": null
                                }]
                            });
                            return Some(format!("data: {}\n\n", serde_json::to_string(&chunk).unwrap()));
                        }
                    }
                }
                "error" => {
                    let err_msg = serde_json::from_str::<Value>(data_trimmed)
                        .ok()
                        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| data_trimmed.to_string());

                    let chunk = json!({
                        "id": self.stream_id,
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": model,
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": format!("\n[1min.AI Error: {}]", err_msg)
                            },
                            "finish_reason": "stop"
                        }]
                    });
                    return Some(format!("data: {}\n\ndata: [DONE]\n\n", serde_json::to_string(&chunk).unwrap()));
                }
                "done" => {
                    let finish_chunk = json!({
                        "id": self.stream_id,
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": model,
                        "choices": [{
                            "index": 0,
                            "delta": {},
                            "finish_reason": "stop"
                        }]
                    });
                    return Some(format!("data: {}\n\ndata: [DONE]\n\n", serde_json::to_string(&finish_chunk).unwrap()));
                }
                _ => {}
            }
        }

        None
    }
}

/// Transformer bọc lấy Byte Stream SSE từ 1min.AI, đệm dòng đầy đủ
/// và biến đổi tuần tự sang byte chuẩn OpenAI SSE (`data: {...}\n\n`).
pub struct OneMinSseTransformer {
    pub state: OneMinSseState,
    pub buffer: String,
    pub model: String,
    pub done_emitted: bool,
}

impl OneMinSseTransformer {
    pub fn new(model: &str) -> Self {
        Self {
            state: OneMinSseState::new(),
            buffer: String::new(),
            model: model.to_string(),
            done_emitted: false,
        }
    }

    /// Nhận nạp chunk bytes từ mạng, tách thành các dòng kết thúc bằng `\n` và biến đổi
    pub fn feed_bytes(&mut self, bytes: &[u8]) -> Vec<u8> {
        let text = String::from_utf8_lossy(bytes);
        self.buffer.push_str(&text);

        let mut output = Vec::new();
        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 1..].to_string();

            if let Some(chunk_str) = self.state.process_line(&line, &self.model) {
                if chunk_str.contains("[DONE]") {
                    self.done_emitted = true;
                }
                output.extend_from_slice(chunk_str.as_bytes());
            }
        }
        output
    }

    /// Xả sạch phần còn lại trong buffer và bảo đảm luôn có `data: [DONE]\n\n`
    pub fn finish(&mut self) -> Vec<u8> {
        let mut output = Vec::new();
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            if let Some(chunk_str) = self.state.process_line(&line, &self.model) {
                if chunk_str.contains("[DONE]") {
                    self.done_emitted = true;
                }
                output.extend_from_slice(chunk_str.as_bytes());
            }
        }

        if !self.done_emitted {
            self.done_emitted = true;
            let finish_chunk = json!({
                "id": self.state.stream_id,
                "object": "chat.completion.chunk",
                "created": chrono::Utc::now().timestamp(),
                "model": self.model,
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            });
            let s = format!("data: {}\n\ndata: [DONE]\n\n", serde_json::to_string(&finish_chunk).unwrap());
            output.extend_from_slice(s.as_bytes());
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_1min_target() {
        assert!(is_1min_target("https://api.1min.ai"));
        assert!(is_1min_target("https://api.1min.ai/api/features"));
        assert!(is_1min_target("https://app.1min.ai/v1"));
        assert!(!is_1min_target("https://api.openai.com/v1"));
    }

    #[test]
    fn test_resolve_model_id() {
        assert_eq!(resolve_model_id("gpt-4o"), "gpt-4o");
        assert_eq!(resolve_model_id("claude-3-5-sonnet"), "us.anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert_eq!(resolve_model_id("gemini-2.5-flash"), "gemini-2.5-flash");
        assert_eq!(resolve_model_id("deepseek-chat"), "deepseek-v4-pro");
        assert_eq!(resolve_model_id("custom-unknown-model"), "custom-unknown-model");
    }

    #[test]
    fn test_transform_translation_request_low_temp() {
        let openai_req = json!({
            "model": "gpt-4o",
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": "You are a professional translator into Vietnamese."},
                {"role": "user", "content": "Hello world"}
            ]
        });

        let bytes = serde_json::to_vec(&openai_req).unwrap();
        let (url, transformed, stream, model) = transform_request("https://api.1min.ai", &bytes).unwrap();

        assert_eq!(model, "gpt-4o");
        assert!(!stream);
        assert!(url.contains("/api/features"));

        let val: Value = serde_json::from_slice(&transformed).unwrap();
        assert_eq!(val["type"], "CONTENT_TRANSLATOR");
        assert_eq!(val["model"], "gpt-4o");
        assert_eq!(val["promptObject"]["targetLanguage"], "vi");
        assert_eq!(val["promptObject"]["tone"], "clinical");
        assert_eq!(val["promptObject"]["writingStyle"], "Academic");
        assert_eq!(val["promptObject"]["prompt"], "Hello world");
    }

    #[test]
    fn test_transform_chat_request_normal_temp() {
        let openai_req = json!({
            "model": "gpt-4o-mini",
            "temperature": 0.7,
            "stream": true,
            "messages": [
                {"role": "user", "content": "What is AI?"}
            ]
        });

        let bytes = serde_json::to_vec(&openai_req).unwrap();
        let (url, transformed, stream, model) = transform_request("https://api.1min.ai", &bytes).unwrap();

        assert_eq!(model, "gpt-4o-mini");
        assert!(stream);
        assert!(url.contains("/api/chat-with-ai?isStreaming=true"));

        let val: Value = serde_json::from_slice(&transformed).unwrap();
        assert_eq!(val["type"], "UNIFY_CHAT_WITH_AI");
        assert_eq!(val["promptObject"]["prompt"], "What is AI?");
    }

    #[test]
    fn test_transform_response_json() {
        let one_min_resp = json!({
            "aiRecord": {
                "uuid": "test-uuid-999",
                "aiRecordDetail": {
                    "resultObject": ["Xin chào thế giới"]
                }
            }
        });

        let bytes = serde_json::to_vec(&one_min_resp).unwrap();
        let openai_bytes = transform_response_json(&bytes, "gpt-4o").unwrap();
        let val: Value = serde_json::from_slice(&openai_bytes).unwrap();

        assert_eq!(val["id"], "chatcmpl-test-uuid-999");
        assert_eq!(val["object"], "chat.completion");
        assert_eq!(val["choices"][0]["message"]["content"], "Xin chào thế giới");
        assert_eq!(val["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn test_sse_state_parser() {
        let mut parser = OneMinSseState::new();

        assert_eq!(parser.process_line("event: content", "gpt-4o"), None);
        let chunk = parser.process_line("data: {\"content\": \"Hello\"}", "gpt-4o");
        assert!(chunk.is_some());
        assert!(chunk.unwrap().contains("\"delta\":{\"content\":\"Hello\"}"));

        assert_eq!(parser.process_line("event: done", "gpt-4o"), None);
        let done_chunk = parser.process_line("data: {\"message\": \"Stream completed\"}", "gpt-4o");
        assert!(done_chunk.is_some());
        let s = done_chunk.unwrap();
        assert!(s.contains("\"finish_reason\":\"stop\""));
        assert!(s.contains("data: [DONE]"));
    }

    #[test]
    fn test_sse_transformer_feed_and_finish() {
        let mut transformer = OneMinSseTransformer::new("gpt-4o");
        let chunk1 = b"event: content\ndata: {\"content\": \"Xin ch";
        let out1 = transformer.feed_bytes(chunk1);
        // "Xin ch" hasn't got newline yet, so buffer keeps it
        assert!(out1.is_empty());

        let chunk2 = b"ao!\"}\n\nevent: done\ndata: {}\n\n";
        let out2 = transformer.feed_bytes(chunk2);
        let s2 = String::from_utf8(out2).unwrap();
        assert!(s2.contains("\"delta\":{\"content\":\"Xin chao!\"}"));
        assert!(s2.contains("\"finish_reason\":\"stop\""));
        assert!(s2.contains("data: [DONE]"));

        let out3 = transformer.finish();
        // Since done was already emitted, finish won't duplicate done
        assert!(out3.is_empty());
    }

    #[test]
    fn test_get_models_list_json() {
        let models = get_models_list_json();
        assert_eq!(models["object"], "list");
        let arr = models["data"].as_array().unwrap();
        assert!(arr.iter().any(|m| m["id"] == "gpt-4o"));
        assert!(arr.iter().any(|m| m["id"] == "claude-3-5-sonnet"));
        assert!(arr.iter().any(|m| m["id"] == "gemini-2.5-flash"));
        assert!(arr.iter().any(|m| m["id"] == "deepseek-chat"));
    }
}
