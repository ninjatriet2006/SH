//! CodeBuddy Global (International) Authentication Logic.
//!
//! Nền tảng: www.codebuddy.ai (Quốc tế)
//! Xác thực: Google OAuth / Email / GitHub (chuẩn IDE desktop)
//! Đặc thù: B3 Distributed Tracing + session IDE kéo dài.

use std::time::Duration;
use rand::RngCore;
use serde::Deserialize;
use serde_json::Value;
use super::super::{PlatformLoginStart, PlatformPollOutcome, PlatformTokenBundle};

pub const API_BASE_GLOBAL: &str = "https://www.codebuddy.ai";
pub const UA_CHROMIUM: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub const UA_GLOBAL: &str = UA_CHROMIUM;
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

fn generate_uuid_v4() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

fn apply_global_headers(req: ureq::Request) -> ureq::Request {
    let req_id = new_trace_id();
    let span_id: String = new_trace_id().chars().take(16).collect();
    req.set("Content-Type", "application/json")
        .set("Accept", "application/json, text/plain, */*")
        .set("X-Requested-With", "XMLHttpRequest")
        .set("User-Agent", UA_CHROMIUM)
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
    data: Value,
}

pub fn start_login_global(proxy_url: Option<&str>) -> Result<PlatformLoginStart, String> {
    let ag = agent(proxy_url);
    let nonce = new_trace_id();
    // Dùng platform=ide thay vì CLI để được cấp session lâu dài
    let endpoint = format!("{API_BASE_GLOBAL}/v2/plugin/auth/state?platform=ide&nonce={nonce}");
    let body = format!(r#"{{"nonce":"{nonce}"}}"#);

    let req = apply_global_headers(ag.post(&endpoint));
    let resp = req.send_bytes(body.as_bytes()).map_err(|e| format!("CodeBuddy Global auth request failed: {e}"))?;

    let env: Envelope = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    if env.code != 0 {
        return Err(format!("CodeBuddy Global API error code={}: {}", env.code, env.msg));
    }

    let state = env.data.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let raw_auth_url = env.data.get("authUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if state.is_empty() || raw_auth_url.is_empty() {
        return Err("CodeBuddy Global returned empty state or authUrl".into());
    }

    let login_session_id = generate_uuid_v4();
    let auth_url = if raw_auth_url.contains('?') {
        format!("{raw_auth_url}&loginSessionId={login_session_id}&version=1.3.65")
    } else {
        format!("{raw_auth_url}?loginSessionId={login_session_id}&version=1.3.65")
    };

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
        domain: env.data.get("domain").and_then(|v| v.as_str()).unwrap_or("codebuddy.ai").to_string(),
    }))
}

/// Làm mới Token định kỳ cho CodeBuddy Global
pub fn refresh_token_global(
    access_token: &str,
    refresh_token: &str,
    domain: Option<&str>,
    proxy_url: Option<&str>,
) -> Result<PlatformTokenBundle, String> {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_GLOBAL}/v2/plugin/auth/token/refresh");
    let mut req = ag.post(&endpoint)
        .set("Content-Type", "application/json")
        .set("User-Agent", UA_CHROMIUM)
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("X-Refresh-Token", refresh_token)
        .set("X-Auth-Refresh-Source", "ide-main");

    if let Some(d) = domain {
        req = req.set("X-Domain", d);
    }

    let resp = req.send_bytes(b"{}").map_err(|e| format!("CodeBuddy Global refresh failed: {e}"))?;
    let env: Envelope = resp.into_json().map_err(|e| format!("Parse refresh response error: {e}"))?;
    if env.code != 0 && env.code != 200 {
        return Err(format!("CodeBuddy Global refresh error code={}: {}", env.code, env.msg));
    }

    let new_access = env.data.get("accessToken")
        .or_else(|| env.data.get("access_token"))
        .and_then(|v| v.as_str())
        .unwrap_or(access_token)
        .to_string();

    let new_refresh = env.data.get("refreshToken")
        .or_else(|| env.data.get("refresh_token"))
        .and_then(|v| v.as_str())
        .unwrap_or(refresh_token)
        .to_string();

    let expires_in = env.data.get("expiresIn")
        .or_else(|| env.data.get("expires_in"))
        .and_then(|v| v.as_i64())
        .unwrap_or(86400 * 7);

    Ok(PlatformTokenBundle {
        access_token: new_access,
        refresh_token: new_refresh,
        expires_in,
        domain: domain.unwrap_or("codebuddy.ai").to_string(),
    })
}
