//! Upstream HTTP client — auth state, JSON envelope, and all API methods.
//!
//! Ported from Go: internal/upstream/client.go + api_core.go

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::core::auth::Auth;

use super::errors::{self, ErrKind, UpstreamError};
use super::headers::{self, HeaderSet};
use super::realms;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const STREAM_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(serde::Deserialize)]
struct ApiEnvelope {
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    data: serde_json::Value,
}

/// Upstream HTTP client. Thread-safe via `Arc<RwLock<...>>` for efforts cache.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    chat_agent: ureq::Agent,
    efforts: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub sanitize_fingerprints: bool,
    pub user_agent: String,
    pub realm: String,
    pub custom_headers: HashMap<String, String>,
}

pub type UpstreamClient = Client;

impl Client {
    pub fn new(proxy_url: &str) -> Self {
        let proxy = (!proxy_url.trim().is_empty())
            .then(|| ureq::Proxy::new(proxy_url).ok())
            .flatten();
        let mut agent_builder = ureq::AgentBuilder::new().timeout(DEFAULT_TIMEOUT);
        let mut chat_builder = ureq::AgentBuilder::new().timeout(STREAM_TIMEOUT);
        if let Some(proxy) = proxy.clone() { agent_builder = agent_builder.proxy(proxy); }
        if let Some(proxy) = proxy { chat_builder = chat_builder.proxy(proxy); }
        let agent = agent_builder.build();
        let chat_agent = chat_builder.build();

        Self {
            agent,
            chat_agent,
            efforts: Arc::new(RwLock::new(HashMap::new())),
            sanitize_fingerprints: true,
            user_agent: String::new(),
            realm: realms::REALM_CN.to_string(),
            custom_headers: HashMap::new(),
        }
    }

    pub fn efforts_snapshot(&self) -> Option<HashMap<String, Vec<String>>> {
        let guard = self.efforts.read().ok()?;
        if guard.is_empty() { None } else { Some(guard.clone()) }
    }

