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

    let (auth_url, state_val, realm_val) = match realm_arg {
        "antigravity" => {
            let (url, state) = crate::core::oauth::start_antigravity_oauth().map_err(from_string)?;
            (url, state, "antigravity".to_string())
        }
        "github_copilot" | "copilot" => {
            let (url, state) = crate::core::oauth::start_github_copilot_oauth().map_err(from_string)?;
            (url, state, "github_copilot".to_string())
        }
        "cursor" => {
            let (url, state) = crate::core::oauth::start_cursor_oauth().map_err(from_string)?;
            (url, state, "cursor".to_string())
        }
        "intl" | "codebuddy_global" => {
            let started = crate::core::platforms::codebuddy_global::auth::start_login_global(proxy_opt).map_err(from_string)?;
            (started.auth_url, started.state, "codebuddy_global".to_string())
        }
        _ => {
            let started = crate::core::platforms::codebuddy_cn::auth::start_login_cn(proxy_opt).map_err(from_string)?;
            (started.auth_url, started.state, "codebuddy_cn".to_string())
        }
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
    let cockpit_dir = crate::core::paths::cockpit_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));

    // Antigravity Google OAuth
    if pending.realm == "antigravity" {
        let outcome = crate::core::oauth::poll_antigravity_oauth(&pending.state, &cockpit_dir).map_err(from_string)?;
        return match outcome {
            None => Ok(respond(
                request_id,
                LoginPollResponse { status: "pending".into(), account: None },
            )),
            Some(res) => {
                *state.pending_login.lock().map_err(|_| from_string("Login lock poisoned".into()))? = None;
                let auth = Auth {
                    access_token: res.access_token.clone(),
                    refresh_token: res.refresh_token.clone(),
                    expires_at: if res.expires_in > 0 { chrono::Utc::now().timestamp() + res.expires_in } else { 0 },
                    domain: res.domain.clone(),
                    uid: res.uid.clone(),
                    enterprise_id: String::new(),
                    nickname: res.nickname.clone(),
                    file_path: format!("antigravity-{}.json", res.uid),
                };
                if let Ok(store) = state.storage() {
                    let _ = store.upsert_account(&auth.uid, &auth.domain, &auth.nickname, &auth.enterprise_id,
                                                &auth.access_token, &auth.refresh_token, auth.expires_at);
                }
                state.pool.add(auth);
                Ok(respond(
                    request_id,
                    LoginPollResponse {
                        status: "done".into(),
                        account: Some(LoginAccountInfo {
                            uid: res.uid,
                            nickname: res.nickname,
                            domain: res.domain,
                            checkin: "Google Antigravity OAuth thành công!".to_string(),
                        }),
                    },
                ))
            }
        };
    }

    // GitHub Copilot OAuth
    if pending.realm == "github_copilot" || pending.realm == "copilot" {
        let outcome = crate::core::oauth::poll_github_copilot_oauth(&pending.state, &cockpit_dir).map_err(from_string)?;
        return match outcome {
            None => Ok(respond(
                request_id,
                LoginPollResponse { status: "pending".into(), account: None },
            )),
            Some(res) => {
                *state.pending_login.lock().map_err(|_| from_string("Login lock poisoned".into()))? = None;
                let auth = Auth {
                    access_token: res.access_token.clone(),
                    refresh_token: res.refresh_token.clone(),
                    expires_at: if res.expires_in > 0 { chrono::Utc::now().timestamp() + res.expires_in } else { 0 },
                    domain: res.domain.clone(),
                    uid: res.uid.clone(),
                    enterprise_id: String::new(),
                    nickname: res.nickname.clone(),
                    file_path: format!("ghcp-{}.json", res.uid),
                };
                if let Ok(store) = state.storage() {
                    let _ = store.upsert_account(&auth.uid, &auth.domain, &auth.nickname, &auth.enterprise_id,
                                                &auth.access_token, &auth.refresh_token, auth.expires_at);
                }
                state.pool.add(auth);
                Ok(respond(
                    request_id,
                    LoginPollResponse {
                        status: "done".into(),
                        account: Some(LoginAccountInfo {
                            uid: res.uid,
                            nickname: res.nickname,
                            domain: res.domain,
                            checkin: "GitHub Copilot OAuth thành công!".to_string(),
                        }),
                    },
                ))
            }
        };
    }

    // Cursor OAuth
    if pending.realm == "cursor" {
        let outcome = crate::core::oauth::poll_cursor_oauth(&pending.state, &cockpit_dir).map_err(from_string)?;
        return match outcome {
            None => Ok(respond(
                request_id,
                LoginPollResponse { status: "pending".into(), account: None },
            )),
            Some(res) => {
                *state.pending_login.lock().map_err(|_| from_string("Login lock poisoned".into()))? = None;
                let auth = Auth {
                    access_token: res.access_token.clone(),
                    refresh_token: res.refresh_token.clone(),
                    expires_at: if res.expires_in > 0 { chrono::Utc::now().timestamp() + res.expires_in } else { 0 },
                    domain: res.domain.clone(),
                    uid: res.uid.clone(),
                    enterprise_id: String::new(),
                    nickname: res.nickname.clone(),
                    file_path: format!("cursor-{}.json", res.uid),
                };
                if let Ok(store) = state.storage() {
                    let _ = store.upsert_account(&auth.uid, &auth.domain, &auth.nickname, &auth.enterprise_id,
                                                &auth.access_token, &auth.refresh_token, auth.expires_at);
                }
                state.pool.add(auth);
                Ok(respond(
                    request_id,
                    LoginPollResponse {
                        status: "done".into(),
                        account: Some(LoginAccountInfo {
                            uid: res.uid,
                            nickname: res.nickname,
                            domain: res.domain,
                            checkin: "Cursor OAuth thành công!".to_string(),
                        }),
                    },
                ))
            }
        };
    }

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
    crate::core::oauth::cancel_oauth();
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
