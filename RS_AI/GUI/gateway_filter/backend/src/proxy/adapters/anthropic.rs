//! Anthropic Claude Adapter: Chuyển đổi 2 chiều giữa chuẩn OpenAI Protocol (Local)
//! và Anthropic Messages API (Upstream).

use serde_json::{json, Value};
use uuid::Uuid;

/// Ánh xạ tên model phổ thông ở Local sang modelId chuẩn của Anthropic.
pub fn resolve_model_id(input_model: &str) -> String {
    let m = input_model.trim().to_lowercase();
    match m.as_str() {
        "claude-3-5-sonnet" | "claude-3.5-sonnet" | "claude-3-5-sonnet-latest" => {
            "claude-3-5-sonnet-20241022".to_string()
        }
        "claude-3-7-sonnet" | "claude-3.7-sonnet" | "claude-3-7-sonnet-latest" => {
            "claude-3-7-sonnet-20250219".to_string()
        }
        "claude-3-5-haiku" | "claude-3.5-haiku" | "claude-3-5-haiku-latest" => {
            "claude-3-5-haiku-20241022".to_string()
        }
        "claude-3-opus" | "claude-3-opus-latest" => "claude-3-opus-20240229".to_string(),

        // Nếu client OpenAI gửi model OpenAI sang route Anthropic -> fallback Sonnet 3.5
        "gpt-4o" | "gpt-4o-mini" | "gpt-4-turbo" | "gpt-4" => "claude-3-5-sonnet-20241022".to_string(),

        _ => input_model.trim().to_string(),
    }
}

/// Trả về danh sách Claude models chuẩn OpenAI list format
pub fn get_models_list_json() -> Value {
    json!({
        "object": "list",
        "data": [
            {"id": "claude-3-5-sonnet", "object": "model", "owned_by": "anthropic"},
            {"id": "claude-3-7-sonnet", "object": "model", "owned_by": "anthropic"},
            {"id": "claude-3-5-haiku", "object": "model", "owned_by": "anthropic"},
            {"id": "claude-3-opus", "object": "model", "owned_by": "anthropic"}
        ]
    })
}

/// Xây dựng URL endpoint Anthropic Messages từ URL gốc
pub fn build_anthropic_target_url(original_url: &str) -> String {
    let trimmed = original_url.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        let base = trimmed.strip_suffix("/chat/completions").unwrap();
        format!("{}/messages", base)
    } else if trimmed.ends_with("/v1") {
        format!("{}/messages", trimmed)
    } else if trimmed.ends_with("/messages") {
        trimmed.to_string()
    } else {
        format!("{}/v1/messages", trimmed)
    }
}

/// Chuyển đổi OpenAI Chat Completion Request Payload sang Anthropic Messages format.
/// Trả về: (new_target_url, new_body_bytes, is_streaming, requested_model)
pub fn transform_request(
    original_target_url: &str,
    raw_bytes: &[u8],
) -> Result<(String, Vec<u8>, bool, String), String> {
    let json_val: Value = serde_json::from_slice(raw_bytes)
        .map_err(|e| format!("Invalid JSON request body: {}", e))?;

    let model_raw = json_val["model"].as_str().unwrap_or("claude-3-5-sonnet");
    let resolved_model = resolve_model_id(model_raw);
    let stream = json_val["stream"].as_bool().unwrap_or(false);
    let max_tokens = json_val["max_tokens"]
        .as_u64()
        .or_else(|| json_val["max_completion_tokens"].as_u64())
        .unwrap_or(4096);

    let mut system_text = String::new();
    let mut anthropic_messages = Vec::new();

    if let Some(msgs) = json_val["messages"].as_array() {
        for m in msgs {
            let role = m["role"].as_str().unwrap_or("");
            let content = match &m["content"] {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    let mut acc = String::new();
                    for item in arr {
                        if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                            acc.push_str(t);
                        }
                    }
                    acc
                }
                other => other.to_string(),
            };

            if role == "system" {
                if !system_text.is_empty() {
                    system_text.push('\n');
                }
                system_text.push_str(&content);
            } else if role == "user" || role == "assistant" {
                anthropic_messages.push(json!({
                    "role": role,
                    "content": content
                }));
            }
        }
    }

    let mut payload = json!({
        "model": resolved_model,
        "max_tokens": max_tokens,
        "messages": anthropic_messages,
        "stream": stream
    });

    if !system_text.is_empty() {
        payload["system"] = Value::String(system_text);
    }

    if let Some(temp) = json_val.get("temperature").and_then(|v| v.as_f64()) {
        payload["temperature"] = json!(temp);
    }

    let new_target_url = build_anthropic_target_url(original_target_url);
    let bytes = serde_json::to_vec(&payload)
        .map_err(|e| format!("Failed to serialize Anthropic payload: {}", e))?;

    Ok((new_target_url, bytes, stream, model_raw.to_string()))
}

