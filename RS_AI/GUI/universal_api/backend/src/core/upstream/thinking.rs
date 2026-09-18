//! Thinking/reasoning request transformations.

use serde_json::{Map, Value};

const DEFAULT_BUDGET: i64 = 10_000;

pub fn is_deep_seek_model(model: &str) -> bool {
    let lower = model.to_lowercase();
    lower.contains("deepseek") && (lower.contains("think") || lower.contains("reason"))
}

pub fn inject_thinking(obj: &mut Map<String, Value>) {
    let model = obj.get("model").and_then(Value::as_str).unwrap_or_default();
    if !is_deep_seek_model(model) || obj.contains_key("thinking") { return; }
    obj.insert("thinking".into(), serde_json::json!({"type":"enabled", "budget_tokens":DEFAULT_BUDGET}));
    obj.remove("temperature");
}

pub fn backfill_reasoning_content(obj: &mut Map<String, Value>) {
    let Some(messages) = obj.get_mut("messages").and_then(Value::as_array_mut) else { return; };
    for message in messages {
        let Some(map) = message.as_object_mut() else { continue; };
        if map.get("role").and_then(Value::as_str) != Some("assistant") { continue; }
        let empty = map.get("content").map(|v| v.as_str() == Some("")).unwrap_or(true);
        if empty {
            if let Some(reasoning) = map.get("reasoning_content").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                map.insert("content".into(), Value::String(format!("<think>\n{}\n</think>", reasoning)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_reasoning_models() {
        assert!(is_deep_seek_model("deepseek-reasoner"));
        assert!(!is_deep_seek_model("deepseek-v2"));
    }
}
