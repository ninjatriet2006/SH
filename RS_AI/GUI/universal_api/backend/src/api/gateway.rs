use crate::core::config::Config;
use crate::core::prompt::{load_prompt, PromptMode};
use crate::core::redisstore::NoopStore;
use crate::core::runtime::RuntimeState;
use crate::core::server::{start_server, AppState};
use crate::core::session;
use crate::ipc::{respond, Empty, IpcError, IpcErrorCode, IpcResult, Req};
use crate::core::gateway_state::{GatewayInfo, GatewayState};
use serde::{Deserialize, Serialize};
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
    // BUGFIX rehydrate: sau restart app, Pool::new chỉ dựng placeholder (token
    // rỗng) từ state.json — phải nạp token thật từ vault, không thì mọi request
    // 401 với token rỗng.
    if let Ok(store) = runtime.storage() {
        if let Ok(saved) = store.list_accounts() {
            for a in saved {
                let file_path = std::path::Path::new(&config.auth_dir)
                    .join(format!("workbuddy-{}.json", crate::core::login::sanitize_uid(&a.uid)))
                    .to_string_lossy()
                    .to_string();
                runtime.pool.add(crate::core::auth::Auth {
                    access_token: a.access_token,
                    refresh_token: a.refresh_token,
                    expires_at: a.expires_at,
                    domain: a.domain,
                    uid: a.uid,
                    enterprise_id: a.enterprise_id,
                    nickname: a.nickname,
                    file_path,
                });
            }
        }
    }
    // BUGFIX configure: Pool::configure trước đây không ai gọi nên setting pool
    // trong Config (max_in_flight, breaker...) không bao giờ có hiệu lực.
    {
        let pc = &config.pool;
        runtime.pool.configure(
            pc.max_in_flight as i64,
            pc.breaker_threshold,
            crate::core::config::parse_duration(&pc.breaker_cooldown).unwrap_or(Duration::from_secs(1800)),
            crate::core::config::parse_duration(&pc.breaker_cooldown_max).unwrap_or(Duration::from_secs(21600)),
            crate::core::config::parse_duration(&config.cooldown.soft_rate_max).unwrap_or(Duration::from_secs(7200)),
            pc.idle_weight_per_hour,
            pc.idle_weight_max,
        );
    }
    let (shutdown_tx, _) = tokio::sync::watch::channel(false);
    let pool = runtime.pool.clone();
    let available_pool = pool.clone();
    let prompt_mode = match load_prompt(&config.prompt.mode, &config.prompt.file) {
        Ok(text) if !text.is_empty() => PromptMode::Custom(text),
        Ok(_) => PromptMode::Passthrough,
        Err(error) => { info.status = crate::core::gateway_state::GatewayStatus::Error; info.error = Some(error.clone()); return Err(IpcError::new(IpcErrorCode::InvalidArgument, error)); }
    };
    let session_cfg = session::Config { ttl: Duration::from_secs(1800), gc_interval: Duration::from_secs(300), store: Arc::new(NoopStore), available: Arc::new(move || available_pool.status_json().into_iter().filter(|a| a.healthy).map(|a| a.uid).collect()) };
    let dynamic_models = Arc::new(RwLock::new(crate::core::server::DynamicModelsCache::default()));
    // Discovery lúc start (dùng chung helper với refresh định kỳ trong handler).
    // Rỗng → ghi last_fail để lần gọi /v1/models đầu tiên tôn trọng fail-cooldown.
    {
        let initial = crate::core::server::fetch_dynamic_model_values(&pool, &config.upstream);
        let mut cache = dynamic_models.write();
        if initial.is_empty() {
            cache.last_fail = Some(std::time::Instant::now());
        } else {
            cache.models = initial;
            cache.fetched_at = Some(std::time::Instant::now());
        }
    }
    // Giữ handle để IPC refresh tay + lần start sau không lẫn cache cũ.
    if let Ok(mut h) = gateway.dynamic_models.lock() {
        *h = Some(dynamic_models.clone());
    }
    let storage = runtime.storage().ok();
    let audit = runtime.audit.clone();
    let external_snapshot = runtime.external_models.lock().map(|c| c.clone()).unwrap_or_default();
    // Giữ session router ở RuntimeState để UI hỏi "account nào đang phục vụ".
    let session_router = session::SessionRouter::new(session_cfg);
    if let Ok(mut s) = runtime.session.lock() {
        *s = Some(session_router.clone());
    }
    let app_state = Arc::new(AppState { config: config.clone(), pool, session: session_router, prompt_mode, degrade: crate::core::server::DegradeGate::new(), shutdown_tx: shutdown_tx.clone(), metrics: gateway.metrics.clone(), dynamic_models, external_models: Arc::new(RwLock::new(external_snapshot)), storage, audit });
    let task = tauri::async_runtime::block_on(start_server(app_state)).map_err(|e| { info.status = crate::core::gateway_state::GatewayStatus::Error; info.error = Some(e.to_string()); IpcError::new(IpcErrorCode::Unavailable, e.to_string()) })?;
    *gateway.server_task.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))? = Some(task);
    // Scheduler nền: trước đây code có Scheduler nhưng không nơi nào spawn nên lịch
    // trong config là chết — chỉ còn nút run-now. Spawn ở đây để lịch có hiệu lực.
    // BỌC block_on: sync command chạy ngoài tokio runtime, tokio::spawn trần ở đây
    // panic "no reactor running" → command treo (đã từng xảy ra).
    {
        let mut sched = gateway.scheduler_task.lock().map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))?;
        if let Some(old) = sched.take() { old.abort(); }
        let scheduler = crate::core::scheduler::Scheduler::new(
            config.schedule.clone(),
            runtime.pool.clone(),
            crate::core::upstream::client::Client::new(&config.upstream.proxy_url),
            shutdown_tx.subscribe(),
        );
        *sched = Some(tauri::async_runtime::block_on(async move { scheduler.spawn() }));
    }
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
    runtime: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, _) = request.validate()?;
    if let Ok(mut task) = gateway.server_task.lock() {
        if let Some(handle) = task.take() { handle.abort(); }
    }
    if let Ok(mut sched) = gateway.scheduler_task.lock() {
        if let Some(handle) = sched.take() { handle.abort(); }
    }
    // Xóa session để UI không hiện serving account đã cũ.
    if let Ok(mut s) = runtime.session.lock() {
        *s = None;
    }
    gateway.shutdown_tx.send(true).ok();
    if let Ok(mut info) = gateway.info.lock() { info.status = crate::core::gateway_state::GatewayStatus::Stopped; }
    Ok(respond(request_id, Empty {}))
}

