//! CodeBuddy Global (International) Authentication Logic.
//!
//! Nền tảng: www.codebuddy.ai (Quốc tế)
//! Xác thực: Google OAuth / Email / GitHub
//! Đặc thù: Dùng hệ thống B3 Distributed Tracing (không gửi Origin/Referer để tránh xung đột CORS/gateway).

use std::time::Duration;
use rand::RngCore;
use serde::Deserialize;
use super::super::{PlatformLoginStart, PlatformPollOutcome, PlatformTokenBundle};

pub const API_BASE_GLOBAL: &str = "https://www.codebuddy.ai";
pub const UA_GLOBAL: &str = "CLI/2.137.1 CodeBuddy/2.137.1";
pub const TIMEOUT: Duration = Duration::from_secs(30);

fn agent(proxy_url: Option<&str>) -> ureq::Agent {
    let mut b = ureq::AgentBuilder::new().timeout(TIMEOUT);
    if let Some(p) = proxy_url.filter(|s| !s.trim().is_empty()) {
        if let Ok(proxy) = ureq::Proxy::new(p) {
            b = b.proxy(proxy);
        }
    }
    b.build()
}

fn new_trace_id() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn apply_global_headers(req: ureq::Request) -> ureq::Request {
    let req_id = new_trace_id();
    let span_id: String = new_trace_id().chars().take(16).collect();
    req.set("Content-Type", "application/json")
        .set("Accept", "application/json, text/plain, */*")
        .set("X-Requested-With", "XMLHttpRequest")
        .set("User-Agent", UA_GLOBAL)
        .set("Cache-Control", "no-cache")
        .set("Pragma", "no-cache")
        .set("X-Domain", "www.codebuddy.ai")
        .set("X-No-Authorization", "true")
        .set("X-No-Department-Info", "true")
        .set("X-No-Enterprise-Id", "true")
        .set("X-No-User-Id", "true")
        .set("X-Product", "SaaS")
        .set("X-Request-ID", &req_id)
        .set("X-B3-Sampled", "1")
        .set("X-B3-SpanId", &span_id)
        .set("X-B3-TraceId", &req_id)
        .set("b3", &format!("{req_id}-{span_id}-1-"))
}

#[derive(Deserialize)]
struct Envelope {
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    data: serde_json::Value,
}

pub fn start_login_global(proxy_url: Option<&str>) -> Result<PlatformLoginStart, String> {
    let ag = agent(proxy_url);
    let nonce = new_trace_id();
    let endpoint = format!("{API_BASE_GLOBAL}/v2/plugin/auth/state?platform=CLI&nonce={nonce}");
    let body = format!(r#"{{"nonce":"{nonce}"}}"#);

    let req = apply_global_headers(ag.post(&endpoint));
    let resp = req.send_bytes(body.as_bytes()).map_err(|e| format!("CodeBuddy Global auth request failed: {e}"))?;

    let env: Envelope = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    if env.code != 0 {
        return Err(format!("CodeBuddy Global API error code={}: {}", env.code, env.msg));
    }

    let state = env.data.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let auth_url = env.data.get("authUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if state.is_empty() || auth_url.is_empty() {
        return Err("CodeBuddy Global returned empty state or authUrl".into());
    }

    Ok(PlatformLoginStart {
        platform_id: "codebuddy_global".to_string(),
        auth_url,
        state,
    })
}

pub fn poll_login_global(state: &str, proxy_url: Option<&str>) -> Result<PlatformPollOutcome, String> {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_GLOBAL}/v2/plugin/auth/token?state={state}");
    let req = apply_global_headers(ag.get(&endpoint));

    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(code, _)) if code == 400 || code == 401 => return Ok(PlatformPollOutcome::Pending),
        Err(e) => return Err(format!("CodeBuddy Global poll error: {e}")),
    };

    let env: Envelope = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    if env.code != 0 {
        return Ok(PlatformPollOutcome::Pending);
    }

    let access = env.data.get("accessToken").and_then(|v| v.as_str()).unwrap_or("");
    if access.is_empty() {
        return Ok(PlatformPollOutcome::Pending);
    }

    Ok(PlatformPollOutcome::Done(PlatformTokenBundle {
        access_token: access.to_string(),
        refresh_token: env.data.get("refreshToken").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        expires_in: env.data.get("expiresIn").and_then(|v| v.as_i64()).unwrap_or(0),
        domain: "codebuddy.ai".to_string(),
    }))
}
