//! Antigravity Account storage, tags, notes, reorder, import & export.

use super::super::storage::read_cockpit_secure_json;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn get_antigravity_account_detail(
    cockpit_dir: &Path,
    account_id: &str,
    email: &str,
    _is_current: bool,
) -> (Option<String>, Option<serde_json::Value>) {
    let key_path = cockpit_dir.join("secure-account-storage.key");

    let mut detected_tier: Option<String> = None;
    let mut acc_quota: Option<serde_json::Value> = None;

    let computed_acc_file = if !account_id.is_empty() && cockpit_dir.join("accounts").join(format!("{account_id}.json")).exists() {
        Some(cockpit_dir.join("accounts").join(format!("{account_id}.json")))
    } else if !email.is_empty() {
        let md5_id = format!("antigravity_{:x}", md5::compute(email.trim().to_lowercase().as_bytes()));
        let p_md5 = cockpit_dir.join("accounts").join(format!("{md5_id}.json"));
        if p_md5.exists() {
            Some(p_md5)
        } else {
            let sha_id = format!("antigravity_{:x}", Sha256::digest(email.trim().to_lowercase().as_bytes()));
            let p_sha = cockpit_dir.join("accounts").join(format!("{sha_id}.json"));
            if p_sha.exists() {
                Some(p_sha)
            } else {
                None
            }
        }
    } else {
        None
    };

    let mut resolved_email = email.trim().to_lowercase();
    if let Some(acc_file) = computed_acc_file {
        if let Some(acc_val) = read_cockpit_secure_json(&acc_file, &key_path) {
            if let Some(em) = acc_val.get("email").and_then(|x| x.as_str()) {
                if !em.is_empty() {
                    resolved_email = em.trim().to_lowercase();
                }
            }
            if let Some(q) = acc_val.get("quota") {
                acc_quota = Some(q.clone());
                if let Some(tier) = q.get("subscription_tier").and_then(|t| t.as_str()) {
                    if !tier.is_empty() {
                        let t_up = tier.to_uppercase();
                        if t_up.contains("PRO") {
                            detected_tier = Some("G1-PRO-TIER".to_string());
                        } else if t_up.contains("ULTRA") {
                            detected_tier = Some("G1-ULTRA-TIER".to_string());
                        } else if t_up.contains("FREE") {
                            detected_tier = Some("FREE".to_string());
                        } else {
                            detected_tier = Some(t_up);
                        }
                    }
                }
                if detected_tier.is_none() {
                    if let Some(tier_id) = q.get("tier_id").and_then(|t| t.as_str()) {
                        if tier_id.to_lowercase().contains("pro") {
                            detected_tier = Some("G1-PRO-TIER".to_string());
                        } else if tier_id.to_lowercase().contains("ultra") {
                            detected_tier = Some("G1-ULTRA-TIER".to_string());
                        }
                    }
                }
            }
        }
    }

    let mut c_5h = None;
    let mut c_weekly = None;
    let mut g_5h = None;
    let mut g_weekly = None;
    let mut last_updated: Option<i64> = None;

    if let Some(ref q) = acc_quota {
        if let Some(models) = q.get("models").and_then(|m| m.as_array()) {
            for m in models {
                let name = m.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let pct = m.get("percentage").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
                let reset_time = m.get("reset_time").and_then(|x| x.as_str()).unwrap_or("");
                let time_left = super::super::storage::format_time_left(reset_time);
                let entry = serde_json::json!({
                    "percentage": pct,
                    "reset_time": reset_time,
                    "time_left": time_left
                });

                if name.contains("claude-3-5-sonnet") || name.contains("claude") {
                    if name.contains("5h") || c_5h.is_none() {
                        c_5h = Some(entry.clone());
                    }
                    if name.contains("weekly") || c_weekly.is_none() {
                        c_weekly = Some(entry);
                    }
                } else if name.contains("gemini-2.5-pro") || name.contains("gemini") {
                    if name.contains("5h") || g_5h.is_none() {
                        g_5h = Some(entry.clone());
                    }
                    if name.contains("weekly") || g_weekly.is_none() {
                        g_weekly = Some(entry);
                    }
                }
            }
        }
        if let Some(lu) = q.get("last_updated").and_then(|x| x.as_i64()) {
            last_updated = Some(lu);
        }
    }

    if (c_5h.is_none() || g_5h.is_none()) && !resolved_email.is_empty() {
        let email_sha256 = format!("{:x}", Sha256::digest(resolved_email.as_bytes()));
        let cache_file = cockpit_dir
            .join("cache")
            .join("quota_api_v1_desktop")
            .join("authorized")
            .join(format!("{}.json", email_sha256));

        if cache_file.exists() {
            if let Ok(c_str) = std::fs::read_to_string(&cache_file) {
                if let Ok(c_json) = serde_json::from_str::<serde_json::Value>(&c_str) {
                    if let Some(tier) = c_json.pointer("/payload/quota_summary/tier").and_then(|x| x.as_str()) {
                        if !tier.is_empty() && detected_tier.is_none() {
                            let t_up = tier.to_uppercase();
                            if t_up.contains("PRO") {
                                detected_tier = Some("G1-PRO-TIER".to_string());
                            } else if t_up.contains("ULTRA") {
                                detected_tier = Some("G1-ULTRA-TIER".to_string());
                            } else if t_up.contains("FREE") {
                                detected_tier = Some("FREE".to_string());
                            } else {
                                detected_tier = Some(t_up);
                            }
                        }
                    }

                    if let Some(groups) = c_json.pointer("/payload/quota_summary/groups").and_then(|x| x.as_array()) {
                        for grp in groups {
                            if let Some(buckets) = grp.get("buckets").and_then(|x| x.as_array()) {
                                for bkt in buckets {
                                    let b_id = bkt.get("bucketId").and_then(|x| x.as_str()).unwrap_or("");
                                    let frac = bkt.get("remainingFraction").and_then(|x| x.as_f64()).unwrap_or(0.0);
                                    let pct = (frac * 100.0).round() as i32;
                                    let reset_time = bkt.get("resetTime").and_then(|x| x.as_str()).unwrap_or("");
                                    let time_left = super::super::storage::format_time_left(reset_time);
                                    let entry = serde_json::json!({
                                        "percentage": pct,
                                        "reset_time": reset_time,
                                        "time_left": time_left
                                    });

                                    if b_id == "3p-5h" {
                                        c_5h = Some(entry);
                                    } else if b_id == "3p-weekly" {
                                        c_weekly = Some(entry);
                                    } else if b_id == "gemini-5h" {
                                        g_5h = Some(entry);
                                    } else if b_id == "gemini-weekly" {
                                        g_weekly = Some(entry);
                                    }
                                }
                            }
                        }
                    }

                    if last_updated.is_none() {
                        if let Some(fetched_at) = c_json.get("fetched_at").and_then(|x| x.as_i64()) {
                            last_updated = Some(fetched_at);
                        }
                    }
                }
            }
        }
    }

    let default_entry = serde_json::json!({
        "percentage": 100,
        "reset_time": "",
        "time_left": "5h"
    });
    let default_entry_w = serde_json::json!({
        "percentage": 100,
        "reset_time": "",
        "time_left": "7d"
    });

    let final_tier = detected_tier.unwrap_or_else(|| "G1-PRO-TIER".to_string());
    let details = serde_json::json!({
        "plan_tier": final_tier,
        "claude_5h": c_5h.unwrap_or(default_entry.clone()),
        "claude_weekly": c_weekly.unwrap_or(default_entry_w.clone()),
        "gemini_5h": g_5h.unwrap_or(default_entry),
        "gemini_weekly": g_weekly.unwrap_or(default_entry_w),
        "last_updated": last_updated.unwrap_or_else(|| chrono::Utc::now().timestamp())
    });

    (Some(final_tier), Some(details))
}

