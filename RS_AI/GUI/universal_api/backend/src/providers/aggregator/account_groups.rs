//! Account groups management (global and per-platform groups).

use std::path::{Path, PathBuf};

#[tauri::command]
pub fn load_account_groups() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = Path::new(&home).join(".cockpit_tools").join("account_groups.json");
    if !path.exists() {
        return Ok("[]".to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read groups: {}", e))
}

#[tauri::command]
pub fn save_account_groups(data: String) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = Path::new(&home).join(".cockpit_tools").join("account_groups.json");
    let _parsed: Vec<serde_json::Value> = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid groups JSON array: {}", e))?;
    crate::core::secure_account_storage::write_string_atomic(&path, &data)
}

pub fn resolve_platform_groups_path(home: &str, platform: &str) -> PathBuf {
    let cockpit_dir = Path::new(home).join(".cockpit_tools");
    let lower = platform.trim().to_ascii_lowercase();
    let filename = match lower.as_str() {
        "antigravity" | "gemini" => "account_groups.json".to_string(),
        "codex" => "codex_account_groups.json".to_string(),
        "claude" => "claude_manager_account_groups.json".to_string(),
        other => {
            let sanitized: String = other
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
                .collect();
            format!("{}_account_groups.json", sanitized)
        }
    };
    cockpit_dir.join(filename)
}

#[tauri::command]
pub fn load_platform_account_groups(platform: String) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = resolve_platform_groups_path(&home, &platform);
    if !path.exists() {
        return Ok("[]".to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read platform groups for {}: {}", platform, e))
}

#[tauri::command]
pub fn save_platform_account_groups(platform: String, data: String) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = resolve_platform_groups_path(&home, &platform);
    let _parsed: Vec<serde_json::Value> = serde_json::from_str(&data)
        .map_err(|e| format!("Invalid platform groups JSON array: {}", e))?;
    crate::core::secure_account_storage::write_string_atomic(&path, &data)
}
