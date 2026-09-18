//! BLOCK Core — HTTP gateway thuần (axum), ported from Go: internal/server/*.go.
//!
//! File này KHÔNG chứa logic anti-api: mọi thứ provider ngoài nằm ở
//! `core/external.rs` (BLOCK anti-api). Điểm biên duy nhất:
//! `is_external_model` (chọn đường), `forward_external_stream` (forward chat),
//! `merge_external_models` (gộp models).
//!
//! Routes:
//!   POST /v1/chat/completions — SSE proxy (with account rotation)
//!   GET  /v1/models           — static + dynamic model list
//!   GET  /status              — per-account detail
//!   GET  /healthz             — healthy / total

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{get, post},
};
use chrono::{Datelike, FixedOffset, TimeZone, Utc};
use log::{error, info, warn};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio_stream::wrappers::ReceiverStream;

use crate::core::config::Config;
use crate::core::pool::Pool;
use crate::core::prompt::{DEGRADED, PromptMode};
use crate::core::protocol::{
    anthropic_content_block_delta, anthropic_content_block_start, anthropic_content_block_stop,
    anthropic_message_delta, anthropic_message_start, anthropic_message_stop,
    AnthropicMessagesRequest,
};
use crate::core::session::SessionRouter;
use crate::core::storage::Storage;
use crate::core::upstream::client::Client as UpstreamClient;
use crate::core::runtime::Metrics;
use parking_lot::RwLock as ParkingRwLock;
use crate::core::upstream::sse::aggregate;

// ─── Service Identity ───────────────────────────────────────────────────────

const SERVICE_NAME: &str = "universal-api";

// ─── Shared State ───────────────────────────────────────────────────────────

/// Shared application state, wrapped in `Arc` for axum.
pub struct AppState {
    pub config: Config,
    pub pool: Pool,
    pub session: SessionRouter,
    pub prompt_mode: PromptMode,
    /// Degrade gate: time-limited degraded mode until next midnight CST.
    pub degrade: DegradeGate,
    /// Shutdown signal sender.
    pub shutdown_tx: watch::Sender<bool>,
    pub metrics: Arc<Metrics>,
    /// Cache models động + TTL/fail-cooldown — port Go `dynamicModelsCache`
    /// (`internal/server/models.go`). Xem `DynamicModelsCache` bên dưới.
    pub dynamic_models: Arc<ParkingRwLock<DynamicModelsCache>>,
    /// Models từ external provider (anti-api). Merge vào `/v1/models`.
    pub external_models: Arc<ParkingRwLock<Vec<Value>>>,
    pub storage: Option<Arc<Storage>>,
    pub audit: Arc<crate::core::audit::AuditBuffer>,
}

pub type SharedState = Arc<AppState>;

// ─── Degrade Gate ───────────────────────────────────────────────────────────

/// Time-based degraded mode: when triggered, active until next 00:00 CST.
/// Mirrors Go `degradeGate` in internal/server/degrade.go.
pub struct DegradeGate {
    inner: Mutex<Option<chrono::DateTime<Utc>>>,
}

impl DegradeGate {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Check whether degraded mode is currently active (now < until).
    pub fn is_active(&self) -> bool {
        let g = self.inner.lock();
        match *g {
            Some(until) => Utc::now() < until,
            None => false,
        }
    }

    /// Trigger degraded mode until next 00:00 CST.
    /// If already active, does NOT extend (keeps the earliest trigger's deadline).
    pub fn trigger(&self) {
        let mut g = self.inner.lock();
        if g.map_or(true, |u| Utc::now() >= u) {
            *g = Some(next_midnight_cst(Utc::now()));
        }
    }
}

/// Next 00:00 in CST (UTC+8). Pure function for testability.
fn next_midnight_cst(now: chrono::DateTime<Utc>) -> chrono::DateTime<Utc> {
    let cst = FixedOffset::east_opt(8 * 3600).unwrap();
    let now_cst = now.with_timezone(&cst);
    let today_midnight = cst
        .with_ymd_and_hms(now_cst.year(), now_cst.month(), now_cst.day(), 0, 0, 0)
        .single()
        .unwrap();
    let mut target = today_midnight;
    while target <= now_cst {
        target = target + chrono::Duration::days(1);
    }
    target.with_timezone(&Utc)
}

// ─── Request / Response Types ───────────────────────────────────────────────

/// Incoming chat completion request (OpenAI-compatible subset).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_true")]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    /// Extra fields passed through transparently.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,
}

fn default_true() -> bool {
    true
}

/// OpenAI error envelope.
#[derive(Serialize)]
struct OaiError {
    error: OaiErrorBody,
}

#[derive(Serialize)]
struct OaiErrorBody {
    message: String,
    r#type: String,
    code: Option<String>,
}

