use crate::core::external::{
    self, ExternalConfig, ExternalStatus, ProviderEntry, check_health, fetch_external_models,
    find_free_port, normalize_base_url, port_available,
};
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

// ─── Multi-provider IPC (mỗi provider = Account + Configuration) ─────────────
// Thiết kế theo mẫu CodeBuddy: mỗi provider có phần Account (trạng thái,
// models, login note — KHÔNG scheduler) + Configuration (endpoint, key,
// timeouts, ports, prefixes). Không quick actions ở đây.

fn validate_providers(providers: &[ProviderEntry], main_listen: &str) -> Result<(), String> {
    let mut names = HashSet::new();
    for p in providers {
        let name = p.name.trim();
        if name.is_empty() {
            return Err("provider name must not be empty".into());
        }
        if name.len() > 64 {
            return Err(format!("provider name too long: {name}"));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(format!("duplicate provider name: {name}"));
        }
        if p.kind.trim().is_empty() {
            return Err(format!("provider '{name}' kind must not be empty"));
        }
        // Validate từng entry enabled như single-bridge cũ (trùng port...).
        // Entry tắt thì chỉ check tên (cho phép lưu nháp).
        if p.config.enabled {
            p.config
                .validate(main_listen)
                .map_err(|e| format!("provider '{name}': {e}"))?;
        }
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_providers(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<ProviderEntry>> {
    let (request_id, _) = request.validate()?;
    let providers = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .providers
        .clone();
    Ok(respond(request_id, providers))
}

#[derive(Deserialize)]
pub struct SaveProvidersRequest {
    pub providers: Vec<ProviderEntry>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_providers(
    request: Req<SaveProvidersRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    if payload.providers.len() > 32 {
        return Err(from_string("too many providers (max 32)".into()));
    }
    let main_listen = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .listen
        .clone();
    validate_providers(&payload.providers, &main_listen).map_err(from_string)?;
    {
        let mut guard = state
            .config
            .lock()
            .map_err(|_| from_string("Config lock poisoned".into()))?;
        guard.providers = payload.providers;
        // Đồng bộ legacy = entry đầu để code cũ (validate/start/forward/health)
        // tiếp tục đúng mà không cần biết registry.
        crate::core::config::sync_legacy_external(&mut guard);
        crate::core::config::save_config(&guard).map_err(from_string)?;
    }
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct ProviderNameRequest {
    pub name: String,
}

fn find_provider(
    state: &State<'_, RuntimeState>,
    name: &str,
) -> Result<ProviderEntry, String> {
    let guard = state
        .config
        .lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    guard
        .providers
        .iter()
        .find(|p| p.name == name)
        .cloned()
        .ok_or_else(|| format!("provider not found: {name}"))
}

#[derive(Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub enabled: bool,
    pub reachable: bool,
    pub http_status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_provider_status(
    request: Req<ProviderNameRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<ProviderStatus> {
    let (request_id, payload) = request.validate()?;
    let entry = find_provider(&state, &payload.name).map_err(from_string)?;
    if !entry.config.enabled {
        return Ok(respond(
            request_id,
            ProviderStatus {
                name: entry.name,
                enabled: false,
                reachable: false,
                http_status: None,
                latency_ms: None,
                error: Some("provider disabled".into()),
            },
        ));
    }
    let s = check_health(&entry.config);
    Ok(respond(
        request_id,
        ProviderStatus {
            name: entry.name,
            enabled: true,
            reachable: s.reachable,
            http_status: s.http_status,
            latency_ms: s.latency_ms,
            error: s.error,
        },
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub fn refresh_provider_models(
    request: Req<ProviderNameRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Vec<serde_json::Value>> {
    let (request_id, payload) = request.validate()?;
    let entry = find_provider(&state, &payload.name).map_err(from_string)?;
    if !entry.config.enabled {
        return Err(from_string(format!("provider '{}' disabled", entry.name)));
    }
    let models = fetch_external_models(&entry.config).map_err(from_string)?;
    let values: Vec<serde_json::Value> = models
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "object": "model",
                "created": chrono::Utc::now().timestamp(),
                "owned_by": entry.name,
                "external": true,
            })
        })
        .collect();
    // Thay phần cache của provider này (xóa entry cũ cùng owned_by), giữ
    // provider khác. `/v1/models` merge ngay không cần restart gateway.
    if let Ok(mut cache) = state.external_models.lock() {
        cache.retain(|v| v.get("owned_by").and_then(|o| o.as_str()) != Some(entry.name.as_str()));
        cache.extend(values.clone());
    }
    Ok(respond(request_id, values))
}
