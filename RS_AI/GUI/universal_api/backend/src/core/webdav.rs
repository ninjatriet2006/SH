//! WebDAV Cloud Backup & Restore Module.
//!
//! Provides cloud synchronization for `~/.cockpit_tools` configuration,
//! account envelopes, and instance profiles using standard WebDAV (Nextcloud, Nutstore/Jianguoyun, InfiniCLOUD, etc.).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};

use crate::core::secure_account_storage::write_string_atomic;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebdavSettings {
    pub url: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
    pub enabled: bool,
    pub last_backup_at: Option<String>,
}

fn default_remote_dir() -> String {
    "cockpit-backups".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebdavRemoteFile {
    pub file_name: String,
    pub size_bytes: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebdavBackupBundle {
    pub version: String,
    pub created_at: String,
    pub files: HashMap<String, String>,
}

fn get_cockpit_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cockpit_tools")
}

fn settings_path() -> PathBuf {
    get_cockpit_dir().join("webdav_settings.json")
}

pub fn load_webdav_settings() -> WebdavSettings {
    let p = settings_path();
    if let Ok(raw) = fs::read_to_string(&p) {
        if let Ok(s) = serde_json::from_str::<WebdavSettings>(&raw) {
            return s;
        }
    }
    WebdavSettings {
        url: "".to_string(),
        username: "".to_string(),
        password: "".to_string(),
        remote_dir: default_remote_dir(),
        enabled: false,
        last_backup_at: None,
    }
}

pub fn save_webdav_settings(settings: &WebdavSettings) -> Result<(), String> {
    let p = settings_path();
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    write_string_atomic(&p, &content)
}

fn build_auth_header(username: &str, password: &str) -> String {
    let creds = format!("{}:{}", username, password);
    format!("Basic {}", general_purpose::STANDARD.encode(creds))
}

fn normalize_base_url(url: &str) -> String {
    let mut trimmed = url.trim().to_string();
    while trimmed.ends_with('/') {
        trimmed.pop();
    }
    trimmed
}

pub fn test_webdav_connection(settings: &WebdavSettings) -> Result<String, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("Vui lòng nhập WebDAV URL hợp lệ".to_string());
    }

    let auth = build_auth_header(&settings.username, &settings.password);

    let resp = ureq::request("PROPFIND", &base)
        .set("Authorization", &auth)
        .set("Depth", "0")
        .call();

    match resp {
        Ok(r) => {
            if r.status() == 200 || r.status() == 207 {
                Ok("Kết nối WebDAV thành công!".to_string())
            } else {
                Err(format!("WebDAV phản hồi mã trạng thái HTTP {}", r.status()))
            }
        }
        Err(e) => {
            // Some servers return 405 on PROPFIND at root, try OPTIONS as fallback
            let opt_resp = ureq::request("OPTIONS", &base)
                .set("Authorization", &auth)
                .call();
            match opt_resp {
                Ok(r) if r.status() == 200 => Ok("Kết nối WebDAV thành công (OPTIONS)!".to_string()),
                _ => Err(format!("Lỗi kết nối WebDAV: {e}")),
            }
        }
    }
}

fn ensure_remote_dir(settings: &WebdavSettings) -> Result<(), String> {
    let base = normalize_base_url(&settings.url);
    let remote_dir = settings.remote_dir.trim().trim_matches('/');
    if remote_dir.is_empty() {
        return Ok(());
    }

    let auth = build_auth_header(&settings.username, &settings.password);
    let target = format!("{}/{}", base, remote_dir);

    // Try MKCOL
    let _ = ureq::request("MKCOL", &target)
        .set("Authorization", &auth)
        .call();

    Ok(())
}

