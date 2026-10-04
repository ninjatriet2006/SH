//! Synchronization of Cockpit Tools account files into Universal API pool & SQLite storage.

use crate::providers::storage::read_cockpit_secure_json;
use crate::core::runtime::RuntimeState;
use std::path::Path;

pub fn sync_cockpit_accounts_to_storage_and_pool(state: &RuntimeState, platform: Option<&str>) -> usize {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");
    let mut imported = 0;
    let target = platform.unwrap_or("all");

    let import_one = |uid: &str, domain: &str, nickname: &str, enterprise_id: &str, access_token: &str, refresh_token: &str, expires_at: i64, file_path: &str| -> bool {
        if uid.is_empty() {
            return false;
        }
        let auth = crate::core::auth::Auth {
            file_path: file_path.to_string(),
            uid: uid.to_string(),
            domain: domain.to_string(),
            nickname: nickname.to_string(),
            enterprise_id: enterprise_id.to_string(),
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expires_at,
        };
        if let Ok(store) = state.storage() {
            let _ = store.upsert_account(
                &auth.uid,
                &auth.domain,
                &auth.nickname,
                &auth.enterprise_id,
                &auth.access_token,
                &auth.refresh_token,
                auth.expires_at,
            );
        }
        state.pool.add(auth);
        true
    };

    // 1. Antigravity Google Accounts
    if target.is_empty() || target == "antigravity" || target == "all" {
        let acc_dir = cockpit_dir.join("accounts");
        if acc_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&acc_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let email = v.get("email").or_else(|| v.get("name")).and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.pointer("/token/access_token").or_else(|| v.get("access_token")).and_then(|x| x.as_str()).unwrap_or("");
                            let refresh_token = v.pointer("/token/refresh_token").or_else(|| v.get("refresh_token")).and_then(|x| x.as_str()).unwrap_or("");
                            let expires_at = v.pointer("/token/expiry_timestamp").or_else(|| v.get("expires_at")).and_then(|x| x.as_i64()).unwrap_or(0);
                            if import_one(id, "antigravity.google.com", email, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                imported += 1;
                            }
                        }
                    }
                }
            }
        }
        let acc_json = cockpit_dir.join("accounts.json");
        if let Some(v) = read_cockpit_secure_json(&acc_json, &key_path) {
            if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                for item in arr {
                    let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    let email = item.get("email").or_else(|| item.get("name")).and_then(|x| x.as_str()).unwrap_or(id);
                    let access_token = item.pointer("/token/access_token").or_else(|| item.get("access_token")).and_then(|x| x.as_str()).unwrap_or("");
                    let refresh_token = item.pointer("/token/refresh_token").or_else(|| item.get("refresh_token")).and_then(|x| x.as_str()).unwrap_or("");
                    let expires_at = item.pointer("/token/expiry_timestamp").or_else(|| item.get("expires_at")).and_then(|x| x.as_i64()).unwrap_or(0);
                    if import_one(id, "antigravity.google.com", email, "", access_token, refresh_token, expires_at, &acc_json.to_string_lossy()) {
                        imported += 1;
                    }
                }
            }
        }
    }

    // 2. GitHub Copilot Accounts
    if target.is_empty() || target == "github_copilot" || target == "copilot" || target == "all" {
        let gh_dir = cockpit_dir.join("github_copilot_accounts");
        if gh_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&gh_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let login = v.get("github_email").or_else(|| v.get("github_login")).and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.get("copilot_token")
                                .or_else(|| v.get("github_access_token"))
                                .or_else(|| v.pointer("/token/access_token"))
                                .or_else(|| v.get("access_token"))
                                .and_then(|x| x.as_str())
                                .unwrap_or("");
                            let refresh_token = v.get("github_refresh_token").and_then(|x| x.as_str()).unwrap_or("");
                            let expires_at = v.get("copilot_expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                            if !access_token.is_empty() {
                                if import_one(id, "github.com/copilot", login, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                    imported += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. CodeBuddy (CN & Global)
    if target.is_empty() || target.starts_with("codebuddy") || target == "all" {
        let cb_dirs = [("codebuddy_accounts", "codebuddy.ai"), ("codebuddy_cn_accounts", "copilot.tencent.com")];
        for (dir_name, domain) in cb_dirs {
            let p_dir = cockpit_dir.join(dir_name);
            if p_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&p_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "json") {
                            if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                                let id = v.get("id").or_else(|| v.get("uid")).and_then(|x| x.as_str()).unwrap_or("");
                                let nickname = v.get("nickname").or_else(|| v.get("email")).and_then(|x| x.as_str()).unwrap_or(id);
                                let access_token = v.get("access_token").or_else(|| v.pointer("/token/access_token")).and_then(|x| x.as_str()).unwrap_or("");
                                let refresh_token = v.get("refresh_token").or_else(|| v.pointer("/token/refresh_token")).and_then(|x| x.as_str()).unwrap_or("");
                                let expires_at = v.get("expires_at").or_else(|| v.pointer("/token/expiry_timestamp")).and_then(|x| x.as_i64()).unwrap_or(0);
                                let enterprise_id = v.get("enterprise_id").and_then(|x| x.as_str()).unwrap_or("");
                                if !access_token.is_empty() {
                                    if import_one(id, domain, nickname, enterprise_id, access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                        imported += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Cursor
    if target.is_empty() || target == "cursor" || target == "all" {
        let cur_dir = cockpit_dir.join("cursor_accounts");
        if cur_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&cur_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |ext| ext == "json") {
                        if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                            let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("");
                            let email = v.get("email").and_then(|x| x.as_str()).unwrap_or(id);
                            let access_token = v.get("access_token")
                                .or_else(|| v.pointer("/token/access_token"))
                                .and_then(|x| x.as_str())
                                .unwrap_or("");
                            let refresh_token = v.get("refresh_token")
                                .or_else(|| v.pointer("/token/refresh_token"))
                                .and_then(|x| x.as_str())
                                .unwrap_or("");
                            let expires_at = v.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                            if !access_token.is_empty() {
                                if import_one(id, "cursor.com", email, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                    imported += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Windsurf, Trae, Claude, Codex, Kiro, Qoder, ZCode, Grok, WorkBuddy
    let other_platforms = [
        ("windsurf_accounts", "windsurf.codeium.com"),
        ("trae_accounts", "trae.ai"),
        ("claude_accounts", "claude.ai"),
        ("codex_accounts", "chatgpt.com"),
        ("kiro_accounts", "kiro.dev"),
        ("qoder_accounts", "qoder.ai"),
        ("zcode_accounts", "zcode.ai"),
        ("grok_accounts", "x.ai"),
        ("workbuddy_accounts", "workbuddy.cn"),
    ];

    for (dir_name, domain) in other_platforms {
        let pf_tag = dir_name.trim_end_matches("_accounts");
        if target.is_empty() || target == pf_tag || target == "all" {
            let p_dir = cockpit_dir.join(dir_name);
            if p_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&p_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "json") {
                            if let Some(v) = read_cockpit_secure_json(&p, &key_path) {
                                let id = v.get("id").or_else(|| v.get("uid")).and_then(|x| x.as_str()).unwrap_or("");
                                let nickname = v.get("nickname")
                                    .or_else(|| v.get("email"))
                                    .or_else(|| v.get("name"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or(id);
                                let access_token = v.get("access_token")
                                    .or_else(|| v.pointer("/token/access_token"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("");
                                let refresh_token = v.get("refresh_token")
                                    .or_else(|| v.pointer("/token/refresh_token"))
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("");
                                let expires_at = v.get("expires_at").and_then(|x| x.as_i64()).unwrap_or(0);
                                if !access_token.is_empty() {
                                    if import_one(id, domain, nickname, "", access_token, refresh_token, expires_at, &p.to_string_lossy()) {
                                        imported += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 6. Native Zed accounts from storage
    if target.is_empty() || target == "zed" || target == "all" {
        if let Ok(store) = state.storage() {
            if let Ok(zed_list) = store.list_zed_accounts() {
                for z in zed_list {
                    let auth = crate::core::auth::Auth {
                        file_path: format!("zed-account-{}", z.id),
                        uid: z.id.clone(),
                        domain: "cloud.zed.dev".to_string(),
                        nickname: if z.label.is_empty() { z.id.clone() } else { z.label.clone() },
                        enterprise_id: z.org_id.clone(),
                        access_token: String::new(),
                        refresh_token: String::new(),
                        expires_at: 0,
                    };
                    state.pool.add(auth);
                    imported += 1;
                }
            }
        }
    }

    imported
}
