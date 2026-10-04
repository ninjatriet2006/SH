//! Local OAuth server and browser authorization flows for Antigravity and GitHub Copilot.
//! Ported directly from Cockpit Tools (`modules/oauth_server.rs` & `modules/github_copilot_oauth.rs`).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

// =========================================================================
// ANTIGRAVITY GOOGLE OAUTH
// =========================================================================

pub const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
pub const ANTIGRAVITY_CLIENT_SECRET: &str = "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";
pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
pub const ANTIGRAVITY_SCOPES: &str = "openid https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs https://www.googleapis.com/auth/aicode";

// =========================================================================
// GITHUB COPILOT OAUTH
// =========================================================================

pub const GITHUB_CLIENT_ID: &str = "01ab8ac9400c4e429b23";
pub const GITHUB_CLIENT_SECRET: &str = "2af589bb2ffd03a29cc0df83f767e3f6693f14cd";
pub const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const GITHUB_USER_URL: &str = "https://api.github.com/user";
pub const GITHUB_COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
pub const GITHUB_SCOPES: &str = "read:user repo user:email workflow";

#[derive(Debug, Clone)]
pub struct PendingOAuthFlow {
    pub realm: String,
    pub login_id: String,
    pub auth_url: String,
    pub port: u16,
    pub code_verifier: Option<String>,
    pub received_code: Arc<Mutex<Option<String>>>,
    pub error: Arc<Mutex<Option<String>>>,
    pub expires_at: Instant,
}

static PENDING_FLOW: OnceLock<Mutex<Option<PendingOAuthFlow>>> = OnceLock::new();

fn get_pending_flow() -> &'static Mutex<Option<PendingOAuthFlow>> {
    PENDING_FLOW.get_or_init(|| Mutex::new(None))
}

pub fn cancel_oauth() {
    if let Ok(mut lock) = get_pending_flow().lock() {
        *lock = None;
    }
}

fn percent_encode(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

fn generate_random_token(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}

fn parse_http_get_params(req: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let first_line = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        let uri = parts[1];
        if let Some((_, query)) = uri.split_once('?') {
            for pair in query.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    params.insert(k.to_string(), v.to_string());
                }
            }
        }
    }
    params
}

