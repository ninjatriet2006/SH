//! Auto Check-in System for Tencent Codebuddy & Workbuddy.
//!
//! Automatically claims daily free credits from Tencent Codebuddy / Workbuddy
//! endpoints and tracks check-in status and logs compatible with Cockpit Tools.

use std::fs;
use std::path::{Path, PathBuf};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::core::secure_account_storage::write_string_atomic;

const CHECKIN_STATUS_URL: &str = "https://www.codebuddy.cn/v2/billing/meter/checkin-activity-status";
const DAILY_CHECKIN_URL: &str = "https://www.codebuddy.cn/v2/billing/meter/daily-checkin";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCheckinConfig {
    pub enabled: bool,
    pub last_checked_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCheckinLogRecord {
    pub id: String,
    pub timestamp: String,
    pub date: String,
    pub account_id: String,
    pub email: String,
    pub status: String, // "success", "already_checked", "failed", "inactive"
    pub message: String,
    pub credit_awarded: Option<i64>,
}

fn get_cockpit_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cockpit_tools")
}

fn config_path() -> PathBuf {
    get_cockpit_dir().join("workbuddy_auto_checkin_config.json")
}

fn logs_path() -> PathBuf {
    get_cockpit_dir().join("workbuddy_auto_checkin_logs.json")
}

pub fn load_auto_checkin_config() -> AutoCheckinConfig {
    let p = config_path();
    if let Ok(raw) = fs::read_to_string(&p) {
        if let Ok(cfg) = serde_json::from_str::<AutoCheckinConfig>(&raw) {
            return cfg;
        }
    }
    AutoCheckinConfig {
        enabled: true,
        last_checked_date: None,
    }
}

pub fn save_auto_checkin_config(cfg: &AutoCheckinConfig) -> Result<(), String> {
    let p = config_path();
    let content = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    write_string_atomic(&p, &content)
}

pub fn load_auto_checkin_logs() -> Vec<AutoCheckinLogRecord> {
    let p = logs_path();
    if let Ok(raw) = fs::read_to_string(&p) {
        if let Ok(logs) = serde_json::from_str::<Vec<AutoCheckinLogRecord>>(&raw) {
            return logs;
        }
    }
    Vec::new()
}

pub fn append_auto_checkin_log(log: AutoCheckinLogRecord) {
    let mut logs = load_auto_checkin_logs();
    logs.insert(0, log);
    if logs.len() > 100 {
        logs.truncate(100);
    }
    let p = logs_path();
    if let Ok(content) = serde_json::to_string_pretty(&logs) {
        let _ = write_string_atomic(&p, &content);
    }
}

pub struct CheckinCandidate {
    pub id: String,
    pub email: String,
    pub access_token: String,
    pub uid: Option<String>,
    pub enterprise_id: Option<String>,
}

fn collect_checkin_candidates() -> Vec<CheckinCandidate> {
    let cockpit_dir = get_cockpit_dir();
    let mut candidates = Vec::new();

    let files = [
        "codebuddy_accounts.json",
        "codebuddy_cn_accounts.json",
        "workbuddy_accounts.json",
    ];

    for file_name in files {
        let p = cockpit_dir.join(file_name);
        if let Ok(raw) = fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
                    for item in arr {
                        let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        if id.is_empty() {
                            continue;
                        }
                        let email = item.get("email").or_else(|| item.get("nickname")).and_then(|x| x.as_str()).unwrap_or(&id).to_string();
                        let token = item.get("access_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
                        let uid = item.get("uid").and_then(|x| x.as_str()).map(|s| s.to_string());
                        let eid = item.get("enterprise_id").and_then(|x| x.as_str()).map(|s| s.to_string());

                        if !token.is_empty() {
                            candidates.push(CheckinCandidate {
                                id,
                                email,
                                access_token: token,
                                uid,
                                enterprise_id: eid,
                            });
                        }
                    }
                }
            }
        }
    }

    candidates
}

