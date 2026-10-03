//! CodeBuddy CN (Tencent) Account & Quota Logic.
//!
//! Quản lý hồ sơ tài khoản, gói cước doanh nghiệp, và điểm danh hàng ngày.

use super::auth::{API_BASE_CN, UA_CN, TIMEOUT};
use super::super::PlatformAccountInfo;

pub const ENTERPRISE_PACKAGE_CODE: &str = "TCACA_code_enterprise";

fn agent(proxy_url: Option<&str>) -> ureq::Agent {
    let mut b = ureq::AgentBuilder::new().timeout(TIMEOUT);
    if let Some(p) = proxy_url.filter(|s| !s.trim().is_empty()) {
        if let Ok(proxy) = ureq::Proxy::new(p) {
            b = b.proxy(proxy);
        }
    }
    b.build()
}

pub fn fetch_account_cn(state: &str, access_token: &str, proxy_url: Option<&str>) -> PlatformAccountInfo {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_CN}/v2/plugin/login/account?state={state}");
    let req = ag.get(&endpoint)
        .set("Content-Type", "application/json")
        .set("User-Agent", UA_CN)
        .set("Origin", "https://www.codebuddy.cn")
        .set("Referer", "https://www.codebuddy.cn/")
        .set("Authorization", &format!("Bearer {access_token}"));

    let mut acct = PlatformAccountInfo {
        domain: "copilot.tencent.com".to_string(),
        ..Default::default()
    };

    if let Ok(resp) = req.call() {
        if let Ok(data) = resp.into_json::<serde_json::Value>() {
            if let Some(d) = data.get("data") {
                acct.uid = d.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                acct.nickname = d.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_string();
                acct.enterprise_id = d.get("enterpriseId").and_then(|v| v.as_str()).unwrap_or("").to_string();
                acct.plan_tier = if !acct.enterprise_id.is_empty() {
                    "Enterprise".to_string()
                } else {
                    "Individual".to_string()
                };
            }
        }
    }

    if acct.uid.is_empty() {
        acct.uid = "unknown_cn".to_string();
    }
    if acct.nickname.is_empty() {
        acct.nickname = acct.uid.clone();
    }

    acct
}

/// Điểm danh hàng ngày nhận quota/credits (đặc trưng của CodeBuddy CN).
pub fn daily_checkin_cn(access_token: &str, proxy_url: Option<&str>) -> Result<String, String> {
    let ag = agent(proxy_url);
    let endpoint = format!("{API_BASE_CN}/v2/plugin/checkin");
    let req = ag.post(&endpoint)
        .set("Content-Type", "application/json")
        .set("User-Agent", UA_CN)
        .set("Origin", "https://www.codebuddy.cn")
        .set("Referer", "https://www.codebuddy.cn/")
        .set("Authorization", &format!("Bearer {access_token}"));

    let resp = req.send_bytes(b"{}").map_err(|e| format!("Check-in HTTP error: {e}"))?;
    let data: serde_json::Value = resp.into_json().map_err(|e| format!("Parse json: {e}"))?;
    let code = data.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let msg = data.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown");

    if code == 0 {
        Ok(format!("Check-in success: {msg}"))
    } else {
        Ok(format!("Check-in note: code={code} {msg}"))
    }
}
