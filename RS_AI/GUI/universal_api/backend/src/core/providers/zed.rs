//! Native Zed provider block — port từ anti-api `services/zed/{oauth,chat}.ts`.
//!
//! Cấu trúc theo mẫu CodeBuddy: **Account** (import credential file, validate,
//! enable/disable, test) + **Configuration** (timeouts, system_id). KHÔNG
//! scheduler, KHÔNG quick actions (đặc thù CodeBuddy).
//!
//! Luồng upstream (host cố định, request không được override):
//! 1. Credential file `{type:"zed", id, access_token}` (owner-only, ≤64KiB).
//! 2. Validate: `GET https://cloud.zed.dev/client/users/me`,
//!    header `Authorization: {id} {accessToken}`.
//! 3. LLM token (TTL 15p): `POST .../client/llm_tokens` + `Authorization` trên.
//! 4. Models (TTL 5p): `GET .../models` + `Bearer {llmToken}`.
//! 5. Completion: `POST .../completions` body `{provider, model,
//!    provider_request}` — NDJSON lines, parse theo `model.provider`.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use axum::response::{IntoResponse, Response, Sse, sse::Event};
use log::info;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

use crate::core::protocol::{
    anthropic_content_block_delta, anthropic_content_block_start, anthropic_content_block_stop,
    anthropic_message_delta, anthropic_message_start, anthropic_message_stop,
};
use crate::core::server::SharedState;

// ─── Hằng số (mirror anti-api) ───────────────────────────────────────────────

pub const ZED_SERVER_URL: &str = "https://zed.dev";
pub const ZED_CLOUD_BASE_URL: &str = "https://cloud.zed.dev";
pub const MAX_CREDENTIAL_FILE_BYTES: u64 = 64 * 1024;
const MAX_FIELD_LEN: usize = 16 * 1024;

const LLM_TOKEN_TTL: Duration = Duration::from_secs(15 * 60);

fn request_timeout() -> Duration {
    Duration::from_secs(8)
}

// ─── Credential ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ZedCredential {
    pub id: String,
    pub access_token: String,
}

fn bad_cred(msg: &str) -> String {
    // Không bao giờ nhả path/nội dung file ra ngoài (mirror TS SAFE_IMPORT_ERROR).
    let _ = msg;
    "Set credential file to an absolute, owner-only Zed JSON file {\"type\":\"zed\",\"id\":\"...\",\"access_token\":\"...\"}.".to_string()
}

/// Parse nội dung file credential. Chỉ đọc field chuẩn, bỏ qua field lạ.
pub fn parse_credential_json(raw: &str) -> Result<ZedCredential, String> {
    if raw.len() as u64 > MAX_CREDENTIAL_FILE_BYTES {
        return Err(bad_cred("size"));
    }
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| bad_cred("json"))?;
    let obj = v.as_object().ok_or_else(|| bad_cred("shape"))?;
    let typ = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let id = obj.get("id").and_then(|t| t.as_str()).unwrap_or("").trim();
    let token = obj
        .get("access_token")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim();
    if typ != "zed" || id.is_empty() || token.is_empty() {
        return Err(bad_cred("fields"));
    }
    if id.len() > MAX_FIELD_LEN || token.len() > MAX_FIELD_LEN {
        return Err(bad_cred("length"));
    }
    Ok(ZedCredential { id: id.to_string(), access_token: token.to_string() })
}

/// Đọc file credential với guard như TS: absolute, không symlink, là file,
/// ≤64KiB, Unix owner-only (0600).
pub fn read_credential_file(path: &str) -> Result<ZedCredential, String> {
    use std::os::unix::ffi::OsStrExt;
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(bad_cred("relative"));
    }
    let meta = std::fs::symlink_metadata(p).map_err(|_| bad_cred("stat"))?;
    if meta.file_type().is_symlink() {
        return Err(bad_cred("symlink"));
    }
    if !meta.is_file() || meta.len() > MAX_CREDENTIAL_FILE_BYTES {
        return Err(bad_cred("type/size"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(bad_cred("perms"));
        }
    }
    let raw = std::fs::read(p).map_err(|_| bad_cred("read"))?;
    // Từ chối byte NUL / nội dung không phải UTF-8 hợp lệ ở biên.
    let text = std::str::from_utf8(&raw).map_err(|_| bad_cred("utf8"))?;
    let _ = OsStrExt::as_bytes(p.as_os_str());
    parse_credential_json(text)
}

// ─── HTTP ────────────────────────────────────────────────────────────────────

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(timeout).build()
}

fn auth_header(id: &str, token: &str) -> String {
    format!("{id} {token}")
}

fn http_error(label: &str, e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let mut body = String::new();
            let _ = resp.into_reader().take(4096).read_to_string(&mut body);
            // Không nhả body upstream thô khi có thể chứa credential — cắt ngắn.
            let short: String = body.chars().take(200).collect();
            format!("{label}: upstream HTTP {code} {short}")
        }
        other => format!("{label}: {other}"),
    }
}

