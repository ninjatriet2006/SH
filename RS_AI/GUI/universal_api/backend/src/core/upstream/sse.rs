//! SSE helpers for upstream chat streams.

use std::io::{BufRead, BufReader, Read};

use serde_json::{json, Value};

/// Aggregate an OpenAI-compatible SSE stream into one completion response.
pub fn aggregate<R: Read>(reader: R) -> Result<Value, String> {
    let mut id = String::new();
    let mut model = String::new();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut finish_reason = "stop".to_string();
    let mut valid_events = 0usize;
    let mut tools: Vec<Value> = Vec::new();

    for line in BufReader::new(reader).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let Some(payload) = line.strip_prefix("data: ") else { continue };
        if payload.trim() == "[DONE]" { break; }
        let Ok(chunk) = serde_json::from_str::<Value>(payload) else { continue };
        valid_events += 1;
        if id.is_empty() { id = chunk["id"].as_str().unwrap_or_default().to_string(); }
        if model.is_empty() { model = chunk["model"].as_str().unwrap_or_default().to_string(); }
        if let Some(choice) = chunk["choices"].as_array().and_then(|v| v.first()) {
            if let Some(reason) = choice["finish_reason"].as_str() {
                if !reason.is_empty() { finish_reason = reason.to_string(); }
            }
            let delta = &choice["delta"];
            if let Some(text) = delta["content"].as_str() { content.push_str(text); }
            if let Some(text) = delta["reasoning_content"].as_str() { reasoning.push_str(text); }
            if let Some(calls) = delta["tool_calls"].as_array() {
                for call in calls {
                    let index = call["index"].as_u64().unwrap_or(tools.len() as u64) as usize;
                    while tools.len() <= index { tools.push(json!({"index": tools.len()})); }
                    merge_tool(&mut tools[index], call);
                }
            }
        }
    }
    if valid_events == 0 { return Err("upstream stream contained no valid data events".into()); }
    let mut message = json!({"role":"assistant", "content":content});
    if !reasoning.is_empty() { message["reasoning_content"] = json!(reasoning); }
    if !tools.is_empty() { message["tool_calls"] = Value::Array(tools); }
    Ok(json!({
        "id": if id.is_empty() { format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis()) } else { id },
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{"index":0,"message":message,"finish_reason":finish_reason}]
    }))
}

fn merge_tool(target: &mut Value, delta: &Value) {
    if let Some(obj) = target.as_object_mut() {
        for key in ["id", "type"] {
            if let Some(value) = delta.get(key).filter(|v| !v.is_null()) { obj.insert(key.into(), value.clone()); }
        }
        if let Some(function) = delta.get("function").and_then(Value::as_object) {
            let target_fn = obj.entry("function").or_insert_with(|| json!({}));
            if let Some(target_fn) = target_fn.as_object_mut() {
                if let Some(name) = function.get("name") { target_fn.insert("name".into(), name.clone()); }
                if let Some(args) = function.get("arguments").and_then(Value::as_str) {
                    let previous = target_fn.get("arguments").and_then(Value::as_str).unwrap_or_default();
                    target_fn.insert("arguments".into(), json!(format!("{}{}", previous, args)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::aggregate;
    use std::io::Cursor;

    #[test]
    fn aggregates_content_and_finish_reason() {
        let input = concat!(
            "data: {\"id\":\"chatcmpl-1\",\"model\":\"glm-5.2\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" world\"},\"finish_reason\":\"stop\"}]}\n",
            "data: [DONE]\n",
        );
        let result = aggregate(Cursor::new(input)).unwrap();

        assert_eq!(result["id"], "chatcmpl-1");
        assert_eq!(result["model"], "glm-5.2");
        assert_eq!(result["choices"][0]["message"]["content"], "Hello world");
        assert_eq!(result["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn aggregates_reasoning_content() {
        let input = r#"data: {"choices":[{"delta":{"reasoning_content":"thinking"}}]}
data: {"choices":[{"delta":{"reasoning_content":"..."}}]}
data: [DONE]
"#;
        let result = aggregate(Cursor::new(input)).unwrap();
        let message = &result["choices"][0]["message"];

        assert_eq!(message["reasoning_content"], "thinking...");
    }

    #[test]
    fn aggregates_tool_arguments() {
        let input = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"lookup","arguments":"{"}}]}}]}
data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"q\":\"rust\"}"}}]}}]}
data: [DONE]
"#;
        let result = aggregate(Cursor::new(input)).unwrap();
        let message = &result["choices"][0]["message"];

        assert_eq!(message["tool_calls"][0]["id"], "call-1");
        assert_eq!(message["tool_calls"][0]["function"]["name"], "lookup");
        assert_eq!(
            message["tool_calls"][0]["function"]["arguments"],
            "{\"q\":\"rust\"}"
        );
    }

    #[test]
    fn rejects_stream_without_valid_data_events() {
        let result = aggregate(Cursor::new("event: ping\n\ndata: [DONE]\n"));

        assert!(result.unwrap_err().contains("no valid data events"));
    }
}
