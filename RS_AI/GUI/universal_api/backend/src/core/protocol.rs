//! Typed schemas and translation between OpenAI (`/v1/chat/completions`) and
//! Anthropic (`/v1/messages`) protocols.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ─── OpenAI Protocol ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(default = "default_true")]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub stop: Option<Value>,
    #[serde(default)]
    pub presence_penalty: Option<f64>,
    #[serde(default)]
    pub frequency_penalty: Option<f64>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: Value,
    #[serde(default)]
    pub name: Option<String>,
}

impl OpenAIMessage {
    pub fn text_content(&self) -> String {
        match &self.content {
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let mut out = String::new();
                for item in arr {
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        out.push_str(text);
                    }
                }
                out
            }
            other => other.to_string(),
        }
    }
}

// ─── Anthropic Protocol ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u64,
    #[serde(default)]
    pub system: Option<Value>, // string or array of text blocks
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub top_k: Option<u64>,
    #[serde(default)]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnthropicMessage {
    pub role: String, // "user" or "assistant"
    pub content: Value, // string or array of content blocks
}

impl AnthropicMessagesRequest {
    /// Convert an Anthropic `/v1/messages` request into an OpenAI-compatible
    /// `OpenAIChatRequest` so it can reuse the existing upstream pipeline.
    pub fn to_openai(&self) -> OpenAIChatRequest {
        let mut messages = Vec::new();

        // System prompt
        if let Some(sys) = &self.system {
            let sys_text = match sys {
                Value::String(s) => s.clone(),
                Value::Array(blocks) => {
                    let mut text = String::new();
                    for b in blocks {
                        if let Some(t) = b.get("text").and_then(|s| s.as_str()) {
                            text.push_str(t);
                        }
                    }
                    text
                }
                other => other.to_string(),
            };
            if !sys_text.is_empty() {
                messages.push(OpenAIMessage {
                    role: "system".into(),
                    content: Value::String(sys_text),
                    name: None,
                });
            }
        }

        // Messages
        for m in &self.messages {
            let text = match &m.content {
                Value::String(s) => s.clone(),
                Value::Array(blocks) => {
                    let mut combined = String::new();
                    for b in blocks {
                        if let Some(t) = b.get("text").and_then(|s| s.as_str()) {
                            combined.push_str(t);
                        }
                    }
                    combined
                }
                other => other.to_string(),
            };
            messages.push(OpenAIMessage {
                role: m.role.clone(),
                content: Value::String(text),
                name: None,
            });
        }

        OpenAIChatRequest {
            model: self.model.clone(),
            messages,
            stream: self.stream,
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: Some(self.max_tokens),
            stop: self
                .stop_sequences
                .as_ref()
                .map(|s| Value::Array(s.iter().map(|x| Value::String(x.clone())).collect())),
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            extra: self.extra.clone(),
        }
    }
}

// ─── Anthropic SSE Event Builders ────────────────────────────────────────────

pub fn anthropic_message_start(id: &str, model: &str) -> String {
    json!({
        "type": "message_start",
        "message": {
            "id": id,
            "type": "message",
            "role": "assistant",
            "content": [],
            "model": model,
            "stop_reason": null,
            "stop_sequence": null,
            "usage": { "input_tokens": 0, "output_tokens": 0 }
        }
    })
    .to_string()
}

pub fn anthropic_content_block_start(index: usize) -> String {
    json!({
        "type": "content_block_start",
        "index": index,
        "content_block": { "type": "text", "text": "" }
    })
    .to_string()
}

pub fn anthropic_content_block_delta(index: usize, text: &str) -> String {
    json!({
        "type": "content_block_delta",
        "index": index,
        "delta": { "type": "text_delta", "text": text }
    })
    .to_string()
}

pub fn anthropic_content_block_stop(index: usize) -> String {
    json!({
        "type": "content_block_stop",
        "index": index
    })
    .to_string()
}

pub fn anthropic_message_delta(output_tokens: u64) -> String {
    json!({
        "type": "message_delta",
        "delta": { "stop_reason": "end_turn", "stop_sequence": null },
        "usage": { "output_tokens": output_tokens }
    })
    .to_string()
}

pub fn anthropic_message_stop() -> String {
    json!({ "type": "message_stop" }).to_string()
}

/// Format an SSE frame per spec: `data: <json>\n\n`.
pub fn format_sse_event(event_type: Option<&str>, data: &str) -> String {
    match event_type {
        Some(ev) => format!("event: {}\ndata: {}\n\n", ev, data),
        None => format!("data: {}\n\n", data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_real_openai_payload() {
        let raw = r#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello, world!"}
            ],
            "temperature": 0.7,
            "max_tokens": 1024,
            "stream": true
        }"#;
        let parsed: Result<OpenAIChatRequest, _> = serde_json::from_str(raw);
        assert!(parsed.is_ok(), "OpenAI payload must parse: {:?}", parsed.err());
        let req = parsed.unwrap();
        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].text_content(), "You are helpful.");
        assert_eq!(req.messages[1].text_content(), "Hello, world!");
        assert!(req.stream);
        assert_eq!(req.max_tokens, Some(1024));
    }

    #[test]
    fn parse_real_anthropic_payload() {
        let raw = r#"{
            "model": "claude-3-7-sonnet-20250219",
            "max_tokens": 2048,
            "system": "You are Claude.",
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "Write a poem"}]}
            ],
            "stream": true,
            "temperature": 0.5
        }"#;
        let parsed: Result<AnthropicMessagesRequest, _> = serde_json::from_str(raw);
        assert!(parsed.is_ok(), "Anthropic payload must parse: {:?}", parsed.err());
        let req = parsed.unwrap();
        assert_eq!(req.model, "claude-3-7-sonnet-20250219");
        assert_eq!(req.max_tokens, 2048);
        assert!(req.stream);

        // Test conversion
        let oai = req.to_openai();
        assert_eq!(oai.messages.len(), 2);
        assert_eq!(oai.messages[0].role, "system");
        assert_eq!(oai.messages[0].text_content(), "You are Claude.");
        assert_eq!(oai.messages[1].role, "user");
        assert_eq!(oai.messages[1].text_content(), "Write a poem");
        assert_eq!(oai.max_tokens, Some(2048));
    }

    #[test]
    fn sse_event_formatting() {
        let frame = format_sse_event(None, r#"{"chunk":"hello"}"#);
        assert_eq!(frame, "data: {\"chunk\":\"hello\"}\n\n");
        assert!(frame.starts_with("data: "));
        assert!(frame.ends_with("\n\n"));

        let typed = format_sse_event(Some("content_block_delta"), r#"{"text":"hi"}"#);
        assert_eq!(typed, "event: content_block_delta\ndata: {\"text\":\"hi\"}\n\n");
    }

    #[test]
    fn anthropic_sse_event_builders_valid_json() {
        let start = anthropic_message_start("msg_1", "claude-3");
        assert!(serde_json::from_str::<Value>(&start).is_ok());

        let delta = anthropic_content_block_delta(0, "partial");
        let v: Value = serde_json::from_str(&delta).unwrap();
        assert_eq!(v["delta"]["text"], "partial");

        let stop = anthropic_message_stop();
        assert_eq!(serde_json::from_str::<Value>(&stop).unwrap()["type"], "message_stop");
    }
}