use std::io::Read as _;

// ─── Profile / quota ─────────────────────────────────────────────────────────

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

/// `GET /client/users/me` — validate credential + lấy profile/quota hiển thị.
pub fn fetch_user(id: &str, token: &str) -> Result<ZedProfile, String> {
    let url = format!("{ZED_CLOUD_BASE_URL}/client/users/me");
    let resp = agent(request_timeout())
        .get(&url)
        .set("Authorization", &auth_header(id, token))
        .call()
        .map_err(|e| http_error("zed users/me", e))?;
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

fn parse_profile(raw: &str) -> Result<ZedProfile, String> {
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "zed users/me: invalid JSON".to_string())?;
    let user = v.get("user").ok_or("zed users/me: missing user")?;
    let uid = user
        .get("id")
        .and_then(|i| i.as_i64().map(|n| n.to_string()).or_else(|| i.as_str().map(|s| s.to_string())))
        .filter(|s| !s.is_empty())
        .ok_or("zed users/me: missing user.id")?;
    let github_login = user.get("github_login").and_then(|s| s.as_str()).unwrap_or("").to_string();
    let name = user
        .get("name")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let (org_id, org_name) = match v.get("organizations").and_then(|o| o.as_array()).and_then(|a| a.first()) {
        Some(o) => (
            o.get("id").and_then(|s| s.as_str()).map(|s| s.to_string()),
            o.get("name").and_then(|s| s.as_str()).map(|s| s.to_string()),
        ),
        None => (None, None),
    };
    let mut plan = ZedPlanUsage::default();
    if let Some(p) = v.get("plan") {
        plan.plan = p.get("plan_v3").and_then(|s| s.as_str()).map(|s| s.to_string());
        if let Some(u) = p.get("usage").and_then(|u| u.get("edit_predictions")) {
            plan.edit_used = u.get("used").and_then(|n| n.as_i64());
            plan.edit_limit = u.get("limit").cloned();
        }
        if let Some(s) = p.get("subscription_period") {
            plan.period_start = s.get("started_at").and_then(|x| x.as_str()).map(|s| s.to_string());
            plan.period_end = s.get("ended_at").and_then(|x| x.as_str()).map(|s| s.to_string());
        }
    }
    Ok(ZedProfile { user_id: uid, github_login, name, org_id, org_name, plan })
}

// ─── LLM token (cache 15p theo account) ──────────────────────────────────────

#[derive(Debug, Default)]
pub struct ZedTokenCache {
    inner: Mutex<HashMap<String, (Instant, String)>>,
}

impl ZedTokenCache {
    pub fn get_or_fetch(
        &self,
        account_key: &str,
        id: &str,
        token: &str,
        org_id: Option<&str>,
        system_id: Option<&str>,
        force: bool,
    ) -> Result<String, String> {
        if !force {
            if let Ok(guard) = self.inner.lock() {
                if let Some((at, tok)) = guard.get(account_key) {
                    if at.elapsed() < LLM_TOKEN_TTL {
                        return Ok(tok.clone());
                    }
                }
            }
        }
        let fresh = fetch_llm_token(id, token, org_id, system_id)?;
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(account_key.to_string(), (Instant::now(), fresh.clone()));
        }
        Ok(fresh)
    }

    pub fn invalidate(&self, account_key: &str) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.remove(account_key);
        }
    }
}

fn fetch_llm_token(
    id: &str,
    token: &str,
    org_id: Option<&str>,
    system_id: Option<&str>,
) -> Result<String, String> {
    let url = format!("{ZED_CLOUD_BASE_URL}/client/llm_tokens");
    let mut req = agent(request_timeout())
        .post(&url)
        .set("Authorization", &auth_header(id, token))
        .set("Content-Type", "application/json");
    if let Some(sid) = system_id.map(str::trim).filter(|s| {
        !s.is_empty() && s.len() <= 256 && s.chars().all(|c| c.is_ascii_alphanumeric() || "._:-".contains(c))
    }) {
        req = req.set("x-zed-system-id", sid);
    }
    let mut body = serde_json::Map::new();
    if let Some(org) = org_id.map(str::trim).filter(|s| !s.is_empty()) {
        body.insert("organization_id".to_string(), serde_json::Value::String(org.to_string()));
    }
    let resp = req
        .send_bytes(serde_json::to_vec(&body).unwrap_or_default().as_slice())
        .map_err(|e| http_error("zed llm_tokens", e))?;
    if resp.status() != 200 {
        return Err(format!("zed llm_tokens: upstream HTTP {}", resp.status()));
    }
    let mut raw = String::new();
    resp.into_reader()
        .take(16 * 1024)
        .read_to_string(&mut raw)
        .map_err(|e| format!("zed llm_tokens read: {e}"))?;
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("token")?.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "zed llm_tokens: missing token".to_string())
}

