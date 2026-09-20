//! Smoke test gateway thật: boot axum router trên port ngẫu nhiên rồi gõ từng
//! endpoint — để phân biệt 404/503/401 nào là đúng, cái nào là bug.
//!
//! Chạy: cargo test -p universal_api --test gateway_smoke

use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use tokio::sync::watch;
use universal_api_lib::core::audit::AuditBuffer;
use universal_api_lib::core::config::Config;
use universal_api_lib::core::pool::Pool;
use universal_api_lib::core::prompt::PromptMode;
use universal_api_lib::core::redisstore::NoopStore;
use universal_api_lib::core::runtime::Metrics;
use universal_api_lib::core::server::{build_router, AppState, DegradeGate};
use universal_api_lib::core::session;

fn test_state(api_key: &str) -> Arc<AppState> {
    let mut config = Config::default();
    config.api_key = api_key.to_string();
    let pool = Pool::new(":memory:");
    let session_cfg = session::Config {
        ttl: Duration::from_secs(1800),
        gc_interval: Duration::from_secs(300),
        store: Arc::new(NoopStore),
        available: Arc::new(|| vec![]),
    };
    let (shutdown_tx, _) = watch::channel(false);
    Arc::new(AppState {
        config,
        pool,
        session: session::SessionRouter::new(session_cfg),
        prompt_mode: PromptMode::Passthrough,
        degrade: DegradeGate::new(),
        shutdown_tx,
        metrics: Arc::new(Metrics::default()),
        dynamic_models: Arc::new(RwLock::new(Default::default())),
        external_models: Arc::new(RwLock::new(Vec::new())),
        zed_models: Arc::new(RwLock::new(Default::default())),
        zed_tokens: Arc::new(universal_api_lib::core::providers::zed::ZedTokenCache::default()),
        storage: None,
        audit: Arc::new(AuditBuffer::default()),
    })
}

async fn boot(api_key: &str) -> String {
    let router = build_router(test_state(api_key));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    // Đợi server nhận socket.
    tokio::time::sleep(Duration::from_millis(100)).await;
    format!("http://127.0.0.1:{port}")
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build()
}

/// Gọi request, TRẢ response kể cả status lỗi (ureq mặc định biến 4xx/5xx
/// thành Err — test này cần assert chính các status đó).
fn call(req: ureq::Request) -> ureq::Response {
    match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => panic!("transport failed: {e}"),
    }
}

fn send(req: ureq::Request, body: &str) -> ureq::Response {
    match req.send_string(body) {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => panic!("transport failed: {e}"),
    }
}

/// NOTE: bắt buộc multi_thread — ureq blocking mà chạy trên runtime
/// single-thread (`#[tokio::test]` mặc định) sẽ chặn luôn thread duy nhất,
/// server trong cùng runtime không được poll → timeout giả.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn open_access_endpoints() {
    let base = boot("").await;
    let ag = agent();

    // /healthz public — 503 khi chưa có account khỏe (KHÔNG phải lỗi bind).
    let resp = call(ag.get(&format!("{base}/healthz")));
    assert_eq!(resp.status(), 503);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["healthy"], 0);

    // /v1/models open access khi api_key trống.
    let resp = call(ag.get(&format!("{base}/v1/models")));
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["object"], "list");
    assert!(body["data"].as_array().unwrap().len() >= 10);

    // / (root) KHÔNG có route → 404. Mở base URL bằng browser là thấy trang trắng.
    let resp = call(ag.get(&format!("{base}/")));
    assert_eq!(resp.status(), 404);

    // GET nhầm method trên POST-only route → 405.
    let resp = call(ag.get(&format!("{base}/v1/chat/completions")));
    assert_eq!(resp.status(), 405);

    // POST chat không account → HTTP 200 + SSE (lỗi nằm TRONG stream, không phải HTTP status).
    let resp = send(
        ag.post(&format!("{base}/v1/chat/completions"))
            .set("Content-Type", "application/json"),
        r#"{"model":"glm-5.2","messages":[{"role":"user","content":"hi"}],"stream":false}"#,
    );
    assert_eq!(resp.status(), 200);
    assert!(resp.header("content-type").unwrap_or("").contains("text/event-stream"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scoped_provider_endpoints() {
    let base = boot("").await;
    let ag = agent();

    // /zed/v1/models khi chưa refresh: list RỖNG (thuần Zed, không lẫn static).
    let resp = call(ag.get(&format!("{base}/zed/v1/models")));
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["object"], "list");
    assert!(body["data"].as_array().unwrap().is_empty());

    // /codebuddy/v1/models: static fallback (không external/zed).
    let resp = call(ag.get(&format!("{base}/codebuddy/v1/models")));
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["data"].as_array().unwrap().len() >= 10);

    // POST /zed/... với model không trong cache → SSE 200 + lỗi rõ (không rơi pool).
    let resp = send(
        ag.post(&format!("{base}/zed/v1/chat/completions"))
            .set("Content-Type", "application/json"),
        r#"{"model":"zed/whatever","messages":[{"role":"user","content":"hi"}],"stream":false}"#,
    );
    assert_eq!(resp.status(), 200);
    assert!(resp.header("content-type").unwrap_or("").contains("text/event-stream"));

    // POST /codebuddy/... không account → SSE 200 (đi pool, lỗi trong stream).
    let resp = send(
        ag.post(&format!("{base}/codebuddy/v1/chat/completions"))
            .set("Content-Type", "application/json"),
        r#"{"model":"glm-5.2","messages":[{"role":"user","content":"hi"}],"stream":false}"#,
    );
    assert_eq!(resp.status(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn auth_protected_endpoints() {
    let base = boot("secret-key").await;
    let ag = agent();

    // Không key → 401.
    let resp = call(ag.get(&format!("{base}/v1/models")));
    assert_eq!(resp.status(), 401);

    // Sai key → 401.
    let resp = call(ag
        .get(&format!("{base}/v1/models"))
        .set("Authorization", "Bearer wrong"));
    assert_eq!(resp.status(), 401);

    // Đúng key (Bearer) → 200.
    let resp = call(ag
        .get(&format!("{base}/v1/models"))
        .set("Authorization", "Bearer secret-key"));
    assert_eq!(resp.status(), 200);

    // x-api-key cũng được.
    let resp = call(ag
        .get(&format!("{base}/v1/models"))
        .set("x-api-key", "secret-key"));
    assert_eq!(resp.status(), 200);

    // /healthz vẫn public kể cả khi có key.
    let resp = call(ag.get(&format!("{base}/healthz")));
    assert_eq!(resp.status(), 503); // 503 vì chưa có account, nhưng KHÔNG 401
}
