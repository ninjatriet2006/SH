//! Sanitise request body — strip blacklisted fingerprints from messages.
//!
//! Ported from Go: internal/upstream/sanitize.go
//!
//! Strategy: strip key-value/header fingerprints entirely; for semantic
//! phrases, do minimal 1-word rewrites to preserve meaning.

use std::sync::LazyLock;
use regex::Regex;

// ── Feature strings for fast pre-check ─────────────────────────────────────

const FEATURES: &[&str] = &[
    "x-anthropic-billing-header",
    "cc_entrypoint=",
    "You are Claude Code",
    "Main branch (",
    "You are a coding agent running in the Codex CLI",
];

// ── Compiled regexes (initialised once) ────────────────────────────────────

static HDR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)x-anthropic-billing-header:[^;\n]*;?\s*").unwrap()
});

static KV_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bcc_[a-z0-9_]+=[^;\n]*;?\s*").unwrap()
});

// ── Phrase rewrites (1-word change each) ───────────────────────────────────

const REWRITES: &[(&str, &str)] = &[
    (
        "You are Claude Code, Anthropic's official CLI for Claude.",
        "You are Claude Code, Anthropic's official CLI tool for Claude.",
    ),
    (
        "Main branch (you will usually use this for PRs)",
        "Default branch (you will usually use this for PRs)",
    ),
    (
        "You are a coding agent running in the Codex CLI, a terminal-based coding assistant.",
        "You are a coding agent running in the Codex CLI tool, a terminal-based coding assistant.",
    ),
];

/// Quick check: does the text contain any fingerprint indicator?
fn has_fingerprint(text: &str) -> bool {
    for f in FEATURES {
        if text.contains(f) {
            return true;
        }
    }
    HDR_RE.is_match(text)
}

/// Sanitise a single text string.
pub fn sanitize_text(text: &str) -> String {
    if !has_fingerprint(text) {
        return text.to_string();
    }
    let mut out = text.to_string();
    for (from, to) in REWRITES {
        out = out.replace(from, to);
    }
    if HDR_RE.is_match(&out) {
        out = HDR_RE.replace_all(&out, "").to_string();
    }
    if out.contains("cc_") {
        let mut prev = String::new();
        while prev != out {
            prev = out.clone();
            out = KV_RE.replace_all(&out, "").to_string();
        }
    }
    out.trim().to_string()
}

/// Sanitise the `content` field of a message (string or multimodal array).
fn sanitize_content(v: &mut serde_json::Value) -> bool {
    match v {
        serde_json::Value::String(s) => {
            let cleaned = sanitize_text(s);
            if cleaned != *s {
                *s = cleaned;
                return true;
            }
            false
        }
        serde_json::Value::Array(arr) => {
            let mut changed = false;
            for part in arr.iter_mut() {
                if let Some(obj) = part.as_object_mut() {
                    if let Some(serde_json::Value::String(text)) = obj.get_mut("text") {
                        let cleaned = sanitize_text(text);
                        if cleaned != *text {
                            *text = cleaned;
                            changed = true;
                        }
                    }
                }
            }
            changed
        }
        _ => false,
    }
}

/// Sanitise `content` in all messages; returns true if anything changed.
pub fn sanitize_messages(messages: &mut Vec<serde_json::Value>) -> bool {
    let mut changed = false;
    for msg in messages.iter_mut() {
        if let Some(obj) = msg.as_object_mut() {
            if let Some(content) = obj.get_mut("content") {
                if sanitize_content(content) {
                    changed = true;
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_fingerprint_passthrough() {
        let text = "Hello world";
        assert_eq!(sanitize_text(text), text);
    }

    #[test]
    fn test_rewrite() {
        let text = "You are Claude Code, Anthropic's official CLI for Claude.";
        let out = sanitize_text(text);
        assert!(out.contains("CLI tool"));
    }

    #[test]
    fn test_strip_header() {
        let text = "prefix x-anthropic-billing-header:foo=bar; suffix";
        let out = sanitize_text(text);
        assert!(!out.contains("anthropic-billing"));
        assert!(out.contains("prefix"));
    }
}