fn oai_error(status: StatusCode, msg: &str, code: &str) -> Response {
    let body = serde_json::to_string(&OaiError {
        error: OaiErrorBody {
            message: msg.to_string(),
            r#type: "api_error".to_string(),
            code: Some(code.to_string()),
        },
    })
    .unwrap_or_default();
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

// ─── Prompt Application ─────────────────────────────────────────────────────

/// Apply the prompt mode to the message list.
///
/// - In degraded mode: replace/inject the `DEGRADED` system prompt.
/// - `PromptMode::Custom(text)`: replace existing system messages.
/// - `PromptMode::Passthrough`: no modification (unless degraded).
fn apply_prompt(mode: &PromptMode, messages: &mut Vec<ChatMessage>, degraded: bool) {
    if degraded {
        // In degraded mode, force the neutral prompt.
        messages.retain(|m| m.role != "system");
        messages.insert(
            0,
            ChatMessage {
                role: "system".to_string(),
                content: Value::String(DEGRADED.to_string()),
            },
        );
        return;
    }

    match mode {
        PromptMode::Passthrough => {}
        PromptMode::Custom(text) => {
            messages.retain(|m| m.role != "system" && m.role != "developer");
            messages.insert(
                0,
                ChatMessage {
                    role: "system".to_string(),
                    content: Value::String(text.clone()),
                },
            );
        }
    }
}

// ─── Server Builder ─────────────────────────────────────────────────────────

/// Build the axum router with all routes and middleware.
pub fn build_router(state: SharedState) -> Router {
    let api_routes = Router::new()
        .route("/v1/chat/completions", post(handle_chat))
        .route("/v1/messages", post(handle_messages))
        .route("/v1/models", get(handle_models))
        .route("/status", get(handle_status))
        .route("/metrics", get(handle_metrics))
        .route("/debug/traces", get(handle_debug_traces))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let public_routes = Router::new()
        .route("/healthz", get(handle_healthz));

    Router::new()
        .merge(api_routes)
        .merge(public_routes)
        .with_state(state)
}

/// Parse the listen address from config (e.g. ":7863" → "0.0.0.0:7863").
/// Chấp nhận `:7863`, `7863`, `127.0.0.1:7863`, `[::]:7863`.
/// Trả về `0.0.0.0:{port}` cho dạng rút gọn để giữ tương thích Go port.
fn parse_listen(listen: &str) -> String {
    let t = listen.trim();
    if t.is_empty() {
        return "0.0.0.0:7863".to_string();
    }
    if t.starts_with(':') {
        return format!("0.0.0.0{t}");
    }
    // Bare port `7863` (không có `:`) — lỗi ngầm hay gặp khi user nhập port thuần.
    if !t.contains(':') && t.parse::<u16>().is_ok() {
        return format!("0.0.0.0:{t}");
    }
    t.to_string()
}

/// Spawn the HTTP server on a background tokio task.
pub async fn start_server(
    state: SharedState,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
    // Validate external config sớm để báo lỗi rõ thay vì chạy nửa chừng.
    if let Err(e) = state.config.external.validate(&state.config.listen) {
        return Err(format!("invalid external config: {e}").into());
    }
    let addr = parse_listen(&state.config.listen);
    let router = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        let hint: Box<dyn std::error::Error + Send + Sync> = if e.kind() == std::io::ErrorKind::AddrInUse {
            format!(
                "port in use: {addr} ({e}). Change `listen` in Config (e.g. \":7864\") or free the port, then Start again."
            )
            .into()
        } else {
            format!("bind {addr} failed: {e}. Check `listen` format (\":7863\" or \"127.0.0.1:7863\").").into()
        };
        hint
    })?;
    info!(
        "server listening on {} (api_key={})",
        addr,
        !state.config.api_key.is_empty()
    );

    let mut shutdown_rx = state.shutdown_tx.subscribe();

    let handle = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.changed().await;
            })
            .await
            .unwrap_or_else(|e| error!("server error: {}", e));
    });

    Ok(handle)
}

// ─── Auth Middleware ────────────────────────────────────────────────────────

async fn auth_middleware(
    State(state): State<SharedState>,
    headers: HeaderMap,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let key = &state.config.api_key;
    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
        })
        .unwrap_or("");

    // 1. Direct config match (fast path)
    if !key.is_empty() && provided == key {
        request.extensions_mut().insert(ApiKeyContext {
            prefix: "config_master".to_string(),
        });
        return next.run(request).await;
    }

    // 2. Check local SQLite access keys (Feature 1 integration)
    if let Some(store) = &state.storage {
        if !provided.is_empty() {
            if let Ok(true) = store.check_access_key(provided) {
                let prefix = store
                    .access_key_prefix(provided)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "wbk-key".to_string());
                request.extensions_mut().insert(ApiKeyContext { prefix });
                return next.run(request).await;
            }
        }
    }

    // 3. Open access if no master key is configured
    if key.is_empty() {
        request.extensions_mut().insert(ApiKeyContext {
            prefix: "open_access".to_string(),
        });
        return next.run(request).await;
    }

    oai_error(
        StatusCode::UNAUTHORIZED,
        "missing or invalid API key",
        "invalid_api_key",
    )
}