pub fn backup_to_webdav(settings: &WebdavSettings) -> Result<String, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("WebDAV URL trống".to_string());
    }

    ensure_remote_dir(settings)?;

    let cockpit_dir = get_cockpit_dir();
    let mut files_map = HashMap::new();

    // 1. Collect root json files
    if let Ok(entries) = fs::read_dir(&cockpit_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |e| e == "json") {
                if let Some(name) = p.file_name().and_then(|x| x.to_str()) {
                    if name != "webdav_settings.json" { // don't upload webdav password
                        if let Ok(content) = fs::read_to_string(&p) {
                            files_map.insert(name.to_string(), content);
                        }
                    }
                }
            }
        }
    }

    // 2. Collect sub-directories with account details (accounts, github_copilot_accounts, cursor_accounts, etc.)
    let subdirs = [
        "accounts", "github_copilot_accounts", "cursor_accounts", "windsurf_accounts",
        "trae_accounts", "codebuddy_accounts", "workbuddy_accounts", "claude_accounts",
        "codex_accounts", "zed_accounts"
    ];

    for sub in subdirs {
        let dir_p = cockpit_dir.join(sub);
        if dir_p.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir_p) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map_or(false, |e| e == "json") {
                        if let Some(file_name) = p.file_name().and_then(|x| x.to_str()) {
                            let rel_key = format!("{}/{}", sub, file_name);
                            if let Ok(content) = fs::read_to_string(&p) {
                                files_map.insert(rel_key, content);
                            }
                        }
                    }
                }
            }
        }
    }

    let bundle = WebdavBackupBundle {
        version: "1.0".to_string(),
        created_at: Utc::now().to_rfc3339(),
        files: files_map,
    };

    let bundle_json = serde_json::to_string_pretty(&bundle)
        .map_err(|e| format!("Lỗi serialize backup bundle: {e}"))?;

    let timestamp_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let file_name = format!("cockpit-backup-{timestamp_str}.json");

    let remote_dir = settings.remote_dir.trim().trim_matches('/');
    let upload_url = if remote_dir.is_empty() {
        format!("{}/{}", base, file_name)
    } else {
        format!("{}/{}/{}", base, remote_dir, file_name)
    };

    let auth = build_auth_header(&settings.username, &settings.password);

    let resp = ureq::put(&upload_url)
        .set("Authorization", &auth)
        .set("Content-Type", "application/json")
        .send_bytes(bundle_json.as_bytes())
        .map_err(|e| format!("Lỗi tải backup lên WebDAV: {e}"))?;

    if resp.status() >= 200 && resp.status() < 300 {
        let mut updated_settings = settings.clone();
        updated_settings.last_backup_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        let _ = save_webdav_settings(&updated_settings);
        Ok(format!("Sao lưu thành công: {file_name}"))
    } else {
        Err(format!("WebDAV phản hồi lỗi tải lên HTTP {}", resp.status()))
    }
}

pub fn list_remote_backups(settings: &WebdavSettings) -> Result<Vec<WebdavRemoteFile>, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("WebDAV URL trống".to_string());
    }

    let remote_dir = settings.remote_dir.trim().trim_matches('/');
    let target = if remote_dir.is_empty() {
        base.clone()
    } else {
        format!("{}/{}", base, remote_dir)
    };

    let auth = build_auth_header(&settings.username, &settings.password);

    let resp = ureq::request("PROPFIND", &target)
        .set("Authorization", &auth)
        .set("Depth", "1")
        .call()
        .map_err(|e| format!("Lỗi gửi PROPFIND: {e}"))?;

    let body = resp.into_string().unwrap_or_default();
    let mut files = Vec::new();

    // Simple robust regex extraction of hrefs ending with cockpit-backup-*.json
    let re = regex::Regex::new(r"(?i)<(?:d:)?href>([^<]*cockpit-backup-[^<]*\.json)</(?:d:)?href>").unwrap();
    for cap in re.captures_iter(&body) {
        if let Some(matched) = cap.get(1) {
            let full_href = matched.as_str();
            let name = full_href.split('/').last().unwrap_or(full_href).to_string();
            if !files.iter().any(|f: &WebdavRemoteFile| f.file_name == name) {
                files.push(WebdavRemoteFile {
                    file_name: name,
                    size_bytes: 0,
                    modified_at: None,
                });
            }
        }
    }

    files.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(files)
}

pub fn restore_remote_backup(settings: &WebdavSettings, file_name: &str) -> Result<usize, String> {
    let base = normalize_base_url(&settings.url);
    if base.is_empty() {
        return Err("WebDAV URL trống".to_string());
    }

    let remote_dir = settings.remote_dir.trim().trim_matches('/');
    let target = if remote_dir.is_empty() {
        format!("{}/{}", base, file_name)
    } else {
        format!("{}/{}/{}", base, remote_dir, file_name)
    };

    let auth = build_auth_header(&settings.username, &settings.password);

    let resp = ureq::get(&target)
        .set("Authorization", &auth)
        .call()
        .map_err(|e| format!("Lỗi tải backup file từ WebDAV: {e}"))?;

    let bundle: WebdavBackupBundle = resp.into_json().map_err(|e| format!("Parse backup JSON: {e}"))?;

    let cockpit_dir = get_cockpit_dir();
    let mut restored = 0;

    for (rel_path, content) in bundle.files {
        let dest = cockpit_dir.join(&rel_path);
        if let Ok(()) = write_string_atomic(&dest, &content) {
            restored += 1;
        }
    }

    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webdav_auth_header_and_url() {
        let auth = build_auth_header("user", "pass");
        assert!(auth.starts_with("Basic "));

        let norm1 = normalize_base_url("https://dav.example.com/dav/");
        assert_eq!(norm1, "https://dav.example.com/dav");

        let norm2 = normalize_base_url("https://dav.example.com/dav");
        assert_eq!(norm2, "https://dav.example.com/dav");
    }

    #[test]
    fn test_webdav_backup_bundle_roundtrip() {
        let mut files = HashMap::new();
        files.insert("accounts/acc1.json".to_string(), "{\"id\":1}".to_string());
        files.insert("config.json".to_string(), "{\"theme\":\"dark\"}".to_string());

        let bundle = WebdavBackupBundle {
            version: "1.0.0".to_string(),
            created_at: Utc::now().to_rfc3339(),
            files,
        };

        let serialized = serde_json::to_string(&bundle).unwrap();
        let deserialized: WebdavBackupBundle = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.files.len(), 2);
    }
}