// ─── Models ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZedModelInfo {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    /// anthropic | open_ai | google | x_ai (unknown → open_ai-chat shape).
    pub provider: String,
    #[serde(default)]
    pub supports_tools: bool,
    #[serde(default)]
    pub max_output_tokens: Option<i64>,
}

/// Map cache id → info, share giữa AppState (routing) và GatewayState (IPC refresh).
pub type ZedModelMap = std::collections::HashMap<String, ZedModelInfo>;

fn parse_models(raw: &str) -> Result<Vec<ZedModelInfo>, String> {
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "zed models: invalid JSON".to_string())?;
    let arr = v.get("models").and_then(|m| m.as_array()).ok_or("zed models: missing models[]")?;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for m in arr {
        let id = m.get("id").and_then(|s| s.as_str()).unwrap_or("").trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        out.push(ZedModelInfo {
            id: id.to_string(),
            display_name: m.get("display_name").and_then(|s| s.as_str()).map(|s| s.to_string()),
            provider: m.get("provider").and_then(|s| s.as_str()).unwrap_or("open_ai").to_string(),
            supports_tools: m.get("supports_tools").and_then(|b| b.as_bool()).unwrap_or(false),
            max_output_tokens: m.get("max_output_tokens").and_then(|n| n.as_i64()),
        });
    }
    Ok(out)
}

/// `GET /models` + `Bearer {llmToken}`. Tự refresh llm token 1 lần khi gặp
/// header `x-zed-expired-token` / `x-zed-outdated-token` (mirror TS).
pub fn list_models(
    cache: &ZedTokenCache,
    account_key: &str,
    id: &str,
    token: &str,
    org_id: Option<&str>,
    system_id: Option<&str>,
) -> Result<Vec<ZedModelInfo>, String> {
    let mut llm = cache.get_or_fetch(account_key, id, token, org_id, system_id, false)?;
    let mut refreshed = false;
    loop {
        let resp = agent(request_timeout())
            .get(&format!("{ZED_CLOUD_BASE_URL}/models"))
            .set("Authorization", &format!("Bearer {llm}"))
            .set("x-zed-client-supports-x-ai", "true")
            .call();
        let resp = match resp {
            Ok(r) => r,
            Err(e) => return Err(http_error("zed models", e)),
        };
        let refreshable = resp
            .headers_names()
            .iter()
            .any(|h| {
                let l = h.to_lowercase();
                l == "x-zed-expired-token" || l == "x-zed-outdated-token"
            });
        let status = resp.status();
        let mut raw = String::new();
        let _ = resp.into_reader().take(256 * 1024).read_to_string(&mut raw);
        if status == 200 {
            return parse_models(&raw);
        }
        if !refreshed && refreshable {
            cache.invalidate(account_key);
            llm = cache.get_or_fetch(account_key, id, token, org_id, system_id, true)?;
            refreshed = true;
            continue;
        }
        let short: String = raw.chars().take(200).collect();
        return Err(format!("zed models: upstream HTTP {status} {short}"));
    }
}

// ─── Chat ────────────────────────────────────────────────────────────────────
// Scope v1: text messages. Tools/images → lỗi rõ ràng (không corrupt ngầm).
// Upstream trả NDJSON (không phải SSE `data:`) — đọc hết rồi parse theo
// `model.provider`, emit lại SSE OpenAI/Anthropic ở tầng gateway.

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(90);

pub struct ZedChatUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

pub struct ZedChatResult {
    pub text: String,
    pub usage: ZedChatUsage,
    pub stop_reason: String,
}

