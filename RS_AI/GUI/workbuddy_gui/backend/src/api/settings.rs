use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    pub language: String,
    pub theme: String,
    pub font: String,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "default".to_string(),
            font: "default".to_string(),
        }
    }
}

fn settings_path() -> std::path::PathBuf {
    crate::core::resources::resource_base().join("gui_settings.json")
}

fn load_settings() -> GuiSettings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => GuiSettings::default(),
    }
}

fn save_settings_inner(settings: &GuiSettings) -> Result<(), String> {
    let path = settings_path();
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_gui_settings(request: Req<Empty>) -> IpcResult<GuiSettings> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, load_settings()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_gui_settings(request: Req<GuiSettings>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    save_settings_inner(&payload).map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}
