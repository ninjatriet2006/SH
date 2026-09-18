//! Browser device-login flow — ported from Go `cmd/login/main.go` + `login.sh`.
//!
//! Đây là tính năng bị rớt khi port Go → Rust: bản Go đăng nhập qua browser
//! (device flow của plugin), còn GUI Rust chỉ còn nhập file auth.
//!
//! Luồng (không PKCE — state do máy chủ cấp):
//! 1. `login_start(realm)` → POST `{api}/v2/plugin/auth/state?platform=CLI`
//!    lấy `{state, authUrl}`. User mở URL trong browser, đăng nhập.
//! 2. `login_poll_once(realm, state)` → GET `{api}/v2/plugin/auth/token?state=`:
//!    pending khi code != 0 ("login ing"), xong khi code == 0 + token bundle.
//! 3. GET `{api}/v2/plugin/login/account?state=` kèm Bearer lấy uid/nickname
//!    (intl fallback giải mã JWT — KHÔNG verify chữ ký, chỉ đọc claims).
//!
//! Realm: `cn` (copilot.tencent.com, WeChat/điện thoại) | `intl`
//! (www.codebuddy.ai, Google, thêm nonce + header trace B3).

use std::time::Duration;

use rand::RngCore;
use serde::Deserialize;

use crate::core::upstream::headers::CODEBUDDY_CLI_VERSION;
use crate::core::upstream::realms;

fn login_ua() -> String {
    format!("CLI/{CODEBUDDY_CLI_VERSION} CodeBuddy/{CODEBUDDY_CLI_VERSION}")
}
const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);

fn api_base(realm: &str) -> &'static str {
    if realm == realms::REALM_INTL {
        "https://www.codebuddy.ai"
    } else {
        "https://copilot.tencent.com"
    }
}

/// Resolve realm: đối số → mặc định "cn". Lạ → Err (fail fast như bản Go).
pub fn resolve_realm(arg: &str) -> Result<&'static str, String> {
    let (realm, ok) = realms::normalize_realm(arg);
    if !ok {
        return Err(format!("unknown realm {arg:?} (want cn|intl)"));
    }
    Ok(realm)
}

fn new_trace_id() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn host_of(base: &str) -> &str {
    base.split_once("://").map(|(_, rest)| rest).unwrap_or(base)
}

fn agent(proxy_url: &str) -> ureq::Agent {
    let proxy = (!proxy_url.trim().is_empty())
        .then(|| ureq::Proxy::new(proxy_url).ok())
        .flatten();
    let mut b = ureq::AgentBuilder::new().timeout(LOGIN_TIMEOUT);
    if let Some(p) = proxy {
        b = b.proxy(p);
    }
    b.build()
}