/// Trích text từ OpenAI message content. Part không phải text → Err rõ ràng.
fn extract_text(content: &serde_json::Value) -> Result<String, String> {
    if let Some(s) = content.as_str() {
        return Ok(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        let mut out = String::new();
        for part in arr {
            let t = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if t == "text" {
                if let Some(s) = part.get("text").and_then(|s| s.as_str()) {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(s);
                }
            } else if t == "image_url" || t == "image" || t == "input_audio" {
                return Err("zed native v1 supports text messages only (image/audio)".into());
            } else if t == "tool_call" || t == "tool_calls" || t == "function_call" {
                return Err("zed native v1 supports text messages only (tools)".into());
            }
            // Unknown part khác: bỏ qua (forward-compatible).
        }
        return Ok(out);
    }
    Ok(String::new())
}

fn build_provider_request(
    provider: &str,
    model_id: &str,
    messages: &[(String, String)],
    max_tokens: Option<i64>,
    default_max: Option<i64>,
) -> serde_json::Value {
    match provider {
        "anthropic" => {
            let msgs: Vec<serde_json::Value> = messages
                .iter()
                .map(|(role, text)| {
                    let r = if role == "assistant" { "assistant" } else { "user" };
                    serde_json::json!({"role": r, "content": [{"type": "text", "text": text}]})
                })
                .collect();
            serde_json::json!({
                "model": model_id,
                "messages": msgs,
                "max_tokens": max_tokens.or(default_max).unwrap_or(4096),
            })
        }
        "google" => {
            let contents: Vec<serde_json::Value> = messages
                .iter()
                .map(|(role, text)| {
                    let r = if role == "assistant" { "model" } else { "user" };
                    serde_json::json!({"role": r, "parts": [{"text": text}]})
                })
                .collect();
            let mut gen = serde_json::json!({"candidate_count": 1, "temperature": 1});
            if let Some(m) = max_tokens.or(default_max) {
                gen["max_output_tokens"] = serde_json::Value::from(m);
            }
            serde_json::json!({
                "model": {"model_id": model_id},
                "contents": contents,
                "generation_config": gen,
            })
        }
        "open_ai" => {
            // Responses API shape.
            let input: Vec<serde_json::Value> = messages
                .iter()
                .map(|(role, text)| {
                    serde_json::json!({"type": "message", "role": role, "content": text})
                })
                .collect();
            let mut body = serde_json::json!({
                "model": model_id,
                "input": input,
                "stream": true,
                "store": false,
            });
            if let Some(m) = max_tokens.or(default_max) {
                body["max_output_tokens"] = serde_json::Value::from(m);
            }
            body
        }
        _ => {
            // x_ai + unknown → OpenAI chat shape.
            let msgs: Vec<serde_json::Value> = messages
                .iter()
                .map(|(role, text)| serde_json::json!({"role": role, "content": text}))
                .collect();
            let mut body = serde_json::json!({"model": model_id, "stream": true, "messages": msgs});
            if let Some(m) = max_tokens.or(default_max) {
                body["max_tokens"] = serde_json::Value::from(m);
            }
            body
        }
    }
}

fn get_i64(v: &serde_json::Value, keys: &[&str]) -> i64 {
    for k in keys {
        if let Some(n) = v.get(*k).and_then(|n| n.as_i64()) {
            return n;
        }
    }
    0
}

/// Parse NDJSON completion theo provider → (text, usage, stop_reason).
fn parse_completion(provider: &str, text: &str) -> ZedChatResult {
    let mut out_text = String::new();
    let mut input_tokens = 0i64;
    let mut output_tokens = 0i64;
    let mut stop = "end_turn".to_string();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Status envelope: {"status": {"failed": {...}}} → lỗi upstream.
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(failed) = v.get("status").and_then(|s| s.get("failed")) {
                let msg = failed.get("message").and_then(|m| m.as_str()).unwrap_or("zed completion failed");
                return ZedChatResult {
                    text: String::new(),
                    usage: ZedChatUsage { input_tokens, output_tokens },
                    stop_reason: format!("error: {msg}"),
                };
            }
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        match provider {
            "anthropic" => {
                let t = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match t {
                    "message_start" => {
                        if let Some(u) = v.get("message").and_then(|m| m.get("usage")) {
                            input_tokens = get_i64(u, &["input_tokens"]);
                            output_tokens = get_i64(u, &["output_tokens"]);
                        }
                    }
                    "content_block_delta" => {
                        if let Some(d) = v.get("delta") {
                            if d.get("type").and_then(|t| t.as_str()) == Some("text_delta") {
                                if let Some(s) = d.get("text").and_then(|s| s.as_str()) {
                                    out_text.push_str(s);
                                }
                            }
                        }
                    }
                    "message_delta" => {
                        if v.get("delta").and_then(|d| d.get("stop_reason")).and_then(|r| r.as_str()) == Some("max_tokens") {
                            stop = "max_tokens".to_string();
                        }
                        if let Some(u) = v.get("usage") {
                            let o = get_i64(u, &["output_tokens"]);
                            if o > 0 {
                                output_tokens = o;
                            }
                        }
                    }
                    _ => {}
                }
            }
            "google" => {
                if let Some(u) = v.get("usageMetadata").or_else(|| v.get("usage_metadata")) {
                    let i = get_i64(u, &["promptTokenCount", "prompt_token_count"]);
                    let o = get_i64(u, &["candidatesTokenCount", "candidates_token_count"]);
                    if i > 0 {
                        input_tokens = i;
                    }
                    if o > 0 {
                        output_tokens = o;
                    }
                }
                if let Some(cands) = v.get("candidates").and_then(|c| c.as_array()) {
                    for c in cands {
                        let r = c.get("finishReason").and_then(|r| r.as_str())
                            .or_else(|| c.get("finish_reason").and_then(|r| r.as_str()))
                            .unwrap_or("");
                        if r == "MAX_TOKENS" {
                            stop = "max_tokens".to_string();
                        }
                        if let Some(parts) = c.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
                            for p in parts {
                                if let Some(s) = p.get("text").and_then(|s| s.as_str()) {
                                    out_text.push_str(s);
                                }
                            }
                        }
                    }
                }
            }
            "open_ai" => {
                if v.get("type").and_then(|t| t.as_str()) == Some("response.output_text.delta") {
                    if let Some(s) = v.get("delta").and_then(|s| s.as_str()) {
                        out_text.push_str(s);
                    }
                }
                if let Some(u) = v.get("response").and_then(|r| r.get("usage")) {
                    let i = get_i64(u, &["input_tokens"]);
                    let o = get_i64(u, &["output_tokens"]);
                    if i > 0 {
                        input_tokens = i;
                    }
                    if o > 0 {
                        output_tokens = o;
                    }
                }
                if v.get("type").and_then(|t| t.as_str()) == Some("response.completed") {
                    if let Some(r) = v.get("response").and_then(|r| r.get("status_details")).and_then(|s| s.get("reason")).and_then(|r| r.as_str()) {
                        if r == "max_output_tokens" {
                            stop = "max_tokens".to_string();
                        }
                    }
                }
            }
            _ => {
                if let Some(u) = v.get("usage") {
                    let i = get_i64(u, &["prompt_tokens"]);
                    let o = get_i64(u, &["completion_tokens"]);
                    if i > 0 {
                        input_tokens = i;
                    }
                    if o > 0 {
                        output_tokens = o;
                    }
                }
                if let Some(delta) = v.get("choices").and_then(|c| c.as_array()).and_then(|a| a.first()).and_then(|c| c.get("delta")) {
                    if let Some(s) = delta.get("content").and_then(|s| s.as_str()) {
                        out_text.push_str(s);
                    }
                }
                if let Some(fr) = v.get("choices").and_then(|c| c.as_array()).and_then(|a| a.first()).and_then(|c| c.get("finish_reason")).and_then(|f| f.as_str()) {
                    if fr == "length" {
                        stop = "max_tokens".to_string();
                    } else if fr == "stop" {
                        stop = "end_turn".to_string();
                    }
                }
            }
        }
    }
    ZedChatResult { text: out_text, usage: ZedChatUsage { input_tokens, output_tokens }, stop_reason: stop }
}