fn send_html_response(mut stream: TcpStream, title: &str, message: &str) {
    let body = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: #0b0f19;
            color: #f1f5f9;
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
        }}
        .card {{
            background: #1e293b;
            border: 1px solid #334155;
            border-radius: 12px;
            padding: 32px 40px;
            text-align: center;
            box-shadow: 0 10px 25px rgba(0,0,0,0.5);
            max-width: 420px;
        }}
        h1 {{ color: #38bdf8; font-size: 24px; margin-bottom: 12px; }}
        p {{ color: #94a3b8; font-size: 15px; line-height: 1.5; }}
        .badge {{ background: #0284c7; color: white; padding: 4px 10px; border-radius: 6px; font-size: 12px; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>{title}</h1>
        <p>{message}</p>
        <p style="margin-top:20px; font-size:12px; color:#64748b;">Cửa sổ này sẽ tự động đóng sau vài giây...</p>
        <script>setTimeout(function() {{ window.close(); }}, 2500);</script>
    </div>
</body>
</html>"#
    );

    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

// =========================================================================
// ANTIGRAVITY OAUTH IMPLEMENTATION
// =========================================================================

pub fn start_antigravity_oauth() -> Result<(String, String), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Không thể mở cổng xác thực cục bộ: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Không đọc được cổng: {e}"))?
        .port();

    let state_token = generate_random_token(16);
    let redirect_uri = format!("http://localhost:{port}/oauth-callback");
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&state={}",
        GOOGLE_AUTH_URL,
        percent_encode(ANTIGRAVITY_CLIENT_ID),
        percent_encode(&redirect_uri),
        percent_encode(ANTIGRAVITY_SCOPES),
        percent_encode(&state_token)
    );

    let received_code = Arc::new(Mutex::new(None));
    let error_state = Arc::new(Mutex::new(None));

    let code_clone = received_code.clone();
    let err_clone = error_state.clone();
    let expected_state = state_token.clone();

    thread::spawn(move || {
        listener.set_nonblocking(false).ok();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0u8; 4096];
                    if let Ok(n) = stream.read(&mut buf) {
                        let req_str = String::from_utf8_lossy(&buf[..n]);
                        let params = parse_http_get_params(&req_str);

                        if let Some(err) = params.get("error") {
                            send_html_response(stream, "❌ Xác thực bị từ chối", &format!("Lỗi: {err}"));
                            if let Ok(mut l) = err_clone.lock() {
                                *l = Some(err.clone());
                            }
                            break;
                        }

                        let code = params.get("code").cloned();
                        let state = params.get("state").cloned();

                        if let Some(c) = code {
                            if state.as_deref() == Some(&expected_state) {
                                send_html_response(
                                    stream,
                                    "✅ Xác thực Google Antigravity thành công!",
                                    "Đã cấp quyền Antigravity thành công. Đang đồng bộ hóa tài khoản vào kho..."
                                );
                                if let Ok(mut l) = code_clone.lock() {
                                    *l = Some(c);
                                }
                                break;
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let flow = PendingOAuthFlow {
        realm: "antigravity".to_string(),
        login_id: state_token.clone(),
        auth_url: auth_url.clone(),
        port,
        code_verifier: None,
        received_code,
        error: error_state,
        expires_at: Instant::now() + Duration::from_secs(600),
    };

    if let Ok(mut lock) = get_pending_flow().lock() {
        *lock = Some(flow);
    }

    Ok((auth_url, state_token))
}

/// Canonical Antigravity account storage ID algorithm directly matching Cockpit Tools
/// (`crates/cockpit-core/src/modules/account.rs:389`).
pub fn build_antigravity_storage_id(email: &str) -> String {
    let seed = email.trim().to_lowercase();
    format!("antigravity_{:x}", md5::compute(seed.as_bytes()))
}

pub fn poll_antigravity_oauth(
    state_token: &str,
    cockpit_dir: &std::path::Path,
) -> Result<Option<OAuthSuccessResult>, String> {
    let flow = {
        let lock = get_pending_flow().lock().map_err(|e| e.to_string())?;
        match lock.as_ref() {
            Some(f) if f.login_id == state_token && f.realm == "antigravity" => f.clone(),
            _ => return Err("Phiên đăng nhập không tồn tại hoặc đã hết hạn".to_string()),
        }
    };

    if Instant::now() > flow.expires_at {
        cancel_oauth();
        return Err("Phiên đăng nhập đã hết thời gian (600s). Vui lòng thử lại.".to_string());
    }

    if let Ok(err_lock) = flow.error.lock() {
        if let Some(err) = err_lock.as_ref() {
            cancel_oauth();
            return Err(format!("Google OAuth trả về lỗi: {err}"));
        }
    }

    let code_opt = flow.received_code.lock().ok().and_then(|c| c.clone());
    let Some(code) = code_opt else {
        return Ok(None); // Pending
    };

    // Exchange Code for Tokens
    let redirect_uri = format!("http://localhost:{}/oauth-callback", flow.port);
    let resp = ureq::post(GOOGLE_TOKEN_URL)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_form(&[
            ("client_id", ANTIGRAVITY_CLIENT_ID),
            ("client_secret", ANTIGRAVITY_CLIENT_SECRET),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .map_err(|e| format!("Lỗi đổi Authorization Code: {e}"))?;

    let token_data: GoogleTokenResponse = resp
        .into_json()
        .map_err(|e| format!("Lỗi phân giải Token JSON: {e}"))?;

    // Fetch userinfo
    let user_resp = ureq::get(GOOGLE_USERINFO_URL)
        .set("Authorization", &format!("Bearer {}", token_data.access_token))
        .call()
        .map_err(|e| format!("Lỗi lấy thông tin Google User: {e}"))?;

    let user_info: GoogleUserInfo = user_resp
        .into_json()
        .map_err(|e| format!("Lỗi phân giải User Info: {e}"))?;

    let email = user_info.email.trim().to_string();
    let name = user_info.name.unwrap_or_else(|| email.clone());
    let account_id = build_antigravity_storage_id(&email);

    // Save into Cockpit tools directory (~/.cockpit_tools/)
    save_antigravity_account_to_cockpit(
        cockpit_dir,
        &account_id,
        &email,
        &name,
        &token_data,
        &user_info.id,
    )?;

    // Fetch initial quota metrics from Google Cloud Code Pa immediately
    if let Err(e) = crate::core::antigravity_quota::fetch_and_save_antigravity_quota(
        cockpit_dir,
        &account_id,
        &email,
        &token_data.access_token,
    ) {
        log::warn!("Không thể lấy quota ban đầu cho Antigravity [{email}]: {e}");
    }

    cancel_oauth();

    Ok(Some(OAuthSuccessResult {
        uid: account_id,
        nickname: email.clone(),
        domain: "antigravity.google.com".to_string(),
        access_token: token_data.access_token,
        refresh_token: token_data.refresh_token.unwrap_or_default(),
        expires_in: token_data.expires_in,
    }))
}

// =========================================================================
// GITHUB COPILOT OAUTH IMPLEMENTATION
// =========================================================================

pub fn start_github_copilot_oauth() -> Result<(String, String), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Không thể mở cổng xác thực GitHub cục bộ: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Không đọc được cổng: {e}"))?
        .port();

    let state_token = generate_random_token(16);
    let code_verifier = generate_random_token(32);
    let code_challenge = {
        let digest = Sha256::digest(code_verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(digest)
    };

    let callback_url = format!("http://127.0.0.1:{port}/callback");
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&get_started_with=copilot-vscode&prompt=select_account",
        GITHUB_AUTH_URL,
        percent_encode(GITHUB_CLIENT_ID),
        percent_encode("https://vscode.dev/redirect"),
        percent_encode(GITHUB_SCOPES),
        percent_encode(&callback_url),
        percent_encode(&code_challenge)
    );

    let received_code = Arc::new(Mutex::new(None));
    let error_state = Arc::new(Mutex::new(None));

    let code_clone = received_code.clone();
    let err_clone = error_state.clone();

    thread::spawn(move || {
        listener.set_nonblocking(false).ok();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0u8; 4096];
                    if let Ok(n) = stream.read(&mut buf) {
                        let req_str = String::from_utf8_lossy(&buf[..n]);
                        let params = parse_http_get_params(&req_str);

                        if let Some(err) = params.get("error") {
                            send_html_response(stream, "❌ GitHub Copilot Xác thực thất bại", &format!("Lỗi: {err}"));
                            if let Ok(mut l) = err_clone.lock() {
                                *l = Some(err.clone());
                            }
                            break;
                        }

                        if let Some(c) = params.get("code") {
                            send_html_response(
                                stream,
                                "✅ Xác thực GitHub Copilot thành công!",
                                "Đã nhận mã ủy quyền GitHub. Đang lưu tài khoản vào kho..."
                            );
                            if let Ok(mut l) = code_clone.lock() {
                                *l = Some(c.clone());
                            }
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let flow = PendingOAuthFlow {
        realm: "github_copilot".to_string(),
        login_id: state_token.clone(),
        auth_url: auth_url.clone(),
        port,
        code_verifier: Some(code_verifier),
        received_code,
        error: error_state,
        expires_at: Instant::now() + Duration::from_secs(600),
    };

    if let Ok(mut lock) = get_pending_flow().lock() {
        *lock = Some(flow);
    }

    Ok((auth_url, state_token))
}

pub fn poll_github_copilot_oauth(
    state_token: &str,
    cockpit_dir: &std::path::Path,
) -> Result<Option<OAuthSuccessResult>, String> {
    let flow = {
        let lock = get_pending_flow().lock().map_err(|e| e.to_string())?;
        match lock.as_ref() {
            Some(f) if f.login_id == state_token && f.realm == "github_copilot" => f.clone(),
            _ => return Err("Phiên đăng nhập GitHub không tồn tại hoặc đã hết hạn".to_string()),
        }
    };

    if Instant::now() > flow.expires_at {
        cancel_oauth();
        return Err("Phiên đăng nhập GitHub đã hết thời gian (600s).".to_string());
    }

    if let Ok(err_lock) = flow.error.lock() {
        if let Some(err) = err_lock.as_ref() {
            cancel_oauth();
            return Err(format!("GitHub OAuth lỗi: {err}"));
        }
    }

    let code_opt = flow.received_code.lock().ok().and_then(|c| c.clone());
    let Some(code) = code_opt else {
        return Ok(None); // Pending
    };

    let verifier = flow.code_verifier.unwrap_or_default();

    // Exchange Code for Access Token
    let resp = ureq::post(GITHUB_TOKEN_URL)
        .set("Accept", "application/json")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("client_secret", GITHUB_CLIENT_SECRET),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", "https://vscode.dev/redirect"),
        ])
        .map_err(|e| format!("Lỗi đổi GitHub token: {e}"))?;

    let token_resp: GitHubTokenResponse = resp
        .into_json()
        .map_err(|e| format!("Lỗi đọc token GitHub: {e}"))?;

    if let Some(err) = token_resp.error {
        let desc = token_resp.error_description.unwrap_or_default();
        cancel_oauth();
        return Err(format!("GitHub OAuth lỗi: {err} ({desc})"));
    }

    let gh_access_token = token_resp
        .access_token
        .ok_or_else(|| "GitHub không trả về access_token".to_string())?;

    // Fetch GitHub User Info
    let user_resp = ureq::get(GITHUB_USER_URL)
        .set("Authorization", &format!("Bearer {gh_access_token}"))
        .set("User-Agent", "antigravity-cockpit-tools")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Lỗi lấy thông tin GitHub user: {e}"))?;

    let user_info: GitHubUserInfo = user_resp
        .into_json()
        .map_err(|e| format!("Lỗi parse GitHub user: {e}"))?;

    let login = user_info.login;
    let email = user_info.email.unwrap_or_else(|| format!("{login}@users.noreply.github.com"));
    let account_id = format!(
        "ghcp_{:x}",
        md5::compute(format!("{}:{}", login, user_info.id))
    );

    // Fetch Copilot internal token
    let copilot_token = match ureq::get(GITHUB_COPILOT_TOKEN_URL)
        .set("Authorization", &format!("token {gh_access_token}"))
        .set("User-Agent", "antigravity-cockpit-tools")
        .set("Accept", "application/json")
        .call()
    {
        Ok(res) => res.into_json::<serde_json::Value>().ok(),
        Err(_) => None,
    };

    let github_id = user_info.id;
    save_copilot_account_to_cockpit(
        cockpit_dir,
        &account_id,
        github_id,
        &login,
        &email,
        &gh_access_token,
        copilot_token.as_ref(),
    )?;

    cancel_oauth();

    Ok(Some(OAuthSuccessResult {
        uid: account_id,
        nickname: login,
        domain: "github.com/copilot".to_string(),
        access_token: gh_access_token,
        refresh_token: String::new(),
        expires_in: 0,
    }))
}

// =========================================================================
// CURSOR OAUTH (PKCE + Cloud Poll)
// =========================================================================

pub const CURSOR_LOGIN_URL: &str = "https://cursor.com/loginDeepControl";
pub const CURSOR_POLL_ENDPOINT: &str = "https://api2.cursor.sh/auth/poll";

#[derive(Deserialize)]
struct CursorPollResponse {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "authId")]
    auth_id: Option<String>,
}

pub fn start_cursor_oauth() -> Result<(String, String), String> {
    cancel_oauth();

    let mut verifier_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let mut uuid_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut uuid_bytes);
    let login_uuid = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid_bytes[0], uuid_bytes[1], uuid_bytes[2], uuid_bytes[3],
        uuid_bytes[4], uuid_bytes[5],
        uuid_bytes[6], uuid_bytes[7],
        uuid_bytes[8], uuid_bytes[9],
        uuid_bytes[10], uuid_bytes[11], uuid_bytes[12], uuid_bytes[13], uuid_bytes[14], uuid_bytes[15]
    );

    let auth_url = format!(
        "{}?challenge={}&uuid={}&mode=login",
        CURSOR_LOGIN_URL, code_challenge, login_uuid
    );

    let flow = PendingOAuthFlow {
        realm: "cursor".to_string(),
        login_id: login_uuid.clone(),
        auth_url: auth_url.clone(),
        port: 0,
        code_verifier: Some(code_verifier),
        received_code: Arc::new(Mutex::new(None)),
        error: Arc::new(Mutex::new(None)),
        expires_at: Instant::now() + Duration::from_secs(600),
    };

    if let Ok(mut lock) = get_pending_flow().lock() {
        *lock = Some(flow);
    }

    Ok((auth_url, login_uuid))
}

pub fn poll_cursor_oauth(
    state_token: &str,
    cockpit_dir: &std::path::Path,
) -> Result<Option<OAuthSuccessResult>, String> {
    let flow = {
        let lock = get_pending_flow().lock().map_err(|e| e.to_string())?;
        match lock.as_ref() {
            Some(f) if f.login_id == state_token && f.realm == "cursor" => f.clone(),
            _ => return Err("Phiên đăng nhập Cursor không tồn tại hoặc đã hết hạn".to_string()),
        }
    };

    if Instant::now() > flow.expires_at {
        cancel_oauth();
        return Err("Phiên đăng nhập Cursor đã hết thời gian (600s). Vui lòng thử lại.".to_string());
    }

    let verifier = flow.code_verifier.as_deref().unwrap_or("");
    let poll_url = format!(
        "{}?uuid={}&verifier={}",
        CURSOR_POLL_ENDPOINT, flow.login_id, verifier
    );

    let resp = match ureq::get(&poll_url)
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(10))
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Ok(None); // Waiting for user in browser
        }
        Err(e) => {
            return Err(format!("Lỗi khi thăm dò Cursor OAuth: {e}"));
        }
    };

    let body = resp
        .into_string()
        .map_err(|e| format!("Lỗi đọc phản hồi Cursor: {e}"))?;
    let data: CursorPollResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Lỗi giải mã phản hồi Cursor: {e}"))?;

    let access_token = match data.access_token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };
    let refresh_token = data.refresh_token.unwrap_or_default();
    let auth_id = data.auth_id.unwrap_or_default();

    let email = if auth_id.contains('@') {
        auth_id.clone()
    } else {
        format!("cursor_{}@cursor.sh", &flow.login_id[..8])
    };

    let identity_seed = if !auth_id.is_empty() {
        auth_id.to_lowercase()
    } else if email.contains('@') {
        email.to_lowercase()
    } else {
        access_token.to_lowercase()
    };
    let account_id = format!("cursor_{:x}", md5::compute(identity_seed.as_bytes()));

    save_cursor_account_to_cockpit(
        cockpit_dir,
        &account_id,
        &email,
        &auth_id,
        &access_token,
        &refresh_token,
    )?;

    cancel_oauth();

    Ok(Some(OAuthSuccessResult {
        uid: account_id,
        nickname: email,
        domain: "cursor.com".to_string(),
        access_token,
        refresh_token,
        expires_in: 30 * 86400,
    }))
}

fn save_cursor_account_to_cockpit(
    cockpit_dir: &std::path::Path,
    account_id: &str,
    email: &str,
    auth_id: &str,
    access_token: &str,
    refresh_token: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp();
    let cursor_json_path = cockpit_dir.join("cursor_accounts.json");
    let mut doc = if let Ok(content) = std::fs::read_to_string(&cursor_json_path) {
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({ "version": "1.0", "accounts": [] }))
    } else {
        serde_json::json!({ "version": "1.0", "accounts": [] })
    };

    let accounts_arr = doc
        .get_mut("accounts")
        .and_then(|a| a.as_array_mut())
        .ok_or_else(|| "Cấu trúc cursor_accounts.json không hợp lệ".to_string())?;

    let mut existing_found = false;
    for item in accounts_arr.iter_mut() {
        if item.get("id").and_then(|x| x.as_str()) == Some(account_id)
            || item.get("email").and_then(|x| x.as_str()) == Some(email)
        {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("last_used".to_string(), serde_json::json!(now));
                if !auth_id.is_empty() {
                    obj.insert("auth_id".to_string(), serde_json::json!(auth_id));
                }
                // Strip raw tokens from summary index
                obj.remove("access_token");
                obj.remove("refresh_token");
            }
            existing_found = true;
            break;
        }
    }

    if !existing_found {
        let mut summary_obj = serde_json::Map::new();
        summary_obj.insert("id".to_string(), serde_json::json!(account_id));
        summary_obj.insert("email".to_string(), serde_json::json!(email));
        if !auth_id.is_empty() {
            summary_obj.insert("auth_id".to_string(), serde_json::json!(auth_id));
        }
        summary_obj.insert("created_at".to_string(), serde_json::json!(now));
        summary_obj.insert("last_used".to_string(), serde_json::json!(now));
        accounts_arr.push(serde_json::Value::Object(summary_obj));
    }

    if doc.get("current_account_id").is_none() {
        doc["current_account_id"] = serde_json::json!(account_id);
    }

    let summary_content = serde_json::to_string_pretty(&doc).unwrap_or_default();
    crate::core::secure_account_storage::write_string_atomic(&cursor_json_path, &summary_content)
        .map_err(|e| format!("Không thể ghi cursor_accounts.json: {e}"))?;

    let acc_dir = cockpit_dir.join("cursor_accounts");
    std::fs::create_dir_all(&acc_dir).ok();
    let detail_file = acc_dir.join(format!("{account_id}.json"));

    let mut auth_raw = serde_json::Map::new();
    auth_raw.insert("accessToken".to_string(), serde_json::json!(access_token));
    auth_raw.insert("refreshToken".to_string(), serde_json::json!(refresh_token));
    if !auth_id.is_empty() {
        auth_raw.insert("authId".to_string(), serde_json::json!(auth_id));
    }

    let mut detail_doc = serde_json::Map::new();
    detail_doc.insert("id".to_string(), serde_json::json!(account_id));
    detail_doc.insert("email".to_string(), serde_json::json!(email));
    if !auth_id.is_empty() {
        detail_doc.insert("auth_id".to_string(), serde_json::json!(auth_id));
    }
    detail_doc.insert("access_token".to_string(), serde_json::json!(access_token));
    if !refresh_token.is_empty() {
        detail_doc.insert("refresh_token".to_string(), serde_json::json!(refresh_token));
    }
    detail_doc.insert("cursor_auth_raw".to_string(), serde_json::Value::Object(auth_raw));
    detail_doc.insert("status".to_string(), serde_json::json!("active"));
    detail_doc.insert("created_at".to_string(), serde_json::json!(now));
    detail_doc.insert("last_used".to_string(), serde_json::json!(now));

    let detail_val = serde_json::Value::Object(detail_doc);
    crate::core::secure_account_storage::save_account_envelope_atomic(&detail_file, "cursor", &detail_val)
        .map_err(|e| format!("Không thể ghi cursor account detail file: {e}"))?;

    Ok(())
}

