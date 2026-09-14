//! Auth credential parsing and atomic save — ported from Go internal/auth.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub domain: String,
    pub uid: String,
    pub enterprise_id: String,
    pub nickname: String,
    #[serde(skip)]
    pub file_path: String,
}

impl Auth {
    pub fn needs_refresh(&self, within_secs: i64) -> bool {
        if self.expires_at <= 0 {
            return true;
        }
        let now = chrono::Utc::now().timestamp();
        now + within_secs >= self.expires_at
    }
}

/// Parse auth from raw JSON bytes. Supports both nested and flat formats.
pub fn parse_auth(raw: &[u8]) -> Result<Auth, String> {
    if raw.is_empty() {
        return Err("empty auth storage".to_string());
    }
    let probe: serde_json::Value = serde_json::from_slice(raw)
        .map_err(|e| format!("storage_parse_error: {e}"))?;

    let auth = if probe.get("auth").is_some() {
        // Nested format: {"auth":{...}, "account":{...}}
        let nested: NestedAuth = serde_json::from_slice(raw)
            .map_err(|e| format!("storage_parse_error: {e}"))?;
        Auth {
            access_token: nested.auth.access_token,
            refresh_token: nested.auth.refresh_token,
            expires_at: nested.auth.expires_at,
            domain: nested.auth.domain,
            uid: nested.account.uid,
            enterprise_id: nested.account.enterprise_id,
            nickname: nested.account.nickname,
            file_path: String::new(),
        }
    } else {
        // Flat format
        let flat: FlatAuth = serde_json::from_slice(raw)
            .map_err(|e| format!("storage_parse_error: {e}"))?;
        Auth {
            access_token: flat.access_token,
            refresh_token: flat.refresh_token,
            expires_at: flat.expires_at,
            domain: flat.domain,
            uid: flat.uid,
            enterprise_id: flat.enterprise_id,
            nickname: flat.nickname,
            file_path: String::new(),
        }
    };

    if auth.access_token.trim().is_empty() {
        return Err("parse_error: missing accessToken".to_string());
    }
    Ok(auth)
}

/// Atomic save back to file in nested format.
pub fn save_atomic(auth: &Auth) -> Result<(), String> {
    if auth.access_token.trim().is_empty() {
        return Err(format!("save refused: empty accessToken (uid={})", auth.uid));
    }
    if auth.file_path.is_empty() {
        return Err("no file_path set".to_string());
    }
    let doc = serde_json::json!({
        "auth": {
            "accessToken": auth.access_token,
            "refreshToken": auth.refresh_token,
            "expiresAt": auth.expires_at,
            "domain": auth.domain,
        },
        "account": {
            "uid": auth.uid,
            "enterpriseId": auth.enterprise_id,
            "nickname": auth.nickname,
        },
    });
    let raw = serde_json::to_string_pretty(&doc)
        .map_err(|e| format!("serialize error: {e}"))?;
    let tmp = format!("{}.tmp", auth.file_path);
    std::fs::write(&tmp, &raw).map_err(|e| format!("write error: {e}"))?;
    std::fs::rename(&tmp, &auth.file_path).map_err(|e| format!("rename error: {e}"))?;
    Ok(())
}

/// Load all auth files from a directory (glob workbuddy*.json).
pub fn load_dir(dir: &str) -> Result<Vec<Auth>, String> {
    let mut auths = Vec::new();
    let pattern = PathBuf::from(dir).join("workbuddy*.json");
    let _pattern_str = pattern.to_string_lossy();
    // Simple glob: read dir and filter by prefix
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir error: {e}"))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !name.starts_with("workbuddy") || !name.ends_with(".json") {
            continue;
        }
        let raw = match std::fs::read(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        match parse_auth(&raw) {
            Ok(mut a) => {
                a.file_path = path.to_string_lossy().to_string();
                auths.push(a);
            }
            Err(_) => continue,
        }
    }
    Ok(auths)
}

// --- Internal serde helpers ---
#[derive(Deserialize)]
struct NestedAuth {
    auth: NestedAuthInner,
    account: NestedAccount,
}

#[derive(Deserialize)]
struct NestedAuthInner {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken", default)]
    refresh_token: String,
    #[serde(rename = "expiresAt", default)]
    expires_at: i64,
    #[serde(default)]
    domain: String,
}

#[derive(Deserialize)]
struct NestedAccount {
    #[serde(default)]
    uid: String,
    #[serde(rename = "enterpriseId", default)]
    enterprise_id: String,
    #[serde(default)]
    nickname: String,
}

#[derive(Deserialize)]
struct FlatAuth {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "refreshToken", default)]
    refresh_token: String,
    #[serde(rename = "expiresAt", default)]
    expires_at: i64,
    #[serde(default)]
    domain: String,
    #[serde(default)]
    uid: String,
    #[serde(rename = "enterpriseId", default)]
    enterprise_id: String,
    #[serde(default)]
    nickname: String,
}
