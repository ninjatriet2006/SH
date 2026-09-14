//! System prompt management — ported from Go internal/prompt.

pub const DEGRADED: &str = "You are a helpful assistant. Respond in the user's language, follow the user's instructions, and be direct and concise.";

#[derive(Debug, Clone)]
pub enum PromptMode {
    Passthrough,
    Custom(String),
}

const DEFAULT_PROMPT: &str = include_str!("../../defaultprompt.md");

pub fn load_prompt(mode: &str, file: &str) -> Result<String, String> {
    match mode {
        "passthrough" => Ok(String::new()),
        "custom" | _ => {
            if file.is_empty() {
                Ok(DEFAULT_PROMPT.to_string())
            } else {
                std::fs::read_to_string(file)
                    .map_err(|e| format!("Cannot read prompt file '{}': {}", file, e))
            }
        }
    }
}
