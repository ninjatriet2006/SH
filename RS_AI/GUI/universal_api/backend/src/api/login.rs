//! Tauri IPC cho device-login qua browser (port Go `cmd/login` + `login.sh`).
//!
//! Luồng UI: `login_start` → hiện URL + mở browser → poll `login_poll`
//! mỗi vài giây → `done` thì account đã vào pool (live, không cần restart
//! như bản Go) → `login_cancel` để hủy giữa chừng.

use crate::core::auth::Auth;
use crate::core::login;
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
    let proxy_opt = if proxy_url.trim().is_empty() { None } else { Some(proxy_url.as_str()) };

    // Tách biệt hoàn toàn: CodeBuddy Global vs CodeBuddy CN
    let is_global = realm_arg == "intl" || realm_arg == "codebuddy_global";
    let (auth_url, state_val, realm_val) = if is_global {
        let started = crate::core::platforms::codebuddy_global::auth::start_login_global(proxy_opt).map_err(from_string)?;
        (started.auth_url, started.state, "codebuddy_global".to_string())
    } else {
        let started = crate::core::platforms::codebuddy_cn::auth::start_login_cn(proxy_opt).map_err(from_string)?;
        (started.auth_url, started.state, "codebuddy_cn".to_string())
    };

    *state
        .pending_login
        .lock()
        .map_err(|_| from_string("Login lock poisoned".into()))? = Some(PendingLogin {
        state: state_val,
        realm: realm_val.clone(),
    });
    Ok(respond(
        request_id,
        LoginStartResponse { auth_url, realm: realm_val },
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
    let proxy_opt = if proxy_url.trim().is_empty() { None } else { Some(proxy_url.as_str()) };

    let is_global = pending.realm == "intl" || pending.realm == "codebuddy_global";
    let outcome = if is_global {
        crate::core::platforms::codebuddy_global::auth::poll_login_global(&pending.state, proxy_opt)
    } else {
        crate::core::platforms::codebuddy_cn::auth::poll_login_cn(&pending.state, proxy_opt)
    }
    .map_err(from_string)?;

    match outcome {
        crate::core::platforms::PlatformPollOutcome::Pending => Ok(respond(
            request_id,
            LoginPollResponse { status: "pending".into(), account: None },
        )),
        crate::core::platforms::PlatformPollOutcome::Done(bundle) => {
            let acct_info = if is_global {
                crate::core::platforms::codebuddy_global::account::fetch_account_global(&pending.state, &bundle.access_token, proxy_opt)
            } else {
                crate::core::platforms::codebuddy_cn::account::fetch_account_cn(&pending.state, &bundle.access_token, proxy_opt)
            };
            let uid = if acct_info.uid.trim().is_empty() { "unknown".to_string() } else { acct_info.uid.clone() };
            let domain = if is_global {
                "codebuddy.ai".to_string()
            } else if !bundle.domain.trim().is_empty() {
                bundle.domain.trim().to_string()
            } else {
                "copilot.tencent.com".to_string()
            };
            let expires_at = if bundle.expires_in > 0 {
                chrono::Utc::now().timestamp() + bundle.expires_in
            } else {
                0
            };
            let file_name = format!("workbuddy-{}.json", crate::core::login::sanitize_uid(&uid));
            let file_path = std::path::Path::new(&auth_dir).join(&file_name).to_string_lossy().to_string();
            let auth = Auth {
                access_token: bundle.access_token.clone(),
                refresh_token: bundle.refresh_token.clone(),
                expires_at,
                domain: domain.clone(),
                uid: uid.clone(),
                enterprise_id: acct_info.enterprise_id.clone(),
                nickname: acct_info.nickname.clone(),
                file_path: file_path.clone(),
            };
            let file_err = crate::core::auth::save_atomic(&auth).err();
            let store = state.storage().map_err(from_string)?;
            store
                .upsert_account(&auth.uid, &auth.domain, &auth.nickname, &auth.enterprise_id,
                                &auth.access_token, &auth.refresh_token, auth.expires_at)
                .map_err(from_string)?;
            state.pool.add(auth);
            *state.pending_login.lock().map_err(|_| from_string("Login lock poisoned".into()))? = None;

            let checkin = if !is_global {
                crate::core::platforms::codebuddy_cn::account::daily_checkin_cn(&bundle.access_token, proxy_opt)
                    .unwrap_or_else(|e| format!("checkin error: {e}"))
            } else {
                "checkin skipped (global)".to_string()
            };

            Ok(respond(
                request_id,
                LoginPollResponse {
                    status: "done".into(),
                    account: Some(LoginAccountInfo {
                        uid,
                        nickname: acct_info.nickname,
                        domain: domain.clone(),
                        checkin: file_err.map(|e| format!("saved to pool, file note: {e}")).unwrap_or(checkin),
                    }),
                },
            ))
        }
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