#[derive(Clone)]
pub struct ApiKeyContext {
    pub prefix: String,
}

// ─── /v1/chat/completions ───────────────────────────────────────────────────

async fn handle_chat(
    State(state): State<SharedState>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Response {
    let key_prefix = request
        .extensions()
        .get::<ApiKeyContext>()
        .map(|c| c.prefix.clone())
        .unwrap_or_else(|| "anon".to_string());

    let body = match axum::body::to_bytes(request.into_body(), (state.config.server.max_body_mb as usize) * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return oai_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                &format!(
                    "Request body exceeds limit {} MB",
                    state.config.server.max_body_mb
                ),
                "request_body_too_large",
            );
        }
    };

    // ── Parse request ───────────────────────────────────────────────────
    let mut req: ChatRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return oai_error(
                StatusCode::BAD_REQUEST,
                &format!("Invalid JSON: {}", e),
                "invalid_request",
            );
        }
    };

    // ── Extract caller uid ──────────────────────────────────────────────
    let uid = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anon")
        .to_string();

    // ── Apply prompt mode (system prompt injection / degraded) ──────────
    let degraded = state.degrade.is_active();
    apply_prompt(&state.prompt_mode, &mut req.messages, degraded);

    let upstream_body = serde_json::to_vec(&req).unwrap_or_default();
    let raw_request_headers: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let raw_request_body = String::from_utf8_lossy(&body).to_string();
    let model = req.model.clone();
    let stream = req.stream;
    // ── Unified router: external model → anti-api bridge (lấy ý tưởng
    // Flow/Account routing từ anti-api, forward native qua HTTP) ──────────
    if state.config.external.enabled && crate::core::external::is_external_model(&model) {
        return crate::core::external::forward_external_stream(
            state,
            key_prefix,
            "/v1/chat/completions",
            model,
            upstream_body,
            raw_request_headers,
            raw_request_body,
            stream,
            false,
        );
    }
    dispatch_upstream_stream(
        state,
        uid,
        key_prefix,
        "/v1/chat/completions",
        model,
        upstream_body,
        raw_request_headers,
        raw_request_body,
        stream,
        false,
    )
}

/// Anthropic `/v1/messages` endpoint — accepts Anthropic format, converts to
/// OpenAI format for upstream forwarding, and translates SSE events back.
async fn handle_messages(
    State(state): State<SharedState>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Response {
    let key_prefix = request
        .extensions()
        .get::<ApiKeyContext>()
        .map(|c| c.prefix.clone())
        .unwrap_or_else(|| "anon".to_string());

    let body = match axum::body::to_bytes(request.into_body(), (state.config.server.max_body_mb as usize) * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return oai_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                &format!(
                    "Request body exceeds limit {} MB",
                    state.config.server.max_body_mb
                ),
                "request_body_too_large",
            );
        }
    };

    let req: AnthropicMessagesRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return oai_error(
                StatusCode::BAD_REQUEST,
                &format!("Invalid Anthropic JSON: {}", e),
                "invalid_request",
            );
        }
    };

    let uid = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anon")
        .to_string();

    let oai_req = req.to_openai();
    let upstream_body = serde_json::to_vec(&oai_req).unwrap_or_default();
    let raw_request_headers: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let raw_request_body = String::from_utf8_lossy(&body).to_string();
    let model = req.model.clone();
    let stream = req.stream;
    if state.config.external.enabled && crate::core::external::is_external_model(&model) {
        return crate::core::external::forward_external_stream(
            state,
            key_prefix,
            "/v1/messages",
            model,
            upstream_body,
            raw_request_headers,
            raw_request_body,
            stream,
            true,
        );
    }
    dispatch_upstream_stream(
        state,
        uid,
        key_prefix,
        "/v1/messages",
        model,
        upstream_body,
        raw_request_headers,
        raw_request_body,
        stream,
        true,
    )
}