/// Chuyển đổi Non-Streaming JSON Response từ Anthropic sang OpenAI ChatCompletion format.
pub fn transform_response_json(anthropic_bytes: &[u8], requested_model: &str) -> Result<Vec<u8>, String> {
    let v: Value = serde_json::from_slice(anthropic_bytes)
        .map_err(|e| format!("Invalid Anthropic response JSON: {}", e))?;

    if v.get("error").is_some() || v.get("type").and_then(|t| t.as_str()) == Some("error") {
        return Err(format!("Anthropic returned error: {}", String::from_utf8_lossy(anthropic_bytes)));
    }

    let id = v["id"].as_str().unwrap_or_else(|| "msg_unknown");
    let mut combined_content = String::new();

    if let Some(content_arr) = v["content"].as_array() {
        for block in content_arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = block.get("text").and_then(|s| s.as_str()) {
                    combined_content.push_str(t);
                }
            }
        }
    }

    let stop_reason = match v["stop_reason"].as_str() {
        Some("max_tokens") => "length",
        Some("stop_sequence") => "stop",
        _ => "stop",
    };

    let input_tokens = v["usage"]["input_tokens"].as_u64().unwrap_or(0);
    let output_tokens = v["usage"]["output_tokens"].as_u64().unwrap_or(0);

    let openai_resp = json!({
        "id": format!("chatcmpl-{}", id),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": requested_model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": combined_content
            },
            "finish_reason": stop_reason
        }],
        "usage": {
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens
        }
    });

    serde_json::to_vec(&openai_resp).map_err(|e| format!("Failed to serialize OpenAI response: {}", e))
}

/// Trạng thái phân tích SSE Stream từ Anthropic
#[derive(Debug, Clone)]
pub struct AnthropicSseState {
    pub stream_id: String,
    pub current_event: String,
}

impl Default for AnthropicSseState {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicSseState {
    pub fn new() -> Self {
        Self {
            stream_id: format!("chatcmpl-{}", Uuid::new_v4()),
            current_event: String::new(),
        }
    }

    /// Nhận 1 dòng text SSE từ Anthropic và chuyển đổi sang chuẩn OpenAI SSE chunk
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
                "content_block_delta" => {
                    if let Ok(v) = serde_json::from_str::<Value>(data_trimmed) {
                        if let Some(text_delta) = v["delta"]["text"].as_str() {
                            let chunk = json!({
                                "id": self.stream_id,
                                "object": "chat.completion.chunk",
                                "created": chrono::Utc::now().timestamp(),
                                "model": model,
                                "choices": [{
                                    "index": 0,
                                    "delta": {
                                        "content": text_delta
                                    },
                                    "finish_reason": null
                                }]
                            });
                            return Some(format!("data: {}\n\n", serde_json::to_string(&chunk).unwrap()));
                        }
                    }
                }
                "message_delta" => {
                    let finish_reason = serde_json::from_str::<Value>(data_trimmed)
                        .ok()
                        .and_then(|v| v["delta"]["stop_reason"].as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "stop".to_string());

                    let stop = if finish_reason == "max_tokens" { "length" } else { "stop" };

                    let finish_chunk = json!({
                        "id": self.stream_id,
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": model,
                        "choices": [{
                            "index": 0,
                            "delta": {},
                            "finish_reason": stop
                        }]
                    });
                    return Some(format!("data: {}\n\n", serde_json::to_string(&finish_chunk).unwrap()));
                }
                "message_stop" => {
                    return Some("data: [DONE]\n\n".to_string());
                }
                "error" => {
                    let err_msg = serde_json::from_str::<Value>(data_trimmed)
                        .ok()
                        .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(|s| s.to_string()))
                        .unwrap_or_else(|| data_trimmed.to_string());

