use crate::ipc::{respond, from_string, Empty, IpcResult, Req};
use serde::Deserialize;
use tauri::State;
use crate::core::runtime::RuntimeState;

#[tauri::command(rename_all = "snake_case")]
pub fn get_config(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<serde_json::Value> {
    let (request_id, _) = request.validate()?;
    let config = state.config.lock().map_err(|_| from_string("Config lock poisoned".into()))?;
    let val = serde_json::to_value(&*config).map_err(|e| from_string(e.to_string()))?;
    Ok(respond(request_id, val))
}

#[derive(Deserialize)]
pub struct SaveConfigRequest {
    pub config_json: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_config(request: Req<SaveConfigRequest>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let config: crate::core::config::Config = serde_json::from_str(&payload.config_json)
        .map_err(|e| from_string(format!("Invalid config JSON: {}", e)))?;
    crate::core::config::save_config(&config).map_err(from_string)?;
    *state.config.lock().map_err(|_| from_string("Config lock poisoned".into()))? = config;
    Ok(respond(request_id, Empty {}))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_default_config(request: Req<Empty>) -> IpcResult<serde_json::Value> {
    let (request_id, _) = request.validate()?;
    let default = crate::core::config::Config::default();
    let val = serde_json::to_value(&default).map_err(|e| from_string(e.to_string()))?;
    Ok(respond(request_id, val))
}