/// Core upstream dispatch with 429 Retry-Fallback load balancer.
#[allow(clippy::too_many_arguments)]
fn dispatch_upstream_stream(
    state: SharedState,
    uid: String,
    key_prefix: String,
    route: &'static str,
    model: String,
    upstream_body: Vec<u8>,
    raw_request_headers: Vec<(String, String)>,
    raw_request_body: String,
    stream: bool,
    is_anthropic: bool,
) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(64);
    let upstream_config = state.config.upstream.clone();
    let pool = state.pool.clone();
    let session = state.session.clone();
    let metrics = state.metrics.clone();
    let audit = state.audit.clone();
    let storage = state.storage.clone();

    tokio::task::spawn_blocking(move || {
        let relay_start = Instant::now();
        let mut ttfb_recorded = false;
        let mut ttfb_ms: u64 = 0;
        let mut total_tokens: u64 = 0;
        let mut final_status: u16 = 200;
        let mut err_msg = String::new();
        let mut used_acct_email = String::new();
        let mut resp_preview = String::new();
        let mut final_forwarded_headers: Vec<(String, String)> = Vec::new();
        let mut final_response_headers: Vec<(String, String)> = Vec::new();
        let mut final_acct_uid = String::new();
        let mut final_proxy_used: Option<String> = None;
        // Preview body rộng hơn (học vpn_ai_proxy 4KB) để debug đủ ngữ cảnh.
        const PREVIEW_LIMIT: usize = 4096;

        // ─── 429 Retry-Fallback Loop (Max 3 attempts across pool) ───────────
        const MAX_ATTEMPTS: usize = 3;
        let mut attempts = 0;
        // Refresh inline 1 lần/request khi 401 giữa vòng đời (token bị thu hồi
        // trước hạn — pre-request refresh theo expires_at không bắt được case này).
        let mut refreshed_this_request = false;

        while attempts < MAX_ATTEMPTS {
            attempts += 1;

            let preferred = session.get(&uid);
            let account = match pool.pick(preferred.as_deref()) {
                Some(a) => a,
                None => {
                    final_status = 503;
                    err_msg = "all accounts unavailable".into();
                    break;
                }
            };

            let acct_uid = account.uid.clone();
            let acct_email = account.email.clone();
            used_acct_email = acct_email.clone();
            let mut acct_auth = account.auth.clone();

            if !pool.acquire(&acct_uid) {
                final_status = 503;
                err_msg = "account in-flight capacity exhausted".into();
                continue;
            }
            session.set(&uid, &acct_uid);

            // Egress/identity duy nhất từ config toàn cục (per-account routing
            // + fingerprint đã gỡ: fingerprint ổn định mới giống client thật).
            let mut upstream_client = UpstreamClient::new(&upstream_config.proxy_url);
            if !upstream_config.user_agent.is_empty() {
                upstream_client.user_agent = upstream_config.user_agent.clone();
            }
            if !upstream_config.realm.is_empty() {
                upstream_client.realm = upstream_config.realm.clone();
            }
            // Idle-timeout SSE (port Go idle.go): ngắt stream chết sau khoảng lặng.
            // 0 = tắt (mặc định).
            if upstream_config.idle_timeout_seconds > 0 {
                upstream_client.idle_timeout =
                    Duration::from_secs(upstream_config.idle_timeout_seconds as u64);
            }

            if acct_auth.needs_refresh(60) {
                match upstream_client.refresh_token(&mut acct_auth) {
                    Ok(()) => {
                        let _ = crate::core::auth::save_atomic(&acct_auth);
                        let _ = pool.update_auth(acct_auth.clone());
                    }
                    Err(error) => warn!("token refresh failed for {}: {}", acct_uid, error),
                }
            }

            match upstream_client.chat_stream(&acct_auth, &upstream_body) {
                Ok((body, _, forwarded_hdrs, resp_hdrs)) => {
                    final_forwarded_headers = forwarded_hdrs;
                    final_response_headers = resp_hdrs;
                    final_acct_uid = acct_uid.clone();
                    final_proxy_used = if !upstream_config.proxy_url.trim().is_empty() {
                        Some(upstream_config.proxy_url.clone())
                    } else {
                        None
                    };
                    if is_anthropic {
                        let msg_id = format!("msg_{}", relay_start.elapsed().as_nanos());
                        let _ = tx.blocking_send(Ok(Event::default().event("message_start").data(anthropic_message_start(&msg_id, &model))));
                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_start").data(anthropic_content_block_start(0))));
                    }

                    if !stream {
                        match aggregate(body) {
                            Ok(completion) => {
                                let payload = serde_json::to_string(&completion).unwrap_or_default();
                                if resp_preview.len() < PREVIEW_LIMIT {
                                    resp_preview.push_str(&payload.chars().take(PREVIEW_LIMIT - resp_preview.len()).collect::<String>());
                                }
                                let _ = tx.blocking_send(Ok(Event::default().data(payload)));
                                let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
                            }
                            Err(e) => {
                                final_status = 502;
                                err_msg = e;
                                let _ = tx.blocking_send(Ok(Event::default().data(
                                    "{\"error\":\"failed to aggregate upstream stream\"}",
                                )));
                            }
                        }
                    } else {
                        let reader = std::io::BufReader::new(body);
                        use std::io::BufRead;
                        for line_result in reader.lines() {
                            let line = match line_result {
                                Ok(l) => l,
                                Err(e) => {
                                    final_status = 502;
                                    err_msg = e.to_string();
                                    let _ = tx.blocking_send(Ok(Event::default().data(format!("{{\"error\":\"read: {}\"}}", e))));
                                    break;
                                }
                            };

                            if line.starts_with("data: ") {
                                let payload = &line[6..];
                                if !ttfb_recorded {
                                    ttfb_ms = relay_start.elapsed().as_millis() as u64;
                                    ttfb_recorded = true;
                                }
                                total_tokens += 1;

                                if resp_preview.len() < PREVIEW_LIMIT && payload != "[DONE]" {
                                    resp_preview.push_str(&payload.chars().take(PREVIEW_LIMIT - resp_preview.len()).collect::<String>());
                                    resp_preview.push('\n');
                                }

                                if payload == "[DONE]" {
                                    if is_anthropic {
                                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_stop").data(anthropic_content_block_stop(0))));
                                        let _ = tx.blocking_send(Ok(Event::default().event("message_delta").data(anthropic_message_delta(total_tokens))));
                                        let _ = tx.blocking_send(Ok(Event::default().event("message_stop").data(anthropic_message_stop())));
                                    } else {
                                        let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
                                    }
                                    break;
                                }

                                if is_anthropic {
                                    // Extract delta text from OpenAI chunk JSON if possible
                                    let chunk_text = serde_json::from_str::<Value>(payload)
                                        .ok()
                                        .and_then(|v| v.get("choices")?.get(0)?.get("delta")?.get("content")?.as_str().map(|s| s.to_string()))
                                        .unwrap_or_default();
                                    if !chunk_text.is_empty() {
                                        let delta_event = anthropic_content_block_delta(0, &chunk_text);
                                        if tx.blocking_send(Ok(Event::default().event("content_block_delta").data(delta_event))).is_err() {
                                            break;
                                        }
                                    }
                                } else if tx.blocking_send(Ok(Event::default().data(payload.to_string()))).is_err() {
                                    break;
                                }
                            }
                        }
                    }

                    pool.note_success(&acct_uid);
                    pool.release(&acct_uid);
                    break; // Success -> exit retry loop
                }
                Err((status, _, e)) => {
                    let s = if status == 0 { 502 } else { status };
                    final_status = s;
                    err_msg = e.to_string();

                    // Session chết (12153): Go disable vĩnh viễn + login lại bằng cơm.
                    // Trước đây Rust không ai gọi note_session_dead nên account chết
                    // retry vô hạn mà vẫn "healthy".
                    if e.kind == crate::core::upstream::errors::ErrKind::SessionDead {
                        if pool.note_session_dead(&acct_uid) {
                            warn!("session dead for account {}, disabled (re-login required)", acct_email);
                        }
                        pool.release(&acct_uid);
                        let _ = tx.blocking_send(Ok(Event::default().data(format!(
                            "{{\"error\":\"session dead, re-login required\"}}"
                        ))));
                        break;
                    }

                    if s == 429 {
                        // Rate limited -> cooldown account and retry with another
                        pool.cooldown(&acct_uid, Duration::from_secs(600));
                        warn!("429 rate limit for account {}, cooling down and retrying attempt {}/{}", acct_email, attempts, MAX_ATTEMPTS);
                        pool.release(&acct_uid);
                        continue;
                    } else if s == 401 && !refreshed_this_request && !acct_auth.refresh_token.trim().is_empty() {
                        // 401 giữa vòng đời: thử refresh 1 lần rồi retry ngay trong
                        // request này (consume 1 attempt). Refresh hỏng/token chết
                        // hẳn thì rơi xuống đếm auth_fails bên dưới.
                        refreshed_this_request = true;
                        match upstream_client.refresh_token(&mut acct_auth) {
                            Ok(()) => {
                                let _ = crate::core::auth::save_atomic(&acct_auth);
                                let _ = pool.update_auth(acct_auth.clone());
                                info!("inline refresh ok for {}, retrying request", acct_email);
                            }
                            Err(error) => {
                                warn!("inline refresh failed for {}: {}", acct_uid, error);
                            }
                        }
                        pool.release(&acct_uid);
                        continue;
                    } else {
                        if s >= 500 {
                            pool.note_error(&acct_uid);
                        }
                        if s == 401 && pool.note_auth_failure(&acct_uid) {
                            warn!("account {} disabled after repeated auth failures (re-login required)", acct_email);
                        }
                        pool.release(&acct_uid);
                        let _ = tx.blocking_send(Ok(Event::default().data(format!(
                            "{{\"error\":\"upstream returned {}\"}}",
                            s
                        ))));
                        break;
                    }
                }
            }
        }

        let elapsed = relay_start.elapsed();
        if !ttfb_recorded {
            ttfb_ms = elapsed.as_millis() as u64;
        }
        let tok_speed = if elapsed.as_millis() > 0 {
            (total_tokens as f64 / elapsed.as_millis() as f64) * 1000.0
        } else {
            0.0
        };
        info!(
            "[req] route={} key={} acct={} model={} status={} ttfb={}ms tokens={} speed={:.1}tok/s elapsed={}ms err={}",
            route, key_prefix, used_acct_email, model, final_status,
            ttfb_ms, total_tokens, tok_speed, elapsed.as_millis(), err_msg
        );

        metrics.trace_detail(
            route,
            &model,
            final_status,
            total_tokens,
            elapsed.as_millis() as u64,
            Some(json!({
                "req_bytes": upstream_body.len(),
                "req_preview": String::from_utf8_lossy(&upstream_body).chars().take(1000).collect::<String>(),
                "resp_preview": resp_preview.chars().take(2000).collect::<String>(),
                "error": if err_msg.is_empty() { Value::Null } else { Value::String(err_msg.clone()) },
            })),
        );
        if let Some(s) = &storage {
            let _ = s.log_usage(&key_prefix, route, &model, final_status, total_tokens, elapsed.as_millis() as u64);
        }

        let now_str = chrono::Utc::now().to_rfc3339();
        let log_id = format!("req_{}", relay_start.elapsed().as_nanos());
        // Body thực gửi upstream (sau prompt-injection). Trùng request thì để
        // rỗng cho UI khỏi hiện mục thừa — học vpn_ai_proxy raw_forwarded_body.
        let forwarded_body = {
            let sent = String::from_utf8_lossy(&upstream_body).to_string();
            if sent == raw_request_body { String::new() } else { sent.chars().take(4096).collect() }
        };
        audit.push(crate::core::audit::TrafficAuditLog {
            id: log_id,
            timestamp: now_str,
            route: route.to_string(),
            model: model.clone(),
            status_code: final_status,
            duration_ms: elapsed.as_millis() as u64,
            account_uid: final_acct_uid,
            proxy_used: final_proxy_used,
            raw_request_headers,
            raw_forwarded_headers: final_forwarded_headers,
            raw_request_body,
            raw_response_preview: resp_preview,
            raw_response_headers: final_response_headers,
            raw_forwarded_body: forwarded_body,
            is_streaming: stream,
        });
    });

    let stream_resp = ReceiverStream::new(rx);
    Sse::new(stream_resp)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text(":"),
        )
        .into_response()
}

