//! Zed Cloud Profile & Subscription Quota Logic.

use std::io::Read as _;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use super::super::PlatformAccountInfo;

pub const ZED_CLOUD_BASE_URL: &str = "https://cloud.zed.dev";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZedPlanUsage {
    #[serde(default)]
    pub edit_used: Option<i64>,
    #[serde(default)]
    pub edit_limit: Option<serde_json::Value>,
    #[serde(default)]
    pub period_start: Option<String>,
    #[serde(default)]
    pub period_end: Option<String>,
    #[serde(default)]
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZedProfile {
    pub user_id: String,
    pub github_login: String,
    pub name: String,
    pub org_id: Option<String>,
    pub org_name: Option<String>,
    pub plan: ZedPlanUsage,
}

pub fn fetch_user(id: &str, token: &str) -> Result<ZedProfile, String> {
    let url = format!("{ZED_CLOUD_BASE_URL}/client/users/me");
    let resp = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .build()
        .get(&url)
        .set("Authorization", &format!("{id} {token}"))
        .call()
        .map_err(|e| format!("zed users/me HTTP error: {e}"))?;

    if resp.status() != 200 {
        return Err(format!("zed users/me: upstream HTTP {}", resp.status()));
    }

    let mut raw = String::new();
    resp.into_reader()
        .take(64 * 1024)
        .read_to_string(&mut raw)
        .map_err(|e| format!("zed users/me read: {e}"))?;

    parse_profile(&raw)
}

pub fn parse_profile(raw: &str) -> Result<ZedProfile, String> {
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|e| format!("parse profile json: {e}"))?;
    let obj = v.as_object().ok_or_else(|| "profile not object".to_string())?;

    let user_id = obj.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let github_login = obj.get("github_login").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let name = obj.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();

    let (org_id, org_name) = match obj.get("organization").and_then(|x| x.as_object()) {
        Some(o) => (
            o.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()),
            o.get("name").and_then(|x| x.as_str()).map(|s| s.to_string()),
        ),
        None => (None, None),
    };

    let mut plan = ZedPlanUsage::default();
    if let Some(p) = obj.get("plan").and_then(|x| x.as_object()) {
        plan.plan = p.get("plan").and_then(|x| x.as_str()).map(|s| s.to_string());
        plan.period_start = p.get("period_start").and_then(|x| x.as_str()).map(|s| s.to_string());
        plan.period_end = p.get("period_end").and_then(|x| x.as_str()).map(|s| s.to_string());
        plan.edit_used = p.get("edit_used").and_then(|x| x.as_i64());
        plan.edit_limit = p.get("edit_limit").cloned();
    }

    Ok(ZedProfile {
        user_id,
        github_login,
        name,
        org_id,
        org_name,
        plan,
    })
}

pub fn to_platform_account(profile: &ZedProfile) -> PlatformAccountInfo {
    let label = if !profile.name.is_empty() {
        profile.name.clone()
    } else {
        profile.github_login.clone()
    };
    PlatformAccountInfo {
        uid: profile.user_id.clone(),
        nickname: label,
        domain: "zed.dev".to_string(),
        enterprise_id: profile.org_id.clone().unwrap_or_default(),
        plan_tier: profile.plan.plan.clone().unwrap_or_else(|| "Free".to_string()),
        credits: profile.plan.edit_used.unwrap_or(0),
    }
}