/// Refresh tay danh sách model động: discovery ngay (không chờ TTL), cập nhật
/// cache gateway đang chạy, trả về list. Fail → Err kèm gợi ý check account.
#[tauri::command(rename_all = "snake_case")]
pub fn refresh_gateway_models(
    request: Req<Empty>,
    gateway: State<'_, GatewayState>,
    runtime: State<'_, RuntimeState>,
) -> IpcResult<Vec<serde_json::Value>> {
    let (request_id, _) = request.validate()?;
    let handle = gateway
        .dynamic_models
        .lock()
        .map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))?
        .clone()
        .ok_or_else(|| IpcError::new(IpcErrorCode::Unavailable, "gateway not running (start it first)"))?;
    let (pool, upstream) = {
        let cfg = runtime
            .config
            .lock()
            .map_err(|_| IpcError::new(IpcErrorCode::Internal, "State lock poisoned"))?;
        (runtime.pool.clone(), cfg.upstream.clone())
    };
    // Sync command chạy trên threadpool blocking — gọi trực tiếp, khỏi spawn.
    // Bắt lỗi chi tiết để UI hiện root cause (404 endpoint? 401 token? parse?),
    // thay vì message chung chung khiến user đoán mò.
    let values = match crate::core::server::fetch_dynamic_models_verbose(&pool, &upstream) {
        Ok(v) => v,
        Err(detail) => {
            {
                let mut cache = handle.write();
                cache.last_fail = Some(std::time::Instant::now());
            }
            log::warn!("refresh_gateway_models failed: {detail}");
            return Err(IpcError::new(
                IpcErrorCode::Unavailable,
                format!("model discovery failed: {detail}"),
            ));
        }
    };
    if values.is_empty() {
        {
            let mut cache = handle.write();
            cache.last_fail = Some(std::time::Instant::now());
        }
        return Err(IpcError::new(
            IpcErrorCode::Unavailable,
            "model discovery returned empty list — check Accounts > Billing Check",
        ));
    }
    {
        let mut cache = handle.write();
        cache.models = values.clone();
        cache.fetched_at = Some(std::time::Instant::now());
        cache.last_fail = None;
    }
    Ok(respond(request_id, values))
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

#[derive(Serialize)]
pub struct ServingInfo {
    /// UID đang ghim sticky cho client mặc định (không gửi x-user-id).
    /// Đây là account mà request kế tiếp "không tag" sẽ tiêu tốn.
    pub serving_uid: Option<String>,
    pub sticky_sessions: usize,
    pub gateway_running: bool,
}

/// Account nào đang phục vụ — để UI hiện thay vì user phải đoán.
#[tauri::command(rename_all = "snake_case")]
pub fn get_serving_account(
    request: Req<Empty>,
    gateway: State<'_, GatewayState>,
    runtime: State<'_, RuntimeState>,
) -> IpcResult<ServingInfo> {
    let (request_id, _) = request.validate()?;
    let running = gateway
        .server_task
        .lock()
        .map(|t| t.is_some())
        .unwrap_or(false);
    let (serving_uid, sticky_sessions) = runtime
        .session
        .lock()
        .map(|s| match s.as_ref() {
            Some(router) => (router.get("anon"), router.count()),
            None => (None, 0),
        })
        .unwrap_or((None, 0));
    Ok(respond(request_id, ServingInfo { serving_uid, sticky_sessions, gateway_running: running }))
}