// =========================================================================
// DATA STRUCTURES & PERSISTENCE
// =========================================================================

pub struct OAuthSuccessResult {
    pub uid: String,
    pub nickname: String,
    pub domain: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    id: Option<String>,
    email: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct GitHubTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct GitHubUserInfo {
    id: u64,
    login: String,
    email: Option<String>,
}

fn save_antigravity_account_to_cockpit(
    cockpit_dir: &std::path::Path,
    account_id: &str,
    email: &str,
    name: &str,
    token: &GoogleTokenResponse,
    user_id: &Option<String>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp();
    let accounts_json_path = cockpit_dir.join("accounts.json");
    let mut index_val = if let Ok(content) = std::fs::read_to_string(&accounts_json_path) {
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({ "version": "2.0", "accounts": [] }))
    } else {
        serde_json::json!({ "version": "2.0", "accounts": [] })
    };

    let accounts_arr = index_val
        .get_mut("accounts")
        .and_then(|a| a.as_array_mut())
        .ok_or_else(|| "Cấu trúc accounts.json không hợp lệ".to_string())?;

    let mut existing_found = false;
    for item in accounts_arr.iter_mut() {
        if item.get("email").and_then(|x| x.as_str()) == Some(email)
            || item.get("id").and_then(|x| x.as_str()) == Some(account_id)
        {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("last_used".to_string(), serde_json::json!(now));
                if !name.is_empty() {
                    obj.insert("name".to_string(), serde_json::json!(name));
                }
                // Strip legacy raw token fields if any
                obj.remove("token");
                obj.remove("token_encrypted");
            }
            existing_found = true;
            break;
        }
    }

    if !existing_found {
        let mut summary = serde_json::Map::new();
        summary.insert("id".to_string(), serde_json::json!(account_id));
        summary.insert("email".to_string(), serde_json::json!(email));
        if !name.is_empty() {
            summary.insert("name".to_string(), serde_json::json!(name));
        }
        summary.insert("created_at".to_string(), serde_json::json!(now));
        summary.insert("last_used".to_string(), serde_json::json!(now));
        accounts_arr.push(serde_json::Value::Object(summary));
    }

    // Set current account if none
    if index_val.get("current_account_id").is_none()
        || index_val["current_account_id"].as_str().unwrap_or("").is_empty()
    {
        index_val["current_account_id"] = serde_json::json!(account_id);
    }

    let summary_content = serde_json::to_string_pretty(&index_val).unwrap_or_default();
    crate::core::secure_account_storage::write_string_atomic(&accounts_json_path, &summary_content)
        .map_err(|e| format!("Không thể ghi accounts.json: {e}"))?;

    // Write ~/.cockpit_tools/accounts/<id>.json encrypted with AES-256-GCM
    let accounts_dir = cockpit_dir.join("accounts");
    std::fs::create_dir_all(&accounts_dir).ok();
    let account_file = accounts_dir.join(format!("{account_id}.json"));

    let expiry_timestamp = now + token.expires_in;
    let mut token_obj = serde_json::Map::new();
    token_obj.insert("access_token".to_string(), serde_json::json!(token.access_token));
    token_obj.insert("refresh_token".to_string(), serde_json::json!(token.refresh_token.clone().unwrap_or_default()));
    token_obj.insert("expires_in".to_string(), serde_json::json!(token.expires_in));
    token_obj.insert("expiry_timestamp".to_string(), serde_json::json!(expiry_timestamp));
    token_obj.insert("token_type".to_string(), serde_json::json!("Bearer"));
    token_obj.insert("email".to_string(), serde_json::json!(email));
    if let Some(sid) = user_id {
        token_obj.insert("session_id".to_string(), serde_json::json!(sid));
    }
    if let Some(id_tok) = &token.id_token {
        token_obj.insert("id_token".to_string(), serde_json::json!(id_tok));
    }

    let mut account_doc = serde_json::Map::new();
    account_doc.insert("id".to_string(), serde_json::json!(account_id));
    account_doc.insert("email".to_string(), serde_json::json!(email));
    if !name.is_empty() {
        account_doc.insert("name".to_string(), serde_json::json!(name));
    }
    account_doc.insert("created_at".to_string(), serde_json::json!(now));
    account_doc.insert("last_used".to_string(), serde_json::json!(now));
    account_doc.insert("disabled".to_string(), serde_json::json!(false));
    account_doc.insert("token".to_string(), serde_json::Value::Object(token_obj));

    let account_val = serde_json::Value::Object(account_doc);
    crate::core::secure_account_storage::save_account_envelope_atomic(&account_file, "antigravity", &account_val)
        .map_err(|e| format!("Không thể ghi account detail file: {e}"))?;

    Ok(())
}

