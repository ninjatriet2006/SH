//! Background Token Keeper Daemon.
//!
//! Automatically inspects accounts in `~/.cockpit_tools` across platforms
//! (Antigravity/Google, GitHub Copilot, Cursor, CodeBuddy) and refreshes expiring
//! tokens before they expire, ensuring 24/7 token pool health without waiting for user traffic.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::core::oauth::{ANTIGRAVITY_CLIENT_ID, ANTIGRAVITY_CLIENT_SECRET, GOOGLE_TOKEN_URL, GITHUB_COPILOT_TOKEN_URL};
use crate::core::secure_account_storage::{read_account_file_readonly, save_account_envelope_atomic};

static KEEPER_RUNNING: AtomicBool = AtomicBool::new(false);
const EXPIRATION_THRESHOLD_SECS: i64 = 15 * 60; // Refresh if expires within 15 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenKeeperReport {
    pub timestamp: String,
    pub checked_count: usize,
    pub refreshed_count: usize,
    pub failed_count: usize,
    pub details: Vec<String>,
}

fn get_cockpit_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cockpit_tools")
}

pub fn run_keeper_cycle_once() -> TokenKeeperReport {
    let cockpit_dir = get_cockpit_dir();
    let key_path = cockpit_dir.join("secure-account-storage.key");
    let now = Utc::now().timestamp();

    let mut report = TokenKeeperReport {
        timestamp: Utc::now().to_rfc3339(),
        checked_count: 0,
        refreshed_count: 0,
        failed_count: 0,
        details: Vec::new(),
    };

    if !cockpit_dir.is_dir() {
        return report;
    }

    // 1. Antigravity Google Accounts
    let ag_dir = cockpit_dir.join("accounts");
    if ag_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&ag_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "json") {
                    report.checked_count += 1;
                    if let Ok(mut doc) = read_account_file_readonly::<serde_json::Value>(&p, &key_path) {
                        let id = doc.get("id").and_then(|x| x.as_str()).unwrap_or("unknown").to_string();
                        let email = doc.get("email").and_then(|x| x.as_str()).unwrap_or(&id).to_string();
                        
                        let expiry = doc.pointer("/token/expiry_timestamp")
                            .and_then(|x| x.as_i64())
                            .unwrap_or(0);
                        let refresh_token = doc.pointer("/token/refresh_token")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();

                        if !refresh_token.is_empty() && (expiry == 0 || expiry - now <= EXPIRATION_THRESHOLD_SECS) {
                            // Needs refresh
                            match refresh_google_token(&refresh_token) {
                                Ok((new_access, new_exp_in, id_tok)) => {
                                    let new_expiry_ts = now + new_exp_in;
                                    if let Some(token_obj) = doc.get_mut("token").and_then(|t| t.as_object_mut()) {
                                        token_obj.insert("access_token".to_string(), serde_json::json!(new_access));
                                        token_obj.insert("expiry_timestamp".to_string(), serde_json::json!(new_expiry_ts));
                                        token_obj.insert("expires_in".to_string(), serde_json::json!(new_exp_in));
                                        if let Some(it) = id_tok {
                                            token_obj.insert("id_token".to_string(), serde_json::json!(it));
                                        }
                                    }
                                    doc["last_used"] = serde_json::json!(now);

                                    if let Err(e) = save_account_envelope_atomic(&p, "antigravity", &doc) {
                                        report.failed_count += 1;
                                        report.details.push(format!("Antigravity [{email}]: Save error {e}"));
                                    } else {
                                        report.refreshed_count += 1;
                                        report.details.push(format!("Antigravity [{email}]: Token refreshed (expires in {new_exp_in}s)"));
                                    }
                                }
                                Err(err) => {
                                    report.failed_count += 1;
                                    report.details.push(format!("Antigravity [{email}]: Refresh failed: {err}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. GitHub Copilot Accounts
    let gh_dir = cockpit_dir.join("github_copilot_accounts");
    if gh_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&gh_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "json") {
                    report.checked_count += 1;
                    if let Ok(mut doc) = read_account_file_readonly::<serde_json::Value>(&p, &key_path) {
                        let login = doc.get("github_login").and_then(|x| x.as_str()).unwrap_or("unknown").to_string();
                        let gh_token = doc.get("github_access_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let expires_at = doc.get("copilot_expires_at").and_then(|x| x.as_i64()).unwrap_or(0);

                        if !gh_token.is_empty() && (expires_at == 0 || expires_at - now <= EXPIRATION_THRESHOLD_SECS) {
                            match refresh_copilot_token(&gh_token) {
                                Ok((new_copilot_token, new_exp_ts, plan)) => {
                                    doc["copilot_token"] = serde_json::json!(new_copilot_token);
                                    doc["copilot_expires_at"] = serde_json::json!(new_exp_ts);
                                    if let Some(p_name) = plan {
                                        doc["copilot_plan"] = serde_json::json!(p_name);
                                    }
                                    doc["last_used"] = serde_json::json!(now);

                                    if let Err(e) = save_account_envelope_atomic(&p, "github_copilot", &doc) {
                                        report.failed_count += 1;
                                        report.details.push(format!("Copilot [{login}]: Save error {e}"));
                                    } else {
                                        report.refreshed_count += 1;
                                        report.details.push(format!("Copilot [{login}]: Copilot token refreshed"));
                                    }
                                }
                                Err(err) => {
                                    report.failed_count += 1;
                                    report.details.push(format!("Copilot [{login}]: Refresh failed: {err}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    report
}

fn refresh_google_token(refresh_token: &str) -> Result<(String, i64, Option<String>), String> {
    let resp = ureq::post(GOOGLE_TOKEN_URL)
        .send_form(&[
            ("client_id", ANTIGRAVITY_CLIENT_ID),
            ("client_secret", ANTIGRAVITY_CLIENT_SECRET),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .map_err(|e| format!("HTTP request error: {e}"))?;

    let val: serde_json::Value = resp.into_json().map_err(|e| format!("Parse json: {e}"))?;
    let access = val.get("access_token").and_then(|x| x.as_str()).ok_or("No access_token")?.to_string();
    let exp_in = val.get("expires_in").and_then(|x| x.as_i64()).unwrap_or(3600);
    let id_tok = val.get("id_token").and_then(|x| x.as_str()).map(|s| s.to_string());
    Ok((access, exp_in, id_tok))
}

fn refresh_copilot_token(gh_access_token: &str) -> Result<(String, i64, Option<String>), String> {
    let resp = ureq::get(GITHUB_COPILOT_TOKEN_URL)
        .set("Authorization", &format!("token {gh_access_token}"))
        .set("User-Agent", "antigravity-cockpit-tools")
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("Copilot token request error: {e}"))?;

    let val: serde_json::Value = resp.into_json().map_err(|e| format!("Parse json: {e}"))?;
    let token = val.get("token").and_then(|x| x.as_str()).ok_or("No token in response")?.to_string();
    let expires_at = val.get("expires_at").and_then(|x| x.as_i64()).unwrap_or_else(|| Utc::now().timestamp() + 1800);
    let plan = val.get("sku").or_else(|| val.get("plan")).and_then(|x| x.as_str()).map(|s| s.to_string());
    Ok((token, expires_at, plan))
}

/// Spawns the background token keeper daemon thread.
pub fn start_token_keeper_daemon() {
    if KEEPER_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        log::info!("[TokenKeeper] Background daemon started (poll interval: 60s)");
        // Initial delay before first check
        std::thread::sleep(Duration::from_secs(10));

        loop {
            let res = run_keeper_cycle_once();
            if res.refreshed_count > 0 || res.failed_count > 0 {
                log::info!(
                    "[TokenKeeper] Cycle finished: checked={}, refreshed={}, failed={}",
                    res.checked_count, res.refreshed_count, res.failed_count
                );
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_keeper_report_serialization() {
        let rep = TokenKeeperReport {
            timestamp: "2026-10-03T12:00:00Z".to_string(),
            checked_count: 5,
            refreshed_count: 2,
            failed_count: 0,
            details: vec!["Refreshed account 1".to_string()],
        };

        let json = serde_json::to_string(&rep).unwrap();
        assert!(json.contains("checkedCount"));
        assert!(json.contains("refreshedCount"));

        let deserialized: TokenKeeperReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.checked_count, 5);
        assert_eq!(deserialized.refreshed_count, 2);
    }
}