// ─── /v1/models ─────────────────────────────────────────────────────────────

/// Static model IDs (CN models from Go static table).
const STATIC_MODELS: &[&str] = &[
    "glm-5.2",
    "glm-5.1",
    "glm-5v-turbo",
    "kimi-k2.7",
    "minimax-m3",
    "hy3",
    "hy3-preview",
    "hy3-preview-agent",
    "deepseek-v4-pro",
    "deepseek-v4-flash",
];

// ─── /v1/models ─────────────────────────────────────────────────────────────

/// Cache model động — port Go `dynamicModelsCache` (`internal/server/models.go`).
///
/// - `models` + `fetched_at`: serve thẳng khi còn tươi (TTL 1h).
/// - `last_fail`: negative cache 5 phút — upstream đang lỗi thì dùng bảng tĩnh,
///   không đánh liên tục vào upstream mỗi lần gọi `/v1/models`.
#[derive(Debug, Default)]
pub struct DynamicModelsCache {
    pub models: Vec<Value>,
    pub fetched_at: Option<Instant>,
    pub last_fail: Option<Instant>,
}

/// TTL cache thành công (Go `dynamicModelsTTL`).
pub const DYNAMIC_MODELS_TTL: Duration = Duration::from_secs(3600);
/// Negative cache khi fetch lỗi (Go `modelsFetchFailCooldown`).
pub const MODELS_FETCH_FAIL_COOLDOWN: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelsDecision {
    /// Cache tươi — serve luôn.
    ServeCache,
    /// Hết TTL (hoặc chưa từng fetch) và không trong cooldown — fetch mới.
    TryFetch,
    /// Fetch vừa lỗi — serve bảng tĩnh, khỏi đánh upstream.
    ServeStatic,
}

