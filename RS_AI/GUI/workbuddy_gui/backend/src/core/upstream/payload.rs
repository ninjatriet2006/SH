//! Request body rewriting: enforce stream:true, normalise tool_choice, roles,
//! inject thinking, normalise reasoning_effort, backfill reasoning_content,
//! and optionally sanitise fingerprints.
//!
//! Ported from Go: internal/upstream/payload.go

use std::collections::HashMap;

/// Rewrite chat request body in a single pass.
pub fn prepare_body_with_efforts(
    src: &[u8],
    sanitize: bool,
    efforts: Option<&HashMap<String, Vec<String>>>,
) -> Vec<u8> {
    if src.is_empty() {
        return src.to_vec();
    }
    let mut obj: serde_json::Map<String, serde_json::Value> = match serde_json::from_slice(src) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => return src.to_vec(),
    };

    // Force stream: true
    obj.insert("stream".into(), serde_json::Value::Bool(true));

    normalize_tool_choice(&mut obj);
    normalize_roles(&mut obj);

    // DeepSeek thinking injection (before effort normalisation)
    super::thinking::inject_thinking(&mut obj);
    if let Some(eff) = efforts {
        normalize_reasoning_effort(&mut obj, eff);
    }
    super::thinking::backfill_reasoning_content(&mut obj);

    if sanitize {
        if let Some(serde_json::Value::Array(msgs)) = obj.get_mut("messages") {
            super::sanitize::sanitize_messages(msgs);
        }
    }

    serde_json::to_vec(&obj).unwrap_or_else(|_| src.to_vec())
}

/// Convenience without efforts.
pub fn prepare_body(src: &[u8], sanitize: bool) -> Vec<u8> {
    prepare_body_with_efforts(src, sanitize, None)
}

// ── Effort rank table ──────────────────────────────────────────────────────

fn effort_rank(s: &str) -> Option<i32> {
    match s.trim().to_lowercase().as_str() {
        "off" => Some(0),
        "minimal" => Some(1),
        "low" => Some(2),
        "medium" => Some(3),
        "high" => Some(4),
        "xhigh" => Some(5),
        "max" => Some(6),
        _ => None,
    }
}

/// Downgrade reasoning_effort based on model's supportedEfforts.
fn normalize_reasoning_effort(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    efforts: &HashMap<String, Vec<String>>,
) {
    if efforts.is_empty() {
        return;
    }
    let model = obj.get("model").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    if model.is_empty() {
        return;
    }
    let supported = match efforts.get(&model) {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };

    // Find the key (snake or camel)
    let key = if obj.contains_key("reasoning_effort") {
        "reasoning_effort"
    } else if obj.contains_key("reasoningEffort") {
        "reasoningEffort"
    } else {
        return;
    };

    let req_str = match obj.get(key).and_then(|v| v.as_str()) {
        Some(s) => s.trim().to_lowercase(),
        None => return,
    };
    let req_idx = match effort_rank(&req_str) {
        Some(r) => r,
        None => return,
    };

    // Best supported <= requested
    let mut best: Option<(&str, i32)> = None;
    for s in supported {
        if let Some(idx) = effort_rank(s) {
            if idx <= req_idx {
                if best.is_none() || idx > best.unwrap().1 {
                    best = Some((s.as_str(), idx));
                }
            }
        }
    }
    if let Some((b, _)) = best {
        if !b.eq_ignore_ascii_case(&req_str) {
            log::info!("reasoning_effort downgraded model={} {} -> {}", model, req_str, b);
            obj.insert(key.to_string(), serde_json::Value::String(b.to_string()));
        }
        return;
    }

    // All supported > requested → pick lowest
    let mut lowest: Option<(&str, i32)> = None;
    for s in supported {
        if let Some(idx) = effort_rank(s) {
            if lowest.is_none() || idx < lowest.unwrap().1 {
                lowest = Some((s.as_str(), idx));
            }
        }
    }
    if let Some((l, _)) = lowest {
        log::info!("reasoning_effort floored model={} {} -> {}", model, req_str, l);
        obj.insert(key.to_string(), serde_json::Value::String(l.to_string()));
    }
}

/// Normalise 'developer' role → 'system' (upstream whitelist compatibility).
fn normalize_roles(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let msgs = match obj.get_mut("messages").and_then(|v| v.as_array_mut()) {
        Some(m) => m,
        None => return,
    };
    for (i, m) in msgs.iter_mut().enumerate() {
        if let Some(role) = m.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            if role.trim().eq_ignore_ascii_case("developer") {
                m.as_object_mut().unwrap().insert("role".into(), "system".into());
                log::info!("role normalized developer->system idx={}", i);
            }
        }
    }
}

/// Normalise tool_choice from OpenAI object format to upstream string format.
fn normalize_tool_choice(obj: &mut serde_json::Map<String, serde_json::Value>) {
    let suppress = |o: &mut serde_json::Map<String, serde_json::Value>| {
        o.remove("tools");
        o.remove("functions");
    };

    let tc = match obj.get("tool_choice").cloned() {
        Some(v) => v,
        None => return,
    };

    match &tc {
        serde_json::Value::String(s) => {
            if s.trim().eq_ignore_ascii_case("none") {
                obj.remove("tool_choice");
                suppress(obj);
            }
        }
        serde_json::Value::Object(m) => {
            let typ = m.get("type").and_then(|v| v.as_str()).unwrap_or_default().trim().to_lowercase();
            match typ.as_str() {
                "none" => {
                    obj.remove("tool_choice");
                    suppress(obj);
                }
                "auto" | "required" => {
                    obj.insert("tool_choice".into(), serde_json::Value::String(typ));
                }
                "function" => {
                    let name = m.get("function")
                        .and_then(|v| v.as_object())
                        .and_then(|f| f.get("name"))
                        .and_then(|v| v.as_str())
                        .or_else(|| m.get("name").and_then(|v| v.as_str()))
                        .unwrap_or_default()
                        .trim();
                    let val = if name.is_empty() { "auto" } else { name };
                    obj.insert("tool_choice".into(), serde_json::Value::String(val.to_string()));
                }
                _ => {
                    obj.remove("tool_choice");
                }
            }
        }
        _ => {
            obj.remove("tool_choice");
        }
    }
}