fn apply_headers(req: ureq::Request, realm: &str, api_base: &str) -> ureq::Request {
    let mut req = req
        .set("Content-Type", "application/json")
        .set("Accept", "application/json, text/plain, */*")
        .set("X-Requested-With", "XMLHttpRequest")
        .set("User-Agent", &login_ua());
    if realm != realms::REALM_INTL {
        // CN giữ nguyên hiện trạng: Origin/Referer codebuddy.cn.
        req = req
            .set("Origin", "https://www.codebuddy.cn")
            .set("Referer", "https://www.codebuddy.cn/");
        return req;
    }
    // Intl theo đối chiếu TS buildTraceHeaders: không Origin/Referer.
    let req_id = new_trace_id();
    let span_id: String = new_trace_id().chars().take(16).collect();
    req.set("Cache-Control", "no-cache")
        .set("Pragma", "no-cache")
        .set("X-Domain", host_of(api_base))
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

/// Lỗi API phân loại rõ để poll quyết định Pending vs lỗi thật.
enum ApiError {
    /// Lỗi mạng/timeout — lỗi thật.
    Transport(String),
    /// HTTP status != 2xx.
    Http(u16),
    /// Envelope code != 0 (vd "login ing" = đang chờ user).
    Api(i64, String),
    Parse(String),
}

fn envelope_from(result: Result<ureq::Response, ureq::Error>) -> Result<serde_json::Value, ApiError> {
    let resp = result.map_err(|e| match e {
        ureq::Error::Status(code, _) => ApiError::Http(code),
        other => ApiError::Transport(other.to_string()),
    })?;
    if resp.status() >= 300 {
        return Err(ApiError::Http(resp.status()));
    }
    let env: Envelope = resp.into_json().map_err(|e| ApiError::Parse(e.to_string()))?;
    if env.code != 0 {
        return Err(ApiError::Api(env.code, env.msg));
    }
    Ok(env.data)
}

fn api_error_string(e: ApiError) -> String {
    match e {
        ApiError::Transport(s) => format!("request failed: {s}"),
        ApiError::Http(c) => format!("http_error: upstream {c}"),
        ApiError::Api(c, m) => format!("code={c} msg={m}"),
        ApiError::Parse(s) => format!("parse failed: {s}"),
    }
}

/// Kết quả `login_start`: URL mở browser + state để poll.
pub struct LoginStart {
    pub auth_url: String,
    pub state: String,
    pub realm: String,
}

pub fn login_url(realm_arg: &str, proxy_url: &str) -> Result<LoginStart, String> {
    let realm = resolve_realm(realm_arg)?;
    let base = api_base(realm);
    let ag = agent(proxy_url);
    let (endpoint, body) = if realm == realms::REALM_INTL {
        let nonce = new_trace_id();
        (
            format!("{base}/v2/plugin/auth/state?platform=CLI&nonce={nonce}"),
            format!(r#"{{"nonce":"{nonce}"}}"#),
        )
    } else {
        (format!("{base}/v2/plugin/auth/state?platform=CLI"), "{}".to_string())
    };
    let data = envelope_from(
        apply_headers(ag.post(&endpoint), realm, base).send_bytes(body.as_bytes()),
    )
    .map_err(api_error_string)?;
    let state = data.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let auth_url = data.get("authUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if state.is_empty() || auth_url.is_empty() {
        return Err("auth state: missing state or authUrl".into());
    }
    Ok(LoginStart { auth_url, state, realm: realm.to_string() })
}

/// Token bundle khi poll xong.
#[derive(Debug, Clone)]
pub struct TokenBundle {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub domain: String,
}

pub enum PollOutcome {
    /// User chưa xong trong browser — poll lại sau.
    Pending,
    Done(TokenBundle),
}

pub fn login_poll_once(realm_arg: &str, state: &str, proxy_url: &str) -> Result<PollOutcome, String> {
    let realm = resolve_realm(realm_arg)?;
    let base = api_base(realm);
    let ag = agent(proxy_url);
    let endpoint = format!("{base}/v2/plugin/auth/token?state={state}");
    let data = match envelope_from(apply_headers(ag.get(&endpoint), realm, base).call()) {
        Ok(d) => d,
        Err(e) => match e {
            // Transport / 5xx / parse = lỗi thật (bản Go fatal "token endpoint error").
            ApiError::Transport(_) | ApiError::Parse(_) => {
                return Err(format!("token endpoint error: {}", api_error_string(e)));
            }
            ApiError::Http(code) if code == 0 || code >= 500 => {
                return Err(format!("token endpoint error: {}", api_error_string(e)));
            }
            // 4xx hoặc code nghiệp vụ ("login ing") = user chưa xong → Pending.
            _ => return Ok(PollOutcome::Pending),
        },
    };
    let access = data.get("accessToken").and_then(|v| v.as_str()).unwrap_or("");
    if access.is_empty() {
        return Ok(PollOutcome::Pending);
    }
    Ok(PollOutcome::Done(TokenBundle {
        access_token: access.to_string(),
        refresh_token: data.get("refreshToken").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        expires_in: data.get("expiresIn").and_then(|v| v.as_i64()).unwrap_or(0),
        domain: data.get("domain").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }))
}

#[derive(Debug, Clone, Default)]
pub struct LoginAccount {
    pub uid: String,
    pub enterprise_id: String,
    pub nickname: String,
}

/// Lấy uid/nickname (best-effort) + fallback JWT cho intl. Không bao giờ Err.
pub fn fetch_login_account(realm_arg: &str, state: &str, access_token: &str, proxy_url: &str) -> LoginAccount {
    let mut acct = LoginAccount::default();
    if let Ok(realm) = resolve_realm(realm_arg) {
        let base = api_base(realm);
        let ag = agent(proxy_url);
        let endpoint = format!("{base}/v2/plugin/login/account?state={state}");
        let req = apply_headers(ag.get(&endpoint), realm, base)
            .set("Authorization", &format!("Bearer {access_token}"));
        if let Ok(data) = envelope_from(req.call()) {
            acct.uid = data.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string();
            acct.enterprise_id = data.get("enterpriseId").and_then(|v| v.as_str()).unwrap_or("").to_string();
            acct.nickname = data.get("nickname").and_then(|v| v.as_str()).unwrap_or("").to_string();
        }
    }
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

/// Domain hiệu dụng cho account vừa login: nếu máy chủ không trả domain mà
/// realm là intl thì ép `codebuddy.ai` để routing (chat/billing) không rơi về
/// CN theo default. Token intl bắn sang endpoint CN chắc chắn 401.
/// Realm CN giữ nguyên rỗng (default đã là CN, tránh đổi hành vi đang chạy).
pub fn resolve_login_domain(realm: &str, domain: &str) -> String {
    if !domain.trim().is_empty() {
        return domain.trim().to_string();
    }
    if realm == realms::REALM_INTL {
        return "codebuddy.ai".to_string();
    }
    String::new()
}
/// Bản Go không sanitize — uid chứa `/` sẽ thoát thư mục auth (lỗi ngầm).
pub fn sanitize_uid(uid: &str) -> String {
    let clean: String = uid
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || "@._-".contains(c) { c } else { '_' })
        .collect();
    let clean = clean.trim_matches('.').to_string();
    if clean.is_empty() {
        "unknown".to_string()
    } else {
        clean.chars().take(128).collect()
    }
}

/// Mở URL trong browser hệ thống (không shell — chống injection).
/// URL phải http(s), có host.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    let t = url.trim();
    let rest = if let Some(r) = t.strip_prefix("https://") {
        r
    } else if let Some(r) = t.strip_prefix("http://") {
        r
    } else {
        return Err("refused: URL must start with http:// or https://".into());
    };
    let host = rest.split('/').next().unwrap_or("");
    if host.is_empty() || host.contains(' ') {
        return Err("refused: URL missing host".into());
    }
    #[cfg(target_os = "linux")]
    let r = std::process::Command::new("xdg-open").arg(t).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(t).spawn();
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd").args(["/c", "start", "", t]).spawn();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let r: Result<std::process::Child, std::io::Error> =
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "unsupported OS"));
    r.map(|_| ()).map_err(|e| format!("cannot open browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realm_ok_and_fail_fast() {
        assert_eq!(resolve_realm("").unwrap(), "cn");
        assert_eq!(resolve_realm("intl").unwrap(), "intl");
        assert!(resolve_realm("mars").is_err());
    }

    #[test]
    fn sanitize_uid_blocks_traversal() {
        for bad in ["../../etc", "/abs", "..", "a/b", "x\\y"] {
            let s = sanitize_uid(bad);
            assert!(!s.contains('/'), "{bad} -> {s}");
            assert!(!s.contains('\\'), "{bad} -> {s}");
            assert!(!s.starts_with('.'), "{bad} -> {s}");
            assert!(!s.is_empty(), "{bad}");
        }
        assert_eq!(sanitize_uid("a@b.com"), "a@b.com");
        assert_eq!(sanitize_uid("  "), "unknown");
    }

    #[test]
    fn open_browser_refuses_bad_urls() {
        // Chỉ assert các case validation (không spawn process trong test).
        assert!(open_in_browser("javascript:alert(1)").is_err());
        assert!(open_in_browser("https://").is_err());
        assert!(open_in_browser("").is_err());
        assert!(open_in_browser("ftp://x/y").is_err());
    }

    #[test]
    fn login_domain_prefers_server_value() {
        assert_eq!(resolve_login_domain("intl", "x.codebuddy.ai"), "x.codebuddy.ai");
        assert_eq!(resolve_login_domain("cn", ""), "");
    }

    #[test]
    fn login_domain_intl_defaults_to_codebuddy_ai() {
        // Token intl domain rỗng mà để rỗng sẽ route nhầm sang CN → 401.
        assert_eq!(resolve_login_domain("intl", ""), "codebuddy.ai");
        assert_eq!(resolve_login_domain("intl", "  "), "codebuddy.ai");
    }

    #[test]
    fn jwt_claims_decode() {
        // header.payload.sig — payload {"email":"u@x.ai","name":"U"}
        let tok = "e30.eyJlbWFpbCI6InVAeC5haSIsIm5hbWUiOiJVIn0.c2ln";
        let c = decode_jwt_claims(tok).unwrap();
        assert_eq!(claim_str(&c, &["email"]), "u@x.ai");
        assert!(decode_jwt_claims("not-a-jwt").is_none());
    }
}
