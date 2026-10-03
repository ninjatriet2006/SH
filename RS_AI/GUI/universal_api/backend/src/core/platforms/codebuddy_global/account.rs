//! CodeBuddy Global Account & Profile Logic.
//!
//! Phân giải hồ sơ người dùng quốc tế, hỗ trợ giải mã JWT claims khi máy chủ không trả thông tin đầy đủ.

use super::auth::{API_BASE_GLOBAL, UA_GLOBAL, TIMEOUT};
use super::super::PlatformAccountInfo;

fn agent(proxy_url: Option<&str>) -> ureq::Agent {
    let mut b = ureq::AgentBuilder::new().timeout(TIMEOUT);
    if let Some(p) = proxy_url.filter(|s| !s.trim().is_empty()) {
        if let Ok(proxy) = ureq::Proxy::new(p) {
            b = b.proxy(proxy);
        }
    }
    b.build()
}

pub fn fetch_account_global(state: &str, access_token: &str, proxy_url: Option<&str>) -> PlatformAccountInfo {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_GLOBAL}/v2/plugin/login/account?state={state}");
    let req = ag.get(&endpoint)
        .set("Content-Type", "application/json")
        .set("User-Agent", UA_GLOBAL)
        .set("Authorization", &format!("Bearer {access_token}"));

    let mut acct = PlatformAccountInfo {
        domain: "codebuddy.ai".to_string(),
        ..Default::default()
    };

    if let Ok(resp) = req.call() {
        if let Ok(data) = resp.into_json::<serde_json::Value>() {
            if let Some(d) = data.get("data") {
                acct.uid = d.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                acct.nickname = d.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_string();
                acct.enterprise_id = d.get("enterpriseId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            }
        }
    }

    // Fallback giải mã JWT claims nếu endpoint không trả đủ tên/email
    if acct.uid.is_empty() || acct.nickname.is_empty() {
        if let Some(claims) = decode_jwt_claims(access_token) {
            if acct.uid.is_empty() {
                acct.uid = claim_str(&claims, &["email", "preferred_username", "sub"]);
            }
            if acct.nickname.is_empty() {
                acct.nickname = claim_str(&claims, &["name", "preferred_username"]);
            }
        }
    }

    if acct.uid.is_empty() {
        acct.uid = "unknown_global".to_string();
    }
    if acct.nickname.is_empty() {
        acct.nickname = acct.uid.clone();
    }

    acct
}

fn decode_jwt_claims(token: &str) -> Option<serde_json::Value> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let mut parts = token.split('.');
    let (_h, payload, _s) = (parts.next()?, parts.next()?, parts.next()?);
    let raw = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn claim_str(claims: &serde_json::Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(v) = claims.get(*k).and_then(|v| v.as_str()) {
            if !v.trim().is_empty() {
                return v.to_string();
            }
        }
    }
    String::new()
}