#[tauri::command]
pub fn update_account_tags(account_id: String, tags: Vec<String>) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");

    let acc_file = cockpit_dir.join("accounts").join(format!("{}.json", account_id));
    if acc_file.exists() && key_path.exists() {
        if let Some(mut val) = read_cockpit_secure_json(&acc_file, &key_path) {
            val["tags"] = serde_json::json!(tags);
            if let Ok(encrypted) = crate::core::secure_account_storage::serialize_account_file("antigravity", &val) {
                let _ = crate::core::secure_account_storage::write_string_atomic(&acc_file, &encrypted);
            }
        }
    }

    let platform_subdirs = [
        ("github_copilot_accounts", "github_copilot"),
        ("cursor_accounts", "cursor"),
        ("windsurf_accounts", "windsurf"),
        ("trae_accounts", "trae"),
        ("zed_accounts", "zed"),
        ("codebuddy_accounts", "codebuddy"),
        ("codebuddy_cn_accounts", "codebuddy_cn"),
        ("workbuddy_accounts", "workbuddy"),
        ("claude_accounts", "claude"),
        ("codex_accounts", "codex"),
        ("grok_accounts", "grok"),
        ("kiro_accounts", "kiro"),
        ("qoder_accounts", "qoder"),
        ("zcode_accounts", "zcode"),
    ];
    for (dir_name, kind) in platform_subdirs {
        let pf_file = cockpit_dir.join(dir_name).join(format!("{}.json", account_id));
        if pf_file.exists() && key_path.exists() {
            if let Some(mut val) = read_cockpit_secure_json(&pf_file, &key_path) {
                val["tags"] = serde_json::json!(tags);
                if let Ok(enc) = crate::core::secure_account_storage::serialize_account_file(kind, &val) {
                    let _ = crate::core::secure_account_storage::write_string_atomic(&pf_file, &enc);
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn update_account_notes(account_id: String, notes: String) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");

    let acc_file = cockpit_dir.join("accounts").join(format!("{}.json", account_id));
    if acc_file.exists() && key_path.exists() {
        if let Some(mut val) = read_cockpit_secure_json(&acc_file, &key_path) {
            val["notes"] = if notes.trim().is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(notes.trim())
            };
            if let Ok(encrypted) = crate::core::secure_account_storage::serialize_account_file("antigravity", &val) {
                let _ = crate::core::secure_account_storage::write_string_atomic(&acc_file, &encrypted);
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn update_account_note(
    account_id: String,
    note: Option<String>,
    two_factor_secret: Option<String>,
    account_password: Option<String>,
    phone_number: Option<String>,
    mail_url: Option<String>,
    aux_email: Option<String>,
) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");

    let acc_file = cockpit_dir.join("accounts").join(format!("{}.json", account_id));
    if acc_file.exists() && key_path.exists() {
        if let Some(mut val) = read_cockpit_secure_json(&acc_file, &key_path) {
            if let Some(n) = note {
                val["notes"] = if n.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(n.trim()) };
            }
            if let Some(s) = two_factor_secret {
                val["two_factor_secret"] = if s.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(s.trim()) };
            }
            if let Some(p) = account_password {
                val["account_password"] = if p.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(p.trim()) };
            }
            if let Some(ph) = phone_number {
                val["phone_number"] = if ph.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(ph.trim()) };
            }
            if let Some(m) = mail_url {
                val["mail_url"] = if m.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(m.trim()) };
            }
            if let Some(a) = aux_email {
                val["aux_email"] = if a.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(a.trim()) };
            }
            if let Ok(encrypted) = crate::core::secure_account_storage::serialize_account_file("antigravity", &val) {
                let _ = crate::core::secure_account_storage::write_string_atomic(&acc_file, &encrypted);
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn reorder_accounts(account_ids: Vec<String>) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let accounts_path = std::path::Path::new(&home).join(".cockpit_tools").join("accounts.json");
    if !accounts_path.exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(&accounts_path)
        .map_err(|e| format!("Failed to read accounts.json: {}", e))?;
    let mut v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse accounts.json: {}", e))?;

    if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
        let mut new_arr = Vec::new();
        for target_id in &account_ids {
            if let Some(found) = arr.iter().find(|item| {
                item.get("id").and_then(|x| x.as_str()) == Some(target_id)
            }) {
                new_arr.push(found.clone());
            }
        }
        for item in arr {
            if let Some(id) = item.get("id").and_then(|x| x.as_str()) {
                if !account_ids.iter().any(|target| target == id) {
                    new_arr.push(item.clone());
                }
            }
        }
        v["accounts"] = serde_json::Value::Array(new_arr);
        let serialized = serde_json::to_string_pretty(&v)
            .map_err(|e| format!("Failed to serialize accounts: {}", e))?;
        crate::core::secure_account_storage::write_string_atomic(&accounts_path, &serialized)?;
    }
    Ok(())
}

#[tauri::command]
pub fn export_accounts(account_ids: Vec<String>) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");

    let accounts_path = cockpit_dir.join("accounts.json");
    let raw = std::fs::read_to_string(&accounts_path)
        .map_err(|e| format!("Failed to read accounts.json: {}", e))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse accounts.json: {}", e))?;

    let mut exports = Vec::new();
    if let Some(arr) = v.get("accounts").and_then(|a| a.as_array()) {
        for item in arr {
            let id = item.get("id").and_then(|x| x.as_str()).unwrap_or("");
            if !account_ids.is_empty() && !account_ids.iter().any(|target| target == id) {
                continue;
            }
            let email = item.get("email").and_then(|x| x.as_str()).unwrap_or("");
            let detail_path = cockpit_dir.join("accounts").join(format!("{}.json", id));
            let detail_val = if detail_path.exists() && key_path.exists() {
                read_cockpit_secure_json(&detail_path, &key_path)
            } else {
                None
            };

            let refresh_token = detail_val.as_ref()
                .and_then(|d| d.get("token"))
                .and_then(|t| t.get("refresh_token"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let tags = detail_val.as_ref()
                .and_then(|d| d.get("tags"))
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            let notes = detail_val.as_ref().and_then(|d| d.get("notes")).cloned();

            let mut export_item = serde_json::json!({
                "email": email,
                "tags": tags,
            });
            if !refresh_token.is_empty() {
                export_item["refresh_token"] = serde_json::json!(refresh_token);
            }
            if let Some(n) = notes {
                if !n.is_null() {
                    export_item["notes"] = n;
                }
            }
            exports.push(export_item);
        }
    }
    serde_json::to_string_pretty(&exports).map_err(|e| format!("Failed to serialize export: {}", e))
}

#[tauri::command]
pub fn import_from_json(json_content: String) -> Result<usize, String> {
    let list: Vec<serde_json::Value> = serde_json::from_str(&json_content)
        .map_err(|e| format!("Invalid import JSON array: {}", e))?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let key_path = cockpit_dir.join("secure-account-storage.key");
    let _ = crate::core::secure_account_storage::read_or_create_key_at(&key_path);

    let accounts_path = cockpit_dir.join("accounts.json");
    let mut index_val = if let Ok(raw) = std::fs::read_to_string(&accounts_path) {
        serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| serde_json::json!({
            "version": 1,
            "accounts": [],
            "current_account_id": ""
        }))
    } else {
        serde_json::json!({
            "version": 1,
            "accounts": [],
            "current_account_id": ""
        })
    };

    let mut imported = 0;
    let now = chrono::Utc::now().timestamp();

    for item in list {
        let email = item.get("email").and_then(|x| x.as_str()).unwrap_or("");
        let refresh_token = item.get("refresh_token").and_then(|x| x.as_str()).unwrap_or("");
        if email.is_empty() {
            continue;
        }

        let norm_email = email.trim().to_lowercase();
        let md5_hash = format!("{:x}", md5::compute(norm_email.as_bytes()));
        let account_id = format!("antigravity_{}", md5_hash);

        // 1. Write detail file
        let detail = serde_json::json!({
            "id": account_id,
            "email": norm_email,
            "token": {
                "access_token": "",
                "refresh_token": refresh_token,
                "expires_in": 3600,
                "expiry_timestamp": now,
                "token_type": "Bearer"
            },
            "quota": null,
            "created_at": now,
            "last_used": now,
            "tags": item.get("tags").cloned().unwrap_or_else(|| serde_json::json!([])),
            "notes": item.get("notes").cloned().unwrap_or(serde_json::Value::Null)
        });

        if let Ok(encrypted) = crate::core::secure_account_storage::serialize_account_file("antigravity", &detail) {
            let detail_path = cockpit_dir.join("accounts").join(format!("{}.json", account_id));
            let _ = crate::core::secure_account_storage::write_string_atomic(&detail_path, &encrypted);
        }

        // 2. Update accounts.json
        if let Some(arr) = index_val.get_mut("accounts").and_then(|a| a.as_array_mut()) {
            if !arr.iter().any(|x| x.get("id").and_then(|i| i.as_str()) == Some(&account_id)) {
                arr.push(serde_json::json!({
                    "id": account_id,
                    "email": norm_email,
                    "name": norm_email,
                    "created_at": now,
                    "last_used": now
                }));
                imported += 1;
            }
        }
    }

    let _ = crate::core::secure_account_storage::write_string_atomic(
        &accounts_path,
        &serde_json::to_string_pretty(&index_val).unwrap_or_default(),
    );

    Ok(imported)
}