/// Quyết định thuần (không I/O) để test được — mirror đúng thứ tự Go:
/// cache non-empty + tươi trước, negative cache sau.
fn decide_models(has_models: bool, fetched_at: Option<Instant>, last_fail: Option<Instant>, now: Instant) -> ModelsDecision {
    if has_models {
        if let Some(f) = fetched_at {
            if now.duration_since(f) < DYNAMIC_MODELS_TTL {
                return ModelsDecision::ServeCache;
            }
        }
    }
    if let Some(f) = last_fail {
        if now.duration_since(f) < MODELS_FETCH_FAIL_COOLDOWN {
            return ModelsDecision::ServeStatic;
        }
    }
    ModelsDecision::TryFetch
}

/// Kéo model động từ MỘT account trong pool (Go: `Pool.Pick`, fail thì
/// `NoteError` để lần sau pick trúng account khác). Blocking (ureq) — caller
/// phải gọi trong `spawn_blocking`. Trả về rỗng khi không có account/lỗi.
pub fn fetch_dynamic_model_values(pool: &Pool, upstream: &crate::core::config::UpstreamConfig) -> Vec<Value> {
    fetch_dynamic_models_verbose(pool, upstream).unwrap_or_default()
}

/// Bản verbose cho nút refresh tay: Err mang root cause (endpoint nào, mã gì)
/// để UI hiện thay vì "failed" chung chung.
pub fn fetch_dynamic_models_verbose(
    pool: &Pool,
    upstream: &crate::core::config::UpstreamConfig,
) -> Result<Vec<Value>, String> {
    let account = match pool.pick(None) {
        Some(a) => a,
        None => return Err("no healthy account in pool".into()),
    };
    let mut client = UpstreamClient::new(&upstream.proxy_url);
    if !upstream.user_agent.is_empty() {
        client.user_agent = upstream.user_agent.clone();
    }
    if !upstream.realm.is_empty() {
        client.realm = upstream.realm.clone();
    }
    match client.fetch_models(&account.auth) {
        Ok(models) if !models.is_empty() => Ok(models
            .into_iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "object": "model",
                    "created": 1753600000_i64,
                    "owned_by": "universal-api",
                    // Chặn đáy như Go (ContextWindow == 0 → 131072).
                    "context_length": if m.context_window == 0 { 131072 } else { m.context_window },
                    "max_output_tokens": m.max_tokens,
                })
            })
            .collect()),
        Ok(_) => {
            pool.note_error(&account.uid);
            Err(format!("upstream returned empty model list (uid={})", account.uid))
        }
        Err(e) => {
            pool.note_error(&account.uid);
            Err(format!("uid={} domain={:?}: {e}", account.uid, account.auth.domain))
        }
    }
}