pub fn execute_checkin_cycle() -> Result<Vec<AutoCheckinLogRecord>, String> {
    let candidates = collect_checkin_candidates();
    let today = Local::now().format("%Y-%m-%d").to_string();
    let now_ts = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut generated_logs = Vec::new();

    for cand in candidates {
        let result = perform_single_checkin(&cand);
        let log_rec = match result {
            Ok((status, msg, credit)) => AutoCheckinLogRecord {
                id: format!("chk_{}_{}", Local::now().timestamp_millis(), rand::random::<u16>()),
                timestamp: now_ts.clone(),
                date: today.clone(),
                account_id: cand.id.clone(),
                email: cand.email.clone(),
                status,
                message: msg,
                credit_awarded: credit,
            },
            Err(err) => AutoCheckinLogRecord {
                id: format!("chk_{}_{}", Local::now().timestamp_millis(), rand::random::<u16>()),
                timestamp: now_ts.clone(),
                date: today.clone(),
                account_id: cand.id.clone(),
                email: cand.email.clone(),
                status: "failed".to_string(),
                message: err,
                credit_awarded: None,
            },
        };

        append_auto_checkin_log(log_rec.clone());
        generated_logs.push(log_rec);
    }

    // Update config last checked
    let mut cfg = load_auto_checkin_config();
    cfg.last_checked_date = Some(today);
    let _ = save_auto_checkin_config(&cfg);

    Ok(generated_logs)
}

fn perform_single_checkin(cand: &CheckinCandidate) -> Result<(String, String, Option<i64>), String> {
    // 1. Check status first
    let mut req = ureq::post(CHECKIN_STATUS_URL)
        .set("Authorization", &format!("Bearer {}", cand.access_token))
        .set("Content-Type", "application/json")
        .set("Accept", "application/json");

    if let Some(ref u) = cand.uid {
        req = req.set("X-User-Id", u);
    }
    if let Some(ref eid) = cand.enterprise_id {
        req = req.set("X-Enterprise-Id", eid);
        req = req.set("X-Tenant-Id", eid);
    }

    let status_res = req.send_json(serde_json::json!({}));
    if let Ok(res) = status_res {
        if let Ok(v) = res.into_json::<serde_json::Value>() {
            let code = v.get("code").and_then(|x| x.as_i64()).unwrap_or(-1);
            if code == 0 {
                if let Some(data) = v.get("data") {
                    let already = data.get("today_checked_in")
                        .or_else(|| data.get("todayCheckedIn"))
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    if already {
                        let daily = data.get("daily_credit").and_then(|x| x.as_i64());
                        return Ok(("already_checked".to_string(), "Hôm nay đã hoàn thành điểm danh".to_string(), daily));
                    }
                }
            }
        }
    }

    // 2. Perform daily check-in
    let mut chk_req = ureq::post(DAILY_CHECKIN_URL)
        .set("Authorization", &format!("Bearer {}", cand.access_token))
        .set("Content-Type", "application/json")
        .set("Accept", "application/json");

    if let Some(ref u) = cand.uid {
        chk_req = chk_req.set("X-User-Id", u);
    }
    if let Some(ref eid) = cand.enterprise_id {
        chk_req = chk_req.set("X-Enterprise-Id", eid);
        chk_req = chk_req.set("X-Tenant-Id", eid);
    }

    let chk_res = chk_req.send_json(serde_json::json!({})).map_err(|e| format!("Daily check-in request error: {e}"))?;
    let chk_val: serde_json::Value = chk_res.into_json().map_err(|e| format!("Parse json: {e}"))?;

    let code = chk_val.get("code").and_then(|x| x.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = chk_val.get("message").or_else(|| chk_val.get("msg")).and_then(|x| x.as_str()).unwrap_or("Failed");
        return Err(format!("Lỗi điểm danh (code={code}): {msg}"));
    }

    let credit = chk_val.pointer("/data/credit")
        .or_else(|| chk_val.pointer("/data/daily_credit"))
        .and_then(|x| x.as_i64());

    Ok(("success".to_string(), "Điểm danh nhận credit thành công!".to_string(), credit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_checkin_config_and_logs_serialization() {
        let config = AutoCheckinConfig {
            enabled: true,
            last_checked_date: Some("2026-10-03".to_string()),
        };
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("lastCheckedDate"));
        let deserialized: AutoCheckinConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.enabled, true);
        assert_eq!(deserialized.last_checked_date.as_deref(), Some("2026-10-03"));

        let log = AutoCheckinLogRecord {
            id: "chk-123".to_string(),
            timestamp: "2026-10-03T08:00:00Z".to_string(),
            date: "2026-10-03".to_string(),
            account_id: "acc_456".to_string(),
            email: "user@example.com".to_string(),
            status: "success".to_string(),
            message: "Điểm danh thành công".to_string(),
            credit_awarded: Some(10),
        };
        let log_json = serde_json::to_string(&log).unwrap();
        assert!(log_json.contains("creditAwarded"));
        let log_back: AutoCheckinLogRecord = serde_json::from_str(&log_json).unwrap();
        assert_eq!(log_back.credit_awarded, Some(10));
    }
}