/// Một completion Zed đầy đủ: llm token → POST /completions → parse.
/// Tự refresh llm token 1 lần khi gặp header expired/outdated (mirror TS).
#[allow(clippy::too_many_arguments)]
pub fn chat_once(
    cache: &ZedTokenCache,
    account_key: &str,
    id: &str,
    token: &str,
    org_id: Option<&str>,
    system_id: Option<&str>,
    model: &ZedModelInfo,
    messages: &[(String, String)],
    max_tokens: Option<i64>,
) -> Result<ZedChatResult, String> {
    let mut llm = cache.get_or_fetch(account_key, id, token, org_id, system_id, false)?;
    let mut refreshed = false;
    loop {
        let body = serde_json::json!({
            "provider": model.provider,
            "model": model.id,
            "provider_request": build_provider_request(&model.provider, &model.id, messages, max_tokens, model.max_output_tokens),
        });
        let resp = agent(COMPLETION_TIMEOUT)
            .post(&format!("{ZED_CLOUD_BASE_URL}/completions"))
            .set("Authorization", &format!("Bearer {llm}"))
            .set("Content-Type", "application/json")
            .set("x-zed-client-supports-status-messages", "true")
            .set("x-zed-client-supports-stream-ended-request-completion-status", "true")
            .send_bytes(serde_json::to_vec(&body).unwrap_or_default().as_slice());
        let resp = match resp {
            Ok(r) => r,
            Err(e) => return Err(http_error("zed completions", e)),
        };
        let refreshable = resp
            .headers_names()
            .iter()
            .any(|h| {
                let l = h.to_lowercase();
                l == "x-zed-expired-token" || l == "x-zed-outdated-token"
            });
        let status = resp.status();
        let mut raw = String::new();
        let _ = resp.into_reader().take(4 * 1024 * 1024).read_to_string(&mut raw);
        if status == 200 {
            let parsed = parse_completion(&model.provider, &raw);
            if parsed.stop_reason.starts_with("error: ") {
                return Err(parsed.stop_reason);
            }
            return Ok(parsed);
        }
        if !refreshed && refreshable {
            cache.invalidate(account_key);
            llm = cache.get_or_fetch(account_key, id, token, org_id, system_id, true)?;
            refreshed = true;
            continue;
        }
        let short: String = raw.chars().take(300).collect();
        return Err(format!("zed completions: upstream HTTP {status} {short}"));
    }
}

// ─── Routing (gateway glue dùng, code ở external.rs) ─────────────────────────

/// Bỏ tiền tố tường minh `zed/` hoặc `zed:` (không phân biệt hoa thường).
pub fn strip_prefix(model: &str) -> &str {
    let t = model.trim();
    if t.len() > 4 {
        let (head, tail) = t.split_at(4);
        if head.eq_ignore_ascii_case("zed/") || head.eq_ignore_ascii_case("zed:") {
            return tail.trim();
        }
    }
    t
}