/// Refresh cache nếu hết TTL (chạy fetch blocking trong `spawn_blocking` để
/// không chặn tokio worker). Lỗi → ghi `last_fail` (negative cache).
async fn refresh_dynamic_models_if_stale(state: &SharedState) {
    let decision = {
        let cache = state.dynamic_models.read();
        decide_models(!cache.models.is_empty(), cache.fetched_at, cache.last_fail, Instant::now())
    };
    if decision != ModelsDecision::TryFetch {
        return;
    }
    let pool = state.pool.clone();
    let upstream = state.config.upstream.clone();
    let values = tokio::task::spawn_blocking(move || fetch_dynamic_model_values(&pool, &upstream))
        .await
        .unwrap_or_default();
    let mut cache = state.dynamic_models.write();
    if values.is_empty() {
        cache.last_fail = Some(Instant::now());
    } else {
        cache.models = values;
        cache.fetched_at = Some(Instant::now());
        cache.last_fail = None; // Thành công thì dọn negative cache (như Go).
    }
}

async fn handle_models(State(state): State<SharedState>) -> Response {
    refresh_dynamic_models_if_stale(&state).await;
    let (dynamic, external) = {
        let cache = state.dynamic_models.read();
        (cache.models.clone(), state.external_models.read().clone())
    };
    // Ưu tiên: dynamic (discovery nội bộ) → static; external luôn append thêm
    // (dedupe theo id để tránh trùng khi cùng tên model).
    let mut data: Vec<Value> = if dynamic.is_empty() { STATIC_MODELS
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "created": 1753600000_i64,
                "owned_by": "universal-api",
                "context_length": 131072,
            })
        })
        .collect() } else { dynamic };
    if state.config.external.enabled {
        // Biên anti-api: gộp models ngoài (logic trong core/external.rs).
        crate::core::external::merge_external_models(&mut data, external);
    }

    let resp = json!({
        "object": "list",
        "data": data
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&resp).unwrap_or_default(),
    )
        .into_response()
}

// ─── /healthz ───────────────────────────────────────────────────────────────

async fn handle_healthz(State(state): State<SharedState>) -> Response {
    let healthy = state.pool.healthy_count();
    let total = state.pool.total_count();

    let status = if healthy > 0 {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = json!({
        "healthy": healthy,
        "total": total,
        "service": SERVICE_NAME,
    });

    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::HeaderName::from_static("x-service"), SERVICE_NAME),
        ],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

// ─── /status ────────────────────────────────────────────────────────────────

