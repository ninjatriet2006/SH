//! CodeBuddy CN (Tencent Coding Copilot) Authentication Logic.
//!
//! Nền tảng: copilot.tencent.com / codebuddy.cn (Nội địa Trung Quốc)
//! Đăng nhập: WeChat QR / QQ / Điện thoại qua Device Code State flow.

use std::time::Duration;
use serde::Deserialize;
use super::super::{PlatformLoginStart, PlatformPollOutcome, PlatformTokenBundle};

pub const API_BASE_CN: &str = "https://copilot.tencent.com";
pub const UA_CN: &str = "CLI/2.137.1 CodeBuddy/2.137.1";
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

fn apply_cn_headers(req: ureq::Request) -> ureq::Request {
    req.set("Content-Type", "application/json")
        .set("Accept", "application/json, text/plain, */*")
        .set("X-Requested-With", "XMLHttpRequest")
        .set("User-Agent", UA_CN)
        .set("Origin", "https://www.codebuddy.cn")
        .set("Referer", "https://www.codebuddy.cn/")
}

#[derive(Deserialize)]
struct Envelope {
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    data: serde_json::Value,
}

pub fn start_login_cn(proxy_url: Option<&str>) -> Result<PlatformLoginStart, String> {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_CN}/v2/plugin/auth/state?platform=CLI");
    let req = apply_cn_headers(ag.post(&endpoint));
    let resp = req.send_bytes(b"{}").map_err(|e| format!("CodeBuddy CN auth request failed: {e}"))?;

    let env: Envelope = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    if env.code != 0 {
        return Err(format!("CodeBuddy CN API error code={}: {}", env.code, env.msg));
    }

    let state = env.data.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let auth_url = env.data.get("authUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if state.is_empty() || auth_url.is_empty() {
        return Err("CodeBuddy CN returned empty state or authUrl".into());
    }

    Ok(PlatformLoginStart {
        platform_id: "codebuddy_cn".to_string(),
        auth_url,
        state,
    })
}

pub fn poll_login_cn(state: &str, proxy_url: Option<&str>) -> Result<PlatformPollOutcome, String> {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_CN}/v2/plugin/auth/token?state={state}");
    let req = apply_cn_headers(ag.get(&endpoint));
    
    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(code, _)) if code == 400 || code == 401 => return Ok(PlatformPollOutcome::Pending),
        Err(e) => return Err(format!("CodeBuddy CN poll error: {e}")),
    };

    let env: Envelope = resp.into_json().map_err(|e| format!("Parse error: {e}"))?;
    if env.code != 0 {
        // code != 0 nghĩa là user chưa quét mã / chưa bấm xác nhận
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
        domain: env.data.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }))
}
