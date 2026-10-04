//! Antigravity Account Switching & History Tracking.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AntigravitySwitchHistoryItem {
    pub id: String,
    pub timestamp: i64,
    pub account_id: String,
    pub target_email: String,
    #[serde(default = "default_history_trigger_type")]
    pub trigger_type: String,
    #[serde(default = "default_history_trigger_source")]
    pub trigger_source: String,
    pub local_ok: bool,
    pub seamless_ok: bool,
    pub success: bool,
    pub local_duration_ms: u64,
    pub seamless_duration_ms: Option<u64>,
    pub total_duration_ms: u64,
    pub error_stage: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub seamless_effective_mode: Option<String>,
    pub seamless_from_email: Option<String>,
    pub seamless_to_email: Option<String>,
    pub seamless_execution_id: Option<String>,
    pub seamless_finished_at: Option<String>,
}

fn default_history_trigger_type() -> String {
    "manual".to_string()
}

fn default_history_trigger_source() -> String {
    "tools.account.switch".to_string()
}

pub fn add_antigravity_switch_history(cockpit_dir: &Path, account_id: &str, target_email: &str, success: bool) {
    let history_file = cockpit_dir.join("antigravity_switch_history.json");
    let mut items: Vec<AntigravitySwitchHistoryItem> = if let Ok(content) = std::fs::read_to_string(&history_file) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    let now_ms = chrono::Utc::now().timestamp_millis();
    let item = AntigravitySwitchHistoryItem {
        id: format!("switch_{}", now_ms),
        timestamp: chrono::Utc::now().timestamp(),
        account_id: account_id.to_string(),
        target_email: target_email.to_string(),
        trigger_type: "manual".to_string(),
        trigger_source: "tools.account.switch".to_string(),
        local_ok: success,
        seamless_ok: success,
        success,
        local_duration_ms: 150,
        seamless_duration_ms: None,
        total_duration_ms: 150,
        error_stage: None,
        error_code: None,
        error_message: None,
        seamless_effective_mode: None,
        seamless_from_email: None,
        seamless_to_email: None,
        seamless_execution_id: None,
        seamless_finished_at: None,
    };

    items.insert(0, item);
    if items.len() > 200 {
        items.truncate(200);
    }

    if let Ok(serialized) = serde_json::to_string_pretty(&items) {
        let _ = crate::core::secure_account_storage::write_string_atomic(&history_file, &serialized);
    }
}

#[tauri::command]
pub fn load_antigravity_switch_history() -> Result<Vec<AntigravitySwitchHistoryItem>, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let history_file = std::path::Path::new(&home).join(".cockpit_tools").join("antigravity_switch_history.json");
    if !history_file.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&history_file)
        .map_err(|e| format!("Failed to read switch history: {}", e))?;
    let list: Vec<AntigravitySwitchHistoryItem> = serde_json::from_str(&content)
        .unwrap_or_default();
    Ok(list)
}

#[tauri::command]
pub fn clear_antigravity_switch_history() -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let history_file = std::path::Path::new(&home).join(".cockpit_tools").join("antigravity_switch_history.json");
    let _ = crate::core::secure_account_storage::write_string_atomic(&history_file, "[]");
    Ok(())
}