/// Credential + model đã resolve cho 1 request chat native.
pub struct ZedRoute {
    pub account_id: String,
    pub access_token: String,
    pub org_id: String,
    pub model: ZedModelInfo,
}

/// Resolve model → native Zed route nếu model (sau khi bỏ prefix tường minh) nằm
/// trong cache models đã refresh VÀ còn ≥1 account enabled. None = đi đường khác.
/// Cache rỗng → None (UI hướng dẫn Refresh models trước, như discovery CodeBuddy).
pub fn resolve_native(
    state: &crate::core::server::SharedState,
    model: &str,
) -> Option<ZedRoute> {
    let want = strip_prefix(model);
    if want.is_empty() {
        return None;
    }
    let found = state.zed_models.read().get(want).cloned()?;
    // AppState chỉ giữ Arc<Storage>; API storage tự lock bên trong nên an toàn
    // gọi từ spawn_blocking (không giữ lock nào của AppState lúc này).
    let store = state.storage.as_ref()?;
    let mut accounts = store
        .list_zed_accounts()
        .ok()?
        .into_iter()
        .filter(|a| a.enabled)
        .collect::<Vec<_>>();
    accounts.sort_by(|a, b| a.id.cmp(&b.id));
    let a = accounts.into_iter().next()?;
    Some(ZedRoute {
        account_id: a.id.clone(),
        access_token: a.access_token.clone(),
        org_id: a.org_id.clone(),
        model: found,
    })
}

pub fn system_id_env() -> Option<String> {
    std::env::var("ANTI_API_ZED_SYSTEM_ID").ok().filter(|s| !s.trim().is_empty())
}

/// System ID hiệu dụng: config `zed.system_id` trước, env sau, trống = không gửi.
pub fn resolve_system_id(configured: &str) -> Option<String> {
    let t = configured.trim();
    if !t.is_empty() {
        return Some(t.to_string());
    }
    system_id_env()
}

/// Key cache token theo account — DÙNG CHUNG mọi nơi (IPC + gateway) để
/// không fetch trùng. Đổi format là invalidate toàn bộ cache cũ (an toàn).
pub fn account_key(id: &str) -> String {
    format!("{ZED_SERVER_URL}:{id}")
}