fn save_copilot_account_to_cockpit(
    cockpit_dir: &std::path::Path,
    account_id: &str,
    github_id: u64,
    login: &str,
    email: &str,
    gh_access_token: &str,
    copilot_token: Option<&serde_json::Value>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp();
    let copilot_json_path = cockpit_dir.join("github_copilot_accounts.json");
    let mut doc = if let Ok(content) = std::fs::read_to_string(&copilot_json_path) {
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({ "version": "1.0", "accounts": [] }))
    } else {
        serde_json::json!({ "version": "1.0", "accounts": [] })
    };

    let accounts_arr = doc
        .get_mut("accounts")
        .and_then(|a| a.as_array_mut())
        .ok_or_else(|| "Cấu trúc github_copilot_accounts.json không hợp lệ".to_string())?;

    let copilot_jwt = copilot_token
        .and_then(|t| t.get("token"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let copilot_expires_at = copilot_token
        .and_then(|t| t.get("expires_at"))
        .and_then(|v| v.as_i64());

    let copilot_plan = copilot_token
        .and_then(|t| t.get("sku"))
        .or_else(|| copilot_token.and_then(|t| t.get("plan")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| Some("individual".to_string()));

    let mut existing_found = false;
    for item in accounts_arr.iter_mut() {
        if item.get("id").and_then(|x| x.as_str()) == Some(account_id)
            || item.get("github_login").and_then(|x| x.as_str()) == Some(login)
        {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("last_used".to_string(), serde_json::json!(now));
                if let Some(plan) = &copilot_plan {
                    obj.insert("copilot_plan".to_string(), serde_json::json!(plan));
                }
                // Strip raw tokens from summary index
                obj.remove("github_access_token");
                obj.remove("copilot_token");
            }
            existing_found = true;
            break;
        }
    }

    if !existing_found {
        let mut summary = serde_json::Map::new();
        summary.insert("id".to_string(), serde_json::json!(account_id));
        summary.insert("github_login".to_string(), serde_json::json!(login));
        if !email.is_empty() {
            summary.insert("github_email".to_string(), serde_json::json!(email));
        }
        if let Some(plan) = &copilot_plan {
            summary.insert("copilot_plan".to_string(), serde_json::json!(plan));
        }
        summary.insert("created_at".to_string(), serde_json::json!(now));
        summary.insert("last_used".to_string(), serde_json::json!(now));
        accounts_arr.push(serde_json::Value::Object(summary));
    }

    if doc.get("current_account_id").is_none() {
        doc["current_account_id"] = serde_json::json!(account_id);
    }

    let summary_content = serde_json::to_string_pretty(&doc).unwrap_or_default();
    crate::core::secure_account_storage::write_string_atomic(&copilot_json_path, &summary_content)
        .map_err(|e| format!("Không thể ghi github_copilot_accounts.json: {e}"))?;

    // Write ~/.cockpit_tools/github_copilot_accounts/<id>.json encrypted with AES-256-GCM
    let gh_dir = cockpit_dir.join("github_copilot_accounts");
    std::fs::create_dir_all(&gh_dir).ok();
    let detail_file = gh_dir.join(format!("{account_id}.json"));

    let mut detail_doc = serde_json::Map::new();
    detail_doc.insert("id".to_string(), serde_json::json!(account_id));
    detail_doc.insert("github_login".to_string(), serde_json::json!(login));
    detail_doc.insert("github_id".to_string(), serde_json::json!(github_id));
    if !email.is_empty() {
        detail_doc.insert("github_email".to_string(), serde_json::json!(email));
    }
    detail_doc.insert("github_access_token".to_string(), serde_json::json!(gh_access_token));
    detail_doc.insert("copilot_token".to_string(), serde_json::json!(copilot_jwt));
    if let Some(plan) = &copilot_plan {
        detail_doc.insert("copilot_plan".to_string(), serde_json::json!(plan));
    }
    if let Some(exp) = copilot_expires_at {
        detail_doc.insert("copilot_expires_at".to_string(), serde_json::json!(exp));
    }
    detail_doc.insert("created_at".to_string(), serde_json::json!(now));
    detail_doc.insert("last_used".to_string(), serde_json::json!(now));

    let detail_val = serde_json::Value::Object(detail_doc);
    crate::core::secure_account_storage::save_account_envelope_atomic(&detail_file, "github_copilot", &detail_val)
        .map_err(|e| format!("Không thể ghi copilot account detail file: {e}"))?;

    Ok(())
}