    fn chat_base(&self, auth: &Auth) -> &'static str {
        realms::route_chat_base(auth, &self.realm)
    }

    fn billing_base(&self, auth: &Auth) -> &'static str {
        realms::route_billing_base(auth, &self.realm)
    }

    fn ua(&self) -> &str { &self.user_agent }

    pub fn prepare_body(&self, body: &[u8]) -> Vec<u8> {
        let efforts = self.efforts_snapshot();
        super::payload::prepare_body_with_efforts(body, self.sanitize_fingerprints, efforts.as_ref())
    }

    // ── Helper: send GET, read body, unpack envelope ──────────────────

    fn do_get(
        &self,
        agent: &ureq::Agent,
        url: &str,
        hdrs: &HeaderSet,
    ) -> Result<serde_json::Value, UpstreamError> {
        let mut req = agent.get(url);
        for (k, v) in hdrs.iter() {
            req = req.set(k.as_str(), v.as_str());
        }
        let resp = req.call().map_err(UpstreamError::from_ureq)?;
        self.unpack_envelope(resp)
    }

    // ── Helper: send POST, read body, unpack envelope ─────────────────

    fn do_post(
        &self,
        agent: &ureq::Agent,
        url: &str,
        hdrs: &HeaderSet,
        body: Option<&[u8]>,
    ) -> Result<serde_json::Value, UpstreamError> {
        let mut req = agent.post(url);
        for (k, v) in hdrs.iter() {
            req = req.set(k.as_str(), v.as_str());
        }
        let resp = match body {
            Some(b) => req.send(b).map_err(UpstreamError::from_ureq)?,
            None => req.send_bytes(&[]).map_err(UpstreamError::from_ureq)?,
        };
        self.unpack_envelope(resp)
    }

    /// Read response body and unpack the API envelope `{code, msg, data}`.
    fn unpack_envelope(
        &self,
        resp: ureq::Response,
    ) -> Result<serde_json::Value, UpstreamError> {
        let status = resp.status();
        let mut raw = String::new();
        resp.into_reader().read_to_string(&mut raw)
            .map_err(|e| UpstreamError::new(ErrKind::Client, 0, e.to_string()))?;

        if status >= 400 {
            let kind = errors::classify(status, &raw);
            return Err(UpstreamError::new(kind, status, errors::truncate_str(&raw, 200)));
        }
        let env: ApiEnvelope = serde_json::from_str(&raw)
            .map_err(|e| UpstreamError::new(ErrKind::Client, status,
                format!("parse failed: {} (body: {})", e, errors::truncate_str(&raw, 120))))?;
        if env.code != 0 {
            let kind = errors::classify(status, &env.msg);
            let kind = if kind == ErrKind::None { ErrKind::Client } else { kind };
            return Err(UpstreamError::new(kind, status,
                format!("code={} msg={}", env.code, errors::truncate_str(&env.msg, 160))));
        }
        Ok(env.data)
    }

    fn billing_json(&self, auth: &Auth, method: &str, path: &str, body: Option<&[u8]>) -> Result<serde_json::Value, UpstreamError> {
        let url = format!("{}{}", self.billing_base(auth), path);
        let hdrs = headers::billing_headers(auth, self.ua());
        if method == "GET" { self.do_get(&self.agent, &url, &hdrs) } else { self.do_post(&self.agent, &url, &hdrs, body) }
    }

    fn growth_json(&self, auth: &Auth, method: &str, path: &str, body: Option<&[u8]>) -> Result<serde_json::Value, UpstreamError> {
        let url = format!("{}{}", self.chat_base(auth), path);
        let hdrs = headers::billing_headers(auth, self.ua());
        if method == "GET" { self.do_get(&self.agent, &url, &hdrs) } else { self.do_post(&self.agent, &url, &hdrs, body) }
    }

    pub(crate) fn request_json(&self, auth: &Auth, method: &str, path: &str, body: Option<&[u8]>) -> Result<serde_json::Value, UpstreamError> {
        self.growth_json(auth, method, path, body)
    }

    // ── Public API ─────────────────────────────────────────────────────

    pub fn refresh_token(&self, auth: &mut Auth) -> Result<(), UpstreamError> {
        if auth.refresh_token.trim().is_empty() {
            return Err(UpstreamError::new(ErrKind::Client, 0, "no refreshToken"));
        }
        let url = format!("{}/v2/plugin/auth/token/refresh", self.chat_base(auth));
        let hdrs = headers::refresh_headers(auth, self.ua());
        let data = self.do_post(&self.agent, &url, &hdrs, None)?;

        #[derive(serde::Deserialize)]
        struct Tok {
            #[serde(rename = "accessToken", default)] access_token: String,
            #[serde(rename = "refreshToken", default)] refresh_token: String,
            #[serde(rename = "expiresIn", default)] expires_in: i64,
            #[serde(default)] domain: String,
        }
        let tok: Tok = serde_json::from_value(data)?;
        if tok.access_token.is_empty() {
            return Err(UpstreamError::new(ErrKind::Client, 0, "refresh_failed: no accessToken"));
        }
        auth.access_token = tok.access_token;
        if !tok.refresh_token.is_empty() { auth.refresh_token = tok.refresh_token; }
        if !tok.domain.is_empty() { auth.domain = tok.domain; }
        if tok.expires_in > 0 {
            auth.expires_at = chrono::Utc::now().timestamp() + tok.expires_in;
        }
        Ok(())
    }

    /// Send chat SSE request. Returns (reader, status, final_headers) on success,
    /// or (status, raw_body, error) on failure.
    pub fn chat_stream(
        &self, auth: &Auth, body: &[u8],
    ) -> Result<(Box<dyn Read + Send + Sync>, u16, Vec<(String, String)>), (u16, Vec<u8>, UpstreamError)> {
        let url = format!("{}/v2/chat/completions", self.chat_base(auth));
        let prepared = self.prepare_body(body);
        let mut hdrs = headers::chat_headers(auth, self.ua());
        for (k, v) in &self.custom_headers {
            hdrs.pairs_mut().push((k.clone(), v.clone()));
        }
        let sent_headers: Vec<(String, String)> = hdrs.iter().cloned().collect();
        let mut req = self.chat_agent.post(&url);
        for (k, v) in hdrs.iter() {
            req = req.set(k.as_str(), v.as_str());
        }
        let resp = match req.send(prepared.as_slice()) {
            Ok(r) => r,
            Err(e) => return Err((0, vec![], UpstreamError::from_ureq(e))),
        };
        let status = resp.status();
        if status >= 400 {
            let mut raw = Vec::new();
            let _ = resp.into_reader().take(1 << 20).read_to_end(&mut raw);
            let body_str = String::from_utf8_lossy(&raw);
            let kind = errors::classify(status, &body_str);
            log::warn!("chat_stream uid={}: upstream {} {} body={}",
                auth.uid, status, kind, errors::truncate_str(&body_str, 200));
            return Err((status, raw, UpstreamError::new(kind, status, "upstream error")));
        }
        Ok((Box::new(resp.into_reader()), status, sent_headers))
    }

    pub fn fetch_models(&self, auth: &Auth) -> Result<Vec<ModelInfo>, UpstreamError> {
        let url = format!("{}/console/enterprises/personal/models", self.chat_base(auth));
        let mut hdrs = headers::common_headers(auth, self.ua());
        hdrs.pairs_mut().push(("Authorization".into(), format!("Bearer {}", auth.access_token)));
        let mut req = self.agent.get(&url);
        for (k, v) in hdrs.iter() {
            req = req.set(k.as_str(), v.as_str());
        }
        let resp = req.call().map_err(UpstreamError::from_ureq)?;
        let status = resp.status();
        let mut raw = String::new();
        resp.into_reader().read_to_string(&mut raw)
            .map_err(|e| UpstreamError::new(ErrKind::Client, 0, e.to_string()))?;
        if status != 200 {
            return Err(UpstreamError::new(ErrKind::Server, status,
                format!("models api status {}: {}", status, errors::truncate_str(&raw, 120))));
        }

        #[derive(serde::Deserialize)] struct Env { code: i64, data: EnvData }
        #[derive(serde::Deserialize)] struct EnvData { #[serde(default)] models: Vec<RawModel>, #[serde(default)] agents: Vec<RawAgent> }
        #[derive(serde::Deserialize)] struct RawModel { id: String, #[serde(default)] name: String, #[serde(rename="maxInputTokens",default)] max_input_tokens: i64, #[serde(rename="maxOutputTokens",default)] max_output_tokens: i64, #[serde(default)] disabled: bool, #[serde(default)] reasoning: RawReasoning }
        #[derive(serde::Deserialize, Default)] struct RawReasoning { #[serde(rename="supportedEfforts",default)] supported_efforts: Vec<String> }
        #[derive(serde::Deserialize)] struct RawAgent { name: String, #[serde(default)] models: Vec<String> }

        let env: Env = serde_json::from_str(&raw)?;
        if env.code != 0 {
            return Err(UpstreamError::new(ErrKind::Client, status, format!("models api code={}", env.code)));
        }
        let cli_ids: Vec<String> = env.data.agents.iter()
            .find(|a| a.name == "cli").map(|a| a.models.clone()).unwrap_or_default();
        if cli_ids.is_empty() {
            return Err(UpstreamError::new(ErrKind::Client, status, "no cli agent models found"));
        }
        let dyn_map: HashMap<String, &RawModel> = env.data.models.iter().map(|m| (m.id.clone(), m)).collect();
        let mut out = Vec::new();
        for id in &cli_ids {
            if let Some(m) = dyn_map.get(id) {
                if m.disabled { continue; }
                out.push(ModelInfo { id: m.id.clone(), name: m.name.clone(), context_window: m.max_input_tokens, max_tokens: m.max_output_tokens, efforts: m.reasoning.supported_efforts.clone() });
            }
        }
        if out.is_empty() {
            return Err(UpstreamError::new(ErrKind::Client, status, "models api returned empty list"));
        }
        let mut cache = HashMap::new();
        for mi in &out {
            if !mi.efforts.is_empty() { cache.insert(mi.id.clone(), mi.efforts.clone()); }
        }
        if let Ok(mut guard) = self.efforts.write() { *guard = cache; }
        Ok(out)
    }

    pub fn user_resource(&self, auth: &Auth) -> Result<i64, UpstreamError> {
        let now = chrono::Utc::now();
        let body = serde_json::json!({
            "PageNumber": 1, "PageSize": 100, "ProductCode": "p_tcaca", "Status": [0, 3],
            "PackageEndTimeRangeBegin": now.format("%Y-%m-%d %H:%M:%S").to_string(),
            "PackageEndTimeRangeEnd": (now + chrono::Duration::days(365 * 101)).format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        let raw = serde_json::to_vec(&body)?;
        let data = self.billing_json(auth, "POST", "/v2/billing/meter/get-user-resource", Some(&raw))?;

        #[derive(serde::Deserialize)] struct Resp { #[serde(rename="Response")] response: Inner }
        #[derive(serde::Deserialize)] struct Inner { #[serde(rename="Data")] data: Data }
        #[derive(serde::Deserialize)] struct Data { #[serde(rename="Accounts",default)] accounts: Vec<Acct> }
        #[derive(serde::Deserialize)] struct Acct {
            #[serde(rename="CapacityRemain",default)] capacity_remain: i64,
            #[serde(rename="CycleCapacitySize",default)] cycle_capacity_size: i64,
            #[serde(rename="CycleCapacityRemain",default)] cycle_capacity_remain: i64,
            #[serde(rename="CycleCapacityUsed",default)] cycle_capacity_used: i64,
        }
        let resp: Resp = serde_json::from_value(data)?;
        let mut remain: i64 = 0;
        for a in &resp.response.data.accounts {
            let r = if a.cycle_capacity_size > 0 { a.cycle_capacity_remain }
                    else if a.cycle_capacity_remain > 0 || a.cycle_capacity_used > 0 { a.cycle_capacity_remain }
                    else { a.capacity_remain };
            remain += r.max(0);
        }
        Ok(remain)
    }

    pub fn daily_checkin(&self, auth: &Auth) -> Result<(), UpstreamError> {
        self.billing_json(auth, "POST", "/v2/billing/meter/daily-checkin", Some(b"{}"))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: i64,
    pub max_tokens: i64,
    pub efforts: Vec<String>,
}