/// Tách OpenAI chat body → ([(role, text)], max_tokens?). Lỗi rõ nếu có
/// content phi-text (tools/images chưa hỗ trợ ở native v1).
pub fn openai_request_parts(body: &[u8]) -> Result<(Vec<(String, String)>, Option<i64>), String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("invalid chat JSON: {e}"))?;
    let msgs = v.get("messages").and_then(|m| m.as_array()).ok_or("missing messages[]")?;
    if msgs.is_empty() {
        return Err("empty messages[]".to_string());
    }
    let mut out = Vec::with_capacity(msgs.len());
    for m in msgs {
        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string();
        let content = m.get("content").cloned().unwrap_or(serde_json::Value::String(String::new()));
        out.push((role, extract_text(&content)?));
    }
    let max_tokens = v.get("max_tokens").and_then(|n| n.as_i64()).filter(|n| *n > 0);
    Ok((out, max_tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_credential_ok() {
        let c = parse_credential_json(r#"{"type":"zed","id":"u-1","access_token":"tok","extra":1}"#).unwrap();
        assert_eq!(c.id, "u-1");
        assert_eq!(c.access_token, "tok");
    }

    #[test]
    fn parse_credential_rejects() {
        for bad in [
            r#"{"type":"github","id":"u","access_token":"t"}"#,
            r#"{"type":"zed","id":"","access_token":"t"}"#,
            r#"{"type":"zed","id":"u","access_token":""}"#,
            r#"[1,2]"#,
            "not json",
            "",
        ] {
            assert!(parse_credential_json(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parse_models_dedup() {
        let raw = r#"{"models":[{"id":"m1","provider":"anthropic"},{"id":"m1"},{"id":" ","provider":"x"}]}"#;
        let out = parse_models(raw).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].provider, "anthropic");
    }

    #[test]
    fn parse_profile_shape() {
        let raw = r#"{"user":{"id":42,"github_login":"octo","name":"O"},"organizations":[{"id":"o1","name":"Org"}],"plan":{"plan_v3":"pro","usage":{"edit_predictions":{"used":3,"limit":100}},"subscription_period":{"started_at":"2026-01-01","ended_at":"2026-02-01"}}}"#;
        let p = parse_profile(raw).unwrap();
        assert_eq!(p.user_id, "42");
        assert_eq!(p.github_login, "octo");
        assert_eq!(p.org_id.as_deref(), Some("o1"));
        assert_eq!(p.plan.edit_used, Some(3));
        assert_eq!(p.plan.period_end.as_deref(), Some("2026-02-01"));
    }

    #[test]
    fn extract_text_shapes() {
        let s = serde_json::json!("hello");
        assert_eq!(extract_text(&s).unwrap(), "hello");
        let arr = serde_json::json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]);
        assert_eq!(extract_text(&arr).unwrap(), "a\nb");
        let tool = serde_json::json!([{"type": "text", "text": "x"}, {"type": "tool_call"}]);
        assert!(extract_text(&tool).is_err());
        let img = serde_json::json!([{"type": "image_url"}]);
        assert!(extract_text(&img).is_err());
    }

    #[test]
    fn parse_completion_openai_chat() {
        let raw = "{\"choices\":[{\"delta\":{\"content\":\"Hi\"}}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":1}}\n{\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}";
        let r = parse_completion("x_ai", raw);
        assert_eq!(r.text, "Hi");
        assert_eq!(r.usage.input_tokens, 3);
        assert_eq!(r.usage.output_tokens, 1);
        assert_eq!(r.stop_reason, "end_turn");
    }

    #[test]
    fn parse_completion_anthropic() {
        let raw = "{\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Yo\"}}\n{\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"},\"usage\":{\"output_tokens\":9}}";
        let r = parse_completion("anthropic", raw);
        assert_eq!(r.text, "Yo");
        assert_eq!(r.usage.input_tokens, 5);
        assert_eq!(r.usage.output_tokens, 9);
        assert_eq!(r.stop_reason, "max_tokens");
    }

    #[test]
    fn parse_completion_status_failed() {
        let raw = "{\"status\":{\"failed\":{\"message\":\"quota gone\"}}}";
        let r = parse_completion("google", raw);
        assert!(r.stop_reason.starts_with("error: "));
    }

    #[test]
    fn build_provider_request_shapes() {        let msgs = vec![("user".to_string(), "hi".to_string())];
        let a = build_provider_request("anthropic", "m", &msgs, None, None);
        assert_eq!(a["max_tokens"], 4096);
        assert_eq!(a["messages"][0]["role"], "user");
        let g = build_provider_request("google", "m", &msgs, Some(7), None);
        assert_eq!(g["contents"][0]["role"], "user");
        assert_eq!(g["generation_config"]["max_output_tokens"], 7);
        let o = build_provider_request("open_ai", "m", &msgs, None, None);
        assert_eq!(o["input"][0]["type"], "message");
        let x = build_provider_request("x_ai", "m", &msgs, None, None);
        assert_eq!(x["messages"][0]["content"], "hi");
    }
}

// ─── Gateway glue (SSE envelopes + audit, gọi từ server.rs dispatch) ─────────
// Code từng nằm ở external.rs — chuyển về đây để khối Zed tách hoàn toàn.

fn sse_error_event(msg: &str) -> Event {
    Event::default().data(format!("{{\"error\":\"{msg}\"}}"))
}

/// Response lỗi nhanh khi native route không thực thi được (không account,
/// model chưa refresh...). Envelope SSE như mọi forward khác.
pub fn unavailable_response(msg: &str) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(1);
    let text = msg.chars().take(300).collect::<String>();
    tokio::task::spawn_blocking(move || {
        let _ = tx.blocking_send(Ok(sse_error_event(&text)));
    });
    let stream_resp = ReceiverStream::new(rx);
    Sse::new(stream_resp).into_response()
}

