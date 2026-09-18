use crate::core::external::{
    self, ExternalConfig, ExternalStatus, check_health, fetch_external_models, find_free_port,
    normalize_base_url, port_available,
};
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::Deserialize;
use tauri::State;

#[tauri::command(rename_all = "snake_case")]
pub fn get_external_config(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ExternalConfig> {
    let (request_id, _) = request.validate()?;
    let cfg = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .external
        .clone();
    Ok(respond(request_id, cfg))
}

#[derive(Deserialize)]
pub struct SaveExternalConfigRequest {
    pub config: ExternalConfig,
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_external_config(
    request: Req<SaveExternalConfigRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    // Validate trước khi lưu — bắt trùng port với gateway chính ngay tại UI.
    let main_listen = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .listen
        .clone();
    payload
        .config
        .validate(&main_listen)
        .map_err(from_string)?;
    {
        let mut guard = state
            .config
            .lock()
            .map_err(|_| from_string("Config lock poisoned".into()))?;
        guard.external = payload.config.clone();
        crate::core::config::save_config(&guard).map_err(from_string)?;
    }
    Ok(respond(request_id, Empty {}))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_external_status(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ExternalStatus> {
    let (request_id, _) = request.validate()?;
    let cfg = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .external
        .clone();
    if !cfg.enabled {
        return Ok(respond(
            request_id,
            ExternalStatus {
                reachable: false,
                http_status: None,
                latency_ms: None,
                error: Some("external disabled".into()),
            },
        ));
    }
    // check_health là blocking I/O — chạy trực tiếp ở command thread là đủ
    // vì Tauri command vốn chạy trên threadpool; không block tokio runtime.
    let status = check_health(&cfg);
    Ok(respond(request_id, status))
}

#[tauri::command(rename_all = "snake_case")]
pub fn refresh_external_models(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<serde_json::Value>> {
    let (request_id, _) = request.validate()?;
    let cfg = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .external
        .clone();
    if !cfg.enabled {
        return Err(from_string("external disabled".into()));
    }
    let models = fetch_external_models(&cfg).map_err(from_string)?;
    let values: Vec<serde_json::Value> = models
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "object": "model",
                "created": chrono::Utc::now().timestamp(),
                "owned_by": m.owned_by,
                "external": true,
            })
        })
        .collect();
    // Cache vào RuntimeState để `/v1/models` merge ngay cả trước khi restart gateway.
    if let Ok(mut cache) = state.external_models.lock() {
        *cache = values.clone();
    }
    Ok(respond(request_id, values))
}

#[derive(Deserialize)]
pub struct CheckPortRequest {
    pub port: u16,
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_port_available(
    request: Req<CheckPortRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<serde_json::Value> {
    let (request_id, payload) = request.validate()?;
    if payload.port == 0 {
        return Err(from_string("port must be 1..65535".into()));
    }
    let main_listen = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .listen
        .clone();
    let main_port = external::parse_port_from_listen(&main_listen);
    let available = port_available(payload.port);
    let conflicts_main = main_port == Some(payload.port);
    Ok(respond(
        request_id,
        serde_json::json!({
            "port": payload.port,
            "available": available && !conflicts_main,
            "conflicts_gateway": conflicts_main,
            "gateway_listen": main_listen,
        }),
    ))
}

#[derive(Deserialize)]
pub struct SuggestPortRequest {
    pub preferred: u16,
}

#[tauri::command(rename_all = "snake_case")]
pub fn suggest_free_port(request: Req<SuggestPortRequest>) -> IpcResult<serde_json::Value> {
    let (request_id, payload) = request.validate()?;
    let preferred = if payload.preferred == 0 {
        8964
    } else {
        payload.preferred
    };
    match find_free_port(preferred, 100) {
        Some(p) => Ok(respond(request_id, serde_json::json!({ "port": p }))),
        None => Err(from_string("no free port found in range".into())),
    }
}

#[derive(Deserialize)]
pub struct NormalizeUrlRequest {
    pub base_url: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn normalize_external_url(request: Req<NormalizeUrlRequest>) -> IpcResult<serde_json::Value> {
    let (request_id, payload) = request.validate()?;
    match normalize_base_url(&payload.base_url) {
        Ok(u) => Ok(respond(request_id, serde_json::json!({ "base_url": u }))),
        Err(e) => Err(from_string(e)),
    }
}