                    let chunk = json!({
                        "id": self.stream_id,
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": model,
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": format!("\n[Anthropic Error: {}]", err_msg)
                            },
                            "finish_reason": "stop"
                        }]
                    });
                    return Some(format!("data: {}\n\ndata: [DONE]\n\n", serde_json::to_string(&chunk).unwrap()));
                }
                _ => {}
            }
        }

        None
    }
}

/// Transformer bọc Stream SSE từ Anthropic, đệm dòng đầy đủ và biến đổi sang OpenAI SSE
pub struct AnthropicSseTransformer {
    pub state: AnthropicSseState,
    pub buffer: String,
    pub model: String,
    pub done_emitted: bool,
}

impl AnthropicSseTransformer {
    pub fn new(model: &str) -> Self {
        Self {
            state: AnthropicSseState::new(),
            buffer: String::new(),
            model: model.to_string(),
            done_emitted: false,
        }
    }

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
    fn test_resolve_anthropic_model() {
        assert_eq!(resolve_model_id("claude-3-5-sonnet"), "claude-3-5-sonnet-20241022");
        assert_eq!(resolve_model_id("claude-3-7-sonnet"), "claude-3-7-sonnet-20250219");
        assert_eq!(resolve_model_id("claude-3-5-haiku"), "claude-3-5-haiku-20241022");
        assert_eq!(resolve_model_id("gpt-4o"), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_build_anthropic_target_url() {
        assert_eq!(build_anthropic_target_url("https://api.anthropic.com/v1/chat/completions"), "https://api.anthropic.com/v1/messages");
        assert_eq!(build_anthropic_target_url("https://api.anthropic.com/v1"), "https://api.anthropic.com/v1/messages");
        assert_eq!(build_anthropic_target_url("https://api.anthropic.com"), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn test_transform_anthropic_request() {
        let openai_req = json!({
            "model": "claude-3-5-sonnet",
            "messages": [
                {"role": "system", "content": "You are Claude, created by Anthropic."},
                {"role": "user", "content": "Hello Anthropic!"}
            ],
            "stream": true
        });

        let bytes = serde_json::to_vec(&openai_req).unwrap();
        let (url, transformed, stream, model) = transform_request("https://api.anthropic.com/v1/chat/completions", &bytes).unwrap();

        assert_eq!(model, "claude-3-5-sonnet");
        assert!(stream);
        assert_eq!(url, "https://api.anthropic.com/v1/messages");

        let val: Value = serde_json::from_slice(&transformed).unwrap();
        assert_eq!(val["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(val["system"], "You are Claude, created by Anthropic.");
        assert_eq!(val["max_tokens"], 4096);
        assert_eq!(val["messages"][0]["role"], "user");
        assert_eq!(val["messages"][0]["content"], "Hello Anthropic!");
    }

    #[test]
    fn test_transform_anthropic_response() {
        let anthropic_resp = json!({
            "id": "msg_test123",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Hello, Human!"}
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 8
            }
        });

        let bytes = serde_json::to_vec(&anthropic_resp).unwrap();
        let openai_bytes = transform_response_json(&bytes, "claude-3-5-sonnet").unwrap();
        let val: Value = serde_json::from_slice(&openai_bytes).unwrap();

        assert_eq!(val["id"], "chatcmpl-msg_test123");
        assert_eq!(val["choices"][0]["message"]["content"], "Hello, Human!");
        assert_eq!(val["usage"]["total_tokens"], 20);
    }

    #[test]
    fn test_anthropic_sse_transformer() {
        let mut transformer = AnthropicSseTransformer::new("claude-3-5-sonnet");
        let chunk1 = b"event: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"delta\": {\"type\": \"text_delta\", \"text\": \"Hi!\"}}\n\n";
        let out1 = transformer.feed_bytes(chunk1);
        let s1 = String::from_utf8(out1).unwrap();
        assert!(s1.contains("\"delta\":{\"content\":\"Hi!\"}"));

        let chunk2 = b"event: message_stop\ndata: {\"type\": \"message_stop\"}\n\n";
        let out2 = transformer.feed_bytes(chunk2);
        let s2 = String::from_utf8(out2).unwrap();
        assert!(s2.contains("data: [DONE]"));
    }
}