/// Forward chat sang Zed native: OpenAI in → provider_request → NDJSON parse →
/// SSE OpenAI/Anthropic out. v1 text-only (tools/images → lỗi rõ ràng).
/// `system_id`: đã resolve (config trước, env sau) — None = không gửi header.
#[allow(clippy::too_many_arguments)]
pub fn forward_stream(
    state: SharedState,
    route: ZedRoute,
    system_id: Option<String>,
    key_prefix: String,
    route_path: &'static str,
    model: String,
    upstream_body: Vec<u8>,
    raw_request_headers: Vec<(String, String)>,
    raw_request_body: String,
    stream: bool,
    is_anthropic: bool,
) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(64);
    let metrics = state.metrics.clone();
    let audit = state.audit.clone();
    let storage = state.storage.clone();
    let zed_tokens = state.zed_tokens.clone();

    tokio::task::spawn_blocking(move || {
        let relay_start = Instant::now();
        let mut total_tokens: u64 = 0;
        let mut final_status: u16 = 200;
        let mut err_msg = String::new();
        let mut resp_preview = String::new();
        const PREVIEW_LIMIT: usize = 4096;

        let acct_tag = format!("zed:{}", route.account_id);
        let sys = system_id;
        let key = account_key(&route.account_id);
        let org = if route.org_id.trim().is_empty() { None } else { Some(route.org_id.as_str()) };

        let outcome: Result<(String, i64, i64, String), String> = (|| {
            let (messages, max_tokens) = openai_request_parts(&upstream_body)?;
            let r = chat_once(
                &zed_tokens, &key, &route.account_id, &route.access_token,
                org, sys.as_deref(), &route.model, &messages, max_tokens,
            )?;
            Ok((r.text, r.usage.input_tokens, r.usage.output_tokens, r.stop_reason))
        })();

        match outcome {
            Ok((text, in_tok, out_tok, stop)) => {
                total_tokens = (in_tok.max(0) + out_tok.max(0)) as u64;
                if resp_preview.len() < PREVIEW_LIMIT {
                    resp_preview.push_str(&text.chars().take(PREVIEW_LIMIT).collect::<String>());
                }
                let created = chrono::Utc::now().timestamp();
                let msg_id = format!("msg_{}", relay_start.elapsed().as_nanos());
                if is_anthropic {
                    let _ = tx.blocking_send(Ok(Event::default().event("message_start").data(anthropic_message_start(&msg_id, &model))));
                    let _ = tx.blocking_send(Ok(Event::default().event("content_block_start").data(anthropic_content_block_start(0))));
                    if !text.is_empty() {
                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_delta").data(anthropic_content_block_delta(0, &text))));
                    }
                    let _ = tx.blocking_send(Ok(Event::default().event("content_block_stop").data(anthropic_content_block_stop(0))));
                    let _ = tx.blocking_send(Ok(Event::default().event("message_delta").data(anthropic_message_delta(total_tokens))));
                    let _ = tx.blocking_send(Ok(Event::default().event("message_stop").data(anthropic_message_stop())));
                } else if stream {
                    let chunk = serde_json::json!({
                        "id": msg_id, "object": "chat.completion.chunk", "created": created, "model": model,
                        "choices": [{"index": 0, "delta": {"role": "assistant", "content": text}, "finish_reason": null}],
                    });
                    let _ = tx.blocking_send(Ok(Event::default().data(chunk.to_string())));
                    let done = serde_json::json!({
                        "id": msg_id, "object": "chat.completion.chunk", "created": created, "model": model,
                        "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                    });
                    let _ = tx.blocking_send(Ok(Event::default().data(done.to_string())));
                    let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
                    let _ = stop;
                } else {
                    let full = serde_json::json!({
                        "id": msg_id, "object": "chat.completion", "created": created, "model": model,
                        "choices": [{"index": 0, "message": {"role": "assistant", "content": text}, "finish_reason": stop}],
                        "usage": {"prompt_tokens": in_tok, "completion_tokens": out_tok, "total_tokens": in_tok + out_tok},
                    });
                    let _ = tx.blocking_send(Ok(Event::default().data(full.to_string())));
                }
            }
            Err(e) => {
                // Map mã HTTP trong message (nếu có) để client thấy mã thật.
                final_status = parse_zed_http_status(&e).unwrap_or(502);
                err_msg = e.chars().take(500).collect();
                let short: String = err_msg.chars().take(200).collect();
                let _ = tx.blocking_send(Ok(sse_error_event(&short.replace('"', "'"))));
            }
        }

        let elapsed = relay_start.elapsed();
        info!(
            "[req] route={} key={} acct={} model={} status={} tokens={} elapsed={}ms err={}",
            route_path, key_prefix, acct_tag, model, final_status, total_tokens,
            elapsed.as_millis(), err_msg
        );
        metrics.trace_detail(
            route_path,
            &model,
            final_status,
            total_tokens,
            elapsed.as_millis() as u64,
            Some(json!({
                "req_bytes": upstream_body.len(),
                "req_preview": String::from_utf8_lossy(&upstream_body).chars().take(1000).collect::<String>(),
                "resp_preview": resp_preview.chars().take(2000).collect::<String>(),
                "error": if err_msg.is_empty() { Value::Null } else { Value::String(err_msg.clone()) },
                "via": "zed-native",
            })),
        );
        if let Some(s) = &storage {
            let _ = s.log_usage(&key_prefix, route_path, &model, final_status, total_tokens, elapsed.as_millis() as u64);
        }
        audit.push(crate::core::audit::TrafficAuditLog {
            id: format!("zed_{}", relay_start.elapsed().as_nanos()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            route: route_path.to_string(),
            model: model.clone(),
            status_code: final_status,
            duration_ms: elapsed.as_millis() as u64,
            account_uid: acct_tag,
            proxy_used: None,
            raw_request_headers,
            raw_forwarded_headers: Vec::new(),
            raw_request_body,
            raw_response_preview: resp_preview,
            raw_response_headers: Vec::new(),
            raw_forwarded_body: String::new(),
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

/// Trích mã HTTP từ message lỗi zed ("...upstream HTTP 429...") để client thấy mã thật.
fn parse_zed_http_status(msg: &str) -> Option<u16> {
    msg.split("HTTP ").nth(1)?.split_whitespace().next()?.parse::<u16>().ok()
}