async fn handle_status(State(state): State<SharedState>) -> Response {
    let accounts = state.pool.status_json();
    let healthy = state.pool.healthy_count();
    let total = state.pool.total_count();
    let sticky = state.session.count();

    let body = json!({
        "accounts": accounts,
        "total": total,
        "healthy": healthy,
        "sticky_sessions": sticky,
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response()
}

async fn handle_metrics(State(state): State<SharedState>) -> Response {
    let (requests_total, requests_failed, tokens_total) = state.metrics.snapshot();
    let body = json!({
        "requests_total": requests_total,
        "requests_failed": requests_failed,
        "tokens_total": tokens_total,
        "accounts_total": state.pool.total_count(),
        "accounts_healthy": state.pool.healthy_count(),
    });
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], body.to_string()).into_response()
}

async fn handle_debug_traces(State(state): State<SharedState>) -> Response {
    let traces = state.metrics.recent_traces().into_iter()
        .filter_map(|trace| serde_json::from_str::<Value>(&trace).ok())
        .collect::<Vec<_>>();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], json!({ "traces": traces }).to_string()).into_response()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_midnight_cst_wraps() {
        // 2026-01-15 23:30 CST = 2026-01-15 15:30 UTC
        let now = Utc.with_ymd_and_hms(2026, 1, 15, 15, 30, 0).unwrap();
        let mid = next_midnight_cst(now);
        // Should be 2026-01-16 00:00 CST = 2026-01-15 16:00 UTC
        assert_eq!(mid, Utc.with_ymd_and_hms(2026, 1, 15, 16, 0, 0).unwrap());
    }

    #[test]
    fn degrade_gate_trigger_and_check() {
        let gate = DegradeGate::new();
        assert!(!gate.is_active());
        gate.trigger();
        assert!(gate.is_active());
    }

    #[test]
    fn oai_error_format() {
        let resp = oai_error(StatusCode::BAD_REQUEST, "test", "invalid_request");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn apply_prompt_custom() {
        let mode = PromptMode::Custom("You are a helpful assistant.".into());
        let mut msgs = vec![ChatMessage {
            role: "user".into(),
            content: Value::String("hello".into()),
        }];
        apply_prompt(&mode, &mut msgs, false);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
    }

    #[test]
    fn apply_prompt_degraded_replaces() {
        let mode = PromptMode::Passthrough;
        let mut msgs = vec![
            ChatMessage {
                role: "system".into(),
                content: Value::String("original".into()),
            },
            ChatMessage {
                role: "user".into(),
                content: Value::String("hello".into()),
            },
        ];
        apply_prompt(&mode, &mut msgs, true);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[0].content, Value::String(DEGRADED.to_string()));
    }

    #[test]
    fn apply_prompt_passthrough_noop() {
        let mode = PromptMode::Passthrough;
        let mut msgs = vec![
            ChatMessage {
                role: "system".into(),
                content: Value::String("keep me".into()),
            },
            ChatMessage {
                role: "user".into(),
                content: Value::String("hello".into()),
            },
        ];
        apply_prompt(&mode, &mut msgs, false);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, Value::String("keep me".into()));
    }

    #[test]
    fn parse_listen_colon_prefix() {
        assert_eq!(parse_listen(":7863"), "0.0.0.0:7863");
        assert_eq!(parse_listen("127.0.0.1:8080"), "127.0.0.1:8080");
        // Bare port + empty — chống lỗi ngầm user nhập port thuần.
        assert_eq!(parse_listen("7863"), "0.0.0.0:7863");
        assert_eq!(parse_listen(""), "0.0.0.0:7863");
        assert_eq!(parse_listen("  :7863  "), "0.0.0.0:7863");
    }

    #[test]
    fn models_cache_ttl_decisions() {
        let now = Instant::now();
        let fresh = now - Duration::from_secs(100);
        let stale = now - Duration::from_secs(4000);
        let fail_recent = now - Duration::from_secs(60);
        let fail_old = now - Duration::from_secs(600);
        // Cache tươi → serve.
        assert_eq!(decide_models(true, Some(fresh), None, now), ModelsDecision::ServeCache);
        // Hết TTL → fetch.
        assert_eq!(decide_models(true, Some(stale), None, now), ModelsDecision::TryFetch);
        // Chưa từng fetch, không fail → fetch.
        assert_eq!(decide_models(false, None, None, now), ModelsDecision::TryFetch);
        // Fail mới → static (kể cả cache cũ còn models).
        assert_eq!(decide_models(true, Some(stale), Some(fail_recent), now), ModelsDecision::ServeStatic);
        assert_eq!(decide_models(false, None, Some(fail_recent), now), ModelsDecision::ServeStatic);
        // Fail cũ → fetch lại.
        assert_eq!(decide_models(false, None, Some(fail_old), now), ModelsDecision::TryFetch);
    }
}
