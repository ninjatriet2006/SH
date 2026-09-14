//! OpenAI-compatible HTTP gateway (axum).
//!
//! Ported from Go: internal/server/*.go
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
use chrono::{Datelike, FixedOffset, Local, TimeZone, Utc};
use log::{error, info, warn};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::watch;
use tokio_stream::wrappers::ReceiverStream;

use crate::core::auth::Auth;
use crate::core::config::Config;
use crate::core::pool::{self, Pool};
use crate::core::prompt::{self, DEGRADED, PromptMode};
use crate::core::session::SessionRouter;
use crate::core::upstream::client::Client as UpstreamClient;
use crate::core::runtime::Metrics;
use parking_lot::RwLock as ParkingRwLock;
use crate::core::upstream::sse::aggregate;

// ─── Service Identity ───────────────────────────────────────────────────────

const SERVICE_NAME: &str = "workbuddy2api";

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
    pub dynamic_models: Arc<ParkingRwLock<Vec<Value>>>,
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
fn parse_listen(listen: &str) -> String {
    if listen.starts_with(':') {
        format!("0.0.0.0{}", listen)
    } else {
        listen.to_string()
    }
}

/// Spawn the HTTP server on a background tokio task.
pub async fn start_server(
    state: SharedState,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
    let addr = parse_listen(&state.config.listen);
    let router = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
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
    request: Request<Body>,
    next: Next,
) -> Response {
    let key = &state.config.api_key;
    if key.is_empty() {
        // No key configured → open access.
        return next.run(request).await;
    }

    let provided = headers.get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")))
        .unwrap_or("");

    if provided == key {
        next.run(request).await
    } else {
        oai_error(
            StatusCode::UNAUTHORIZED,
            "missing or invalid API key",
            "invalid_api_key",
        )
    }
}

// ─── /v1/chat/completions ───────────────────────────────────────────────────

async fn handle_chat(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let start = Instant::now();
    let metrics = state.metrics.clone();

    // ── Body size check (413 if exceeded, no rotation) ──────────────────
    let max_bytes = (state.config.server.max_body_mb as usize) * 1024 * 1024;
    if max_bytes > 0 && body.len() > max_bytes {
        return oai_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!(
                "Request body exceeds limit {} MB",
                state.config.server.max_body_mb
            ),
            "request_body_too_large",
        );
    }

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

    // ── Pick account (sticky session) ───────────────────────────────────
    let preferred = state.session.get(&uid);
    let account = match state.pool.pick(preferred.as_deref()) {
        Some(a) => a,
        None => {
            return oai_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "all accounts unavailable (cooling/disabled)",
                "no_healthy_account",
            );
        }
    };
    let acct_uid = account.uid.clone();
    let acct_email = account.email.clone();
    let acct_auth = account.auth.clone();
    let upstream_config = state.config.upstream.clone();
    if !state.pool.acquire(&acct_uid) {
        return oai_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "all accounts are at the in-flight limit",
            "account_capacity_exhausted",
        );
    }
    state.session.set(&uid, &acct_uid);

    // ── Build upstream request body ─────────────────────────────────────
    let upstream_body = serde_json::to_vec(&req).unwrap_or_default();
    let model = req.model.clone();
    let stream = req.stream;

    // ── Forward to upstream (SSE streaming) ─────────────────────────────
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(64);

    let pool = state.pool.clone();

    // Build a per-request upstream client from the picked account's auth.
    let mut upstream_client = UpstreamClient::new(&upstream_config.proxy_url);
    upstream_client.user_agent = upstream_config.user_agent.clone();
    if !upstream_config.realm.is_empty() { upstream_client.realm = upstream_config.realm.clone(); }

    tokio::task::spawn_blocking(move || {
        let relay_start = Instant::now();
        let mut ttfb_recorded = false;
        let mut ttfb_ms: u64 = 0;
        let mut total_tokens: u64 = 0;
        let mut final_status: u16 = 200;
        let mut err_msg = String::new();

        let mut acct_auth = acct_auth;
        if acct_auth.needs_refresh(60) {
            match upstream_client.refresh_token(&mut acct_auth) {
                Ok(()) => { let _ = crate::core::auth::save_atomic(&acct_auth); let _ = pool.update_auth(acct_auth.clone()); }
                Err(error) => warn!("token refresh failed for {}: {}", acct_uid, error),
            }
        }
        match upstream_client.chat_stream(&acct_auth, &upstream_body) {
            Ok((body, _)) => {
                if !stream {
                    match aggregate(body) {
                        Ok(completion) => {
                            let payload = serde_json::to_string(&completion).unwrap_or_default();
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
                    pool.note_success(&acct_uid);
                } else {
                    // Read the SSE stream line-by-line.
                    let reader = std::io::BufReader::new(body);
                    use std::io::BufRead;
                    for line_result in reader.lines() {
                    let line = match line_result {
                        Ok(l) => l,
                        Err(e) => {
                            final_status = 502;
                            err_msg = e.to_string();
                            let _ = tx.blocking_send(Ok(
                                Event::default().data(format!("{{\"error\":\"read: {}\"}}", e))
                            ));
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

                        if payload == "[DONE]" {
                            let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
                            break;
                        }
                        if tx.blocking_send(Ok(Event::default().data(payload.to_string()))).is_err() {
                            break; // client disconnected
                        }
                    }
                    }
                    pool.note_success(&acct_uid);
                }
            }
            Err((status, _, e)) => {
                final_status = if status == 0 { 502 } else { status };
                err_msg = e.to_string();

                if final_status == 429 {
                    pool.cooldown(&acct_uid, Duration::from_secs(600));
                    warn!("429 from upstream for {}, cooldown applied", acct_email);
                } else if final_status >= 500 {
                    pool.note_error(&acct_uid);
                }

                let _ = tx.blocking_send(Ok(
                    Event::default().data(format!(
                        "{{\"error\":\"upstream returned {}\"}}",
                        final_status
                    ))
                ));
            }
        }

        // Emit structured log row.
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
            "[req] uid={} acct={} model={} status={} ttfb={}ms tokens={} speed={:.1}tok/s elapsed={}ms err={}",
            uid, acct_email, model, final_status,
            ttfb_ms, total_tokens, tok_speed, elapsed.as_millis(), err_msg
        );

        metrics.trace("/v1/chat/completions", &model, final_status, total_tokens, elapsed.as_millis() as u64);
        pool.release(&acct_uid);
    });

    // Return SSE stream to client.
    let stream = ReceiverStream::new(rx);
    Sse::new(stream)
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

async fn handle_models(State(state): State<SharedState>) -> Response {
    let dynamic = state.dynamic_models.read().clone();
    let data: Vec<Value> = if dynamic.is_empty() { STATIC_MODELS
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "created": 1753600000_i64,
                "owned_by": "workbuddy",
                "context_length": 131072,
            })
        })
        .collect() } else { dynamic };

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
    }
}
