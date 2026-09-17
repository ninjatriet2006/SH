use crate::core::config::Config;
use crate::core::prompt::{load_prompt, PromptMode};
use crate::core::redisstore::NoopStore;
use crate::core::runtime::RuntimeState;
use crate::core::server::{start_server, AppState};
use crate::core::session;
use crate::ipc::{respond, Empty, IpcError, IpcErrorCode, IpcResult, Req};
use crate::core::gateway_state::{GatewayInfo, GatewayState};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;
use parking_lot::RwLock;

#[tauri::command(rename_all = "snake_case")]
pub fn get_gateway_status(
    request: Req<Empty>,
    state: State<'_, GatewayState>,
) -> IpcResult<GatewayInfo> {
    let (request_id, _) = request.validate()?;
    let info = state.info.lock().map_err(|_| {
        IpcError::new(IpcErrorCode::Internal, "State lock poisoned")
    })?;
    let (requests_total, requests_failed, tokens_total) = state
        .metrics
        .snapshot();
    Ok(respond(request_id, GatewayInfo {
        status: info.status,
        listen: info.listen.clone(),
        total_accounts: info.total_accounts,
        healthy_accounts: info.healthy_accounts,
        error: info.error.clone(),
        requests_total,
        requests_failed,
        tokens_total,
    }))
}

#[derive(Deserialize)]
pub struct StartGatewayRequest {
    pub config_path: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn start_gateway(
    request: Req<StartGatewayRequest>,
    gateway: State<'_, GatewayState>,
    runtime: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let mut info = gateway.info.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))?;
    if gateway.server_task.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))?.is_some() {
        return Ok(respond(request_id, Empty {}));
    }
    info.status = crate::core::gateway_state::GatewayStatus::Starting;
    let mut config = runtime.config.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))?.clone();
    if let Some(path) = payload.config_path.filter(|p| !p.is_empty()) {
        config = load_config_path(&path).map_err(|e| IpcError::new(IpcErrorCode::InvalidArgument, e))?;
        *runtime.config.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))? = config.clone();
    }
    gateway.metrics.configure_path(&config.state_file);
    let (shutdown_tx, _) = tokio::sync::watch::channel(false);
    let pool = runtime.pool.clone();
    let available_pool = pool.clone();
    let prompt_mode = match load_prompt(&config.prompt.mode, &config.prompt.file) {
        Ok(text) if !text.is_empty() => PromptMode::Custom(text),
        Ok(_) => PromptMode::Passthrough,
        Err(error) => { info.status = crate::core::gateway_state::GatewayStatus::Error; info.error = Some(error.clone()); return Err(IpcError::new(IpcErrorCode::InvalidArgument, error)); }
    };
    let session_cfg = session::Config { ttl: Duration::from_secs(1800), gc_interval: Duration::from_secs(300), store: Arc::new(NoopStore), available: Arc::new(move || available_pool.status_json().into_iter().filter(|a| a.healthy).map(|a| a.uid).collect()) };
    let dynamic_models = Arc::new(RwLock::new(Vec::new()));
    let model_client = crate::core::upstream::client::Client::new(&config.upstream.proxy_url);
    let discovery_accounts = runtime.pool.all_accounts();
    for account in &discovery_accounts {
        if let Ok(models) = model_client.fetch_models(&account.auth) {
            let values = models.into_iter().map(|model| serde_json::json!({
                "id": model.id,
                "object": "model",
                "created": chrono::Utc::now().timestamp(),
                "owned_by": "workbuddy",
                "context_length": model.context_window,
                "max_tokens": model.max_tokens,
            })).collect();
            *dynamic_models.write() = values;
            break;
        }
    }
    let storage = runtime.storage().ok();
    let audit = runtime.audit.clone();
    let app_state = Arc::new(AppState { config: config.clone(), pool, session: session::SessionRouter::new(session_cfg), prompt_mode, degrade: crate::core::server::DegradeGate::new(), shutdown_tx: shutdown_tx.clone(), metrics: gateway.metrics.clone(), dynamic_models, storage, audit });
    let task = tauri::async_runtime::block_on(start_server(app_state)).map_err(|e| { info.status = crate::core::gateway_state::GatewayStatus::Error; info.error = Some(e.to_string()); IpcError::new(IpcErrorCode::Unavailable, e.to_string()) })?;
    *gateway.server_task.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))? = Some(task);
    gateway.shutdown_tx.send(true).ok();
    info.status = crate::core::gateway_state::GatewayStatus::Running;
    info.listen = Some(config.listen);
    info.total_accounts = runtime.pool.total_count();
    info.healthy_accounts = runtime.pool.healthy_count();
    info.error = None;
    Ok(respond(request_id, Empty {}))
}

#[tauri::command(rename_all = "snake_case")]
pub fn stop_gateway(
    request: Req<Empty>,
    gateway: State<'_, GatewayState>,
) -> IpcResult<Empty> {
    let (request_id, _) = request.validate()?;
    if let Ok(mut task) = gateway.server_task.lock() {
        if let Some(handle) = task.take() { handle.abort(); }
    }
    gateway.shutdown_tx.send(true).ok();
    if let Ok(mut info) = gateway.info.lock() { info.status = crate::core::gateway_state::GatewayStatus::Stopped; }
    Ok(respond(request_id, Empty {}))
}

fn load_config_path(path: &str) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("Cannot read config: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid config: {e}"))
}

/// Recent raw request/response traces for the Debug tab.
#[tauri::command(rename_all = "snake_case")]
pub fn get_debug_traces(
    request: Req<Empty>,
    gateway: State<'_, GatewayState>,
) -> IpcResult<Vec<serde_json::Value>> {
    let (request_id, _) = request.validate()?;
    let traces = gateway
        .metrics
        .recent_traces()
        .into_iter()
        .filter_map(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .collect();
    Ok(respond(request_id, traces))
}
