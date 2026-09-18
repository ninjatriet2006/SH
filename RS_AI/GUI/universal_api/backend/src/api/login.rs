//! Tauri IPC cho device-login qua browser (port Go `cmd/login` + `login.sh`).
//!
//! Luồng UI: `login_start` → hiện URL + mở browser → poll `login_poll`
//! mỗi vài giây → `done` thì account đã vào pool (live, không cần restart
//! như bản Go) → `login_cancel` để hủy giữa chừng.

use crate::core::auth::Auth;
use crate::core::login::{self, PollOutcome};
use crate::core::runtime::{PendingLogin, RuntimeState};
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct LoginStartRequest {
    /// "cn" (mặc định) | "intl".
    #[serde(default)]
    pub realm: String,
}

#[derive(Serialize)]
pub struct LoginStartResponse {
    pub auth_url: String,
    pub realm: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn login_start(request: Req<LoginStartRequest>, state: State<'_, RuntimeState>) -> IpcResult<LoginStartResponse> {
    let (request_id, payload) = request.validate()?;
    let realm_arg = if payload.realm.trim().is_empty() { "cn" } else { payload.realm.trim() };
    let proxy_url = state
        .config
        .lock()
        .map_err(|_| from_string("Config lock poisoned".into()))?
        .upstream
        .proxy_url
        .clone();
    // Chạy network blocking trên threadpool của Tauri (không block tokio runtime).
    let started = login::login_url(realm_arg, &proxy_url).map_err(from_string)?;
    *state
        .pending_login
        .lock()
        .map_err(|_| from_string("Login lock poisoned".into()))? = Some(PendingLogin {
        state: started.state,
        realm: started.realm.clone(),
    });
    Ok(respond(
        request_id,
        LoginStartResponse { auth_url: started.auth_url, realm: started.realm },
    ))
}

#[derive(Serialize)]
pub struct LoginPollResponse {
    /// "pending" | "done".
    pub status: String,
    pub account: Option<LoginAccountInfo>,
}

#[derive(Serialize)]
pub struct LoginAccountInfo {
    pub uid: String,
    pub nickname: String,
    pub domain: String,
    pub checkin: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn login_poll(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<LoginPollResponse> {
    let (request_id, _) = request.validate()?;
    let pending = state
        .pending_login
        .lock()
        .map_err(|_| from_string("Login lock poisoned".into()))?
        .clone()
        .ok_or_else(|| from_string("no pending login (call login_start first)".into()))?;
    let (proxy_url, auth_dir) = {
        let cfg = state.config.lock().map_err(|_| from_string("Config lock poisoned".into()))?;
        (cfg.upstream.proxy_url.clone(), cfg.auth_dir.clone())
    };
    match login::login_poll_once(&pending.realm, &pending.state, &proxy_url).map_err(from_string)? {
        PollOutcome::Pending => Ok(respond(
            request_id,
            LoginPollResponse { status: "pending".into(), account: None },
        )),
        PollOutcome::Done(bundle) => {
            let login_acct = login::fetch_login_account(&pending.realm, &pending.state, &bundle.access_token, &proxy_url);
            let uid = if login_acct.uid.trim().is_empty() { "unknown".to_string() } else { login_acct.uid.clone() };
            // Domain rỗng + realm intl → ép codebuddy.ai để không route nhầm CN.
            let domain = login::resolve_login_domain(&pending.realm, &bundle.domain);
            let expires_at = if bundle.expires_in > 0 {
                chrono::Utc::now().timestamp() + bundle.expires_in
            } else {
                0
            };
            let file_name = format!("workbuddy-{}.json", login::sanitize_uid(&uid));
            let file_path = std::path::Path::new(&auth_dir).join(&file_name).to_string_lossy().to_string();
            let auth = Auth {
                access_token: bundle.access_token.clone(),
                refresh_token: bundle.refresh_token.clone(),
                expires_at,
                domain: domain.clone(),
                uid: uid.clone(),
                enterprise_id: login_acct.enterprise_id.clone(),
                nickname: login_acct.nickname.clone(),
                file_path: file_path.clone(),
            };
            // Ghi file auth (khớp format parse_auth) — best-effort, lỗi vẫn tiếp tục
            // vì credential đã vào SQLite + pool bên dưới.
            let file_err = crate::core::auth::save_atomic(&auth).err();
            let store = state.storage().map_err(from_string)?;
            store
                .upsert_account(&auth.uid, &auth.domain, &auth.nickname, &auth.enterprise_id,
                                &auth.access_token, &auth.refresh_token, auth.expires_at)
                .map_err(from_string)?;
            state.pool.add(auth);
            // Xóa pending để poll tiếp theo không lặp lại.
            *state.pending_login.lock().map_err(|_| from_string("Login lock poisoned".into()))? = None;
            // Điểm danh ngay sau login như login.sh (best-effort, không fail login).
            let checkin = try_checkin(&state, &uid);
            Ok(respond(
                request_id,
                LoginPollResponse {
                    status: "done".into(),
                    account: Some(LoginAccountInfo {
                        uid,
                        nickname: login_acct.nickname,
                        domain: domain.clone(),
                        checkin: file_err.map(|e| format!("saved to pool, file note: {e}")).unwrap_or(checkin),
                    }),
                },
            ))
        }
    }
}

/// Điểm danh best-effort sau login (bản Go làm trong login.sh).
fn try_checkin(state: &RuntimeState, uid: &str) -> String {
    let account = match state.pool.all_accounts().into_iter().find(|a| a.uid == uid) {
        Some(a) => a.auth.clone(),
        None => return "account registered (checkin skipped)".into(),
    };
    let proxy_url = state
        .config
        .lock()
        .map(|c| c.upstream.proxy_url.clone())
        .unwrap_or_default();
    let client = crate::core::upstream::client::Client::new(&proxy_url);
    match client.daily_checkin(&account) {
        Ok(_) => "check-in: success".into(),
        Err(e) => format!("check-in note: {e}"),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn login_cancel(request: Req<Empty>, state: State<'_, RuntimeState>) -> IpcResult<Empty> {
    let (request_id, _) = request.validate()?;
    *state.pending_login.lock().map_err(|_| from_string("Login lock poisoned".into()))? = None;
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct OpenUrlRequest {
    pub url: String,
}

/// Mở URL trong browser hệ thống (dùng cho trang ủy quyền login).
#[tauri::command(rename_all = "snake_case")]
pub fn open_login_url(request: Req<OpenUrlRequest>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    login::open_in_browser(&payload.url).map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}
