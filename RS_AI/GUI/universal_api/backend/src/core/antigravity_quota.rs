//! Antigravity (Google Cloud Code) Quota Fetcher & Cache Manager.
//!
//! Directly interacts with Google Cloud Code Pa endpoints:
//! - v1internal:loadCodeAssist (determines tier, allowed tiers, credits, project ID)
//! - v1internal:fetchAvailableModels (model list & quota info)
//! - v1internal:retrieveUserQuotaSummary (Claude & Gemini quota pools: 5h and Weekly limits)
//!
//! Caches results into `~/.cockpit_tools/cache/quota_api_v1_desktop/authorized/{sha256}.json`
//! and updates encrypted account detail file `~/.cockpit_tools/accounts/{account_id}.json`
//! matching Cockpit Tools 100% on disk.

use std::path::Path;
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::core::oauth::{ANTIGRAVITY_CLIENT_ID, ANTIGRAVITY_CLIENT_SECRET, GOOGLE_TOKEN_URL};

pub const CLOUD_CODE_BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
pub const LOAD_CODE_ASSIST_UA: &str = "antigravity/1.20.5 linux/amd64 google-api-nodejs-client/10.3.0";
pub const CLOUD_CODE_UA: &str = "antigravity/1.20.5 linux/amd64";

/// Refresh Google OAuth access token using refresh token.
pub fn refresh_google_access_token(refresh_token: &str) -> Result<(String, i64, Option<String>), String> {
    let resp = ureq::post(GOOGLE_TOKEN_URL)
        .send_form(&[
            ("client_id", ANTIGRAVITY_CLIENT_ID),
            ("client_secret", ANTIGRAVITY_CLIENT_SECRET),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .map_err(|e| format!("Lỗi yêu cầu đổi refresh_token Google: {e}"))?;

    let val: Value = resp.into_json().map_err(|e| format!("Lỗi phân giải JSON Google Token: {e}"))?;
    let access = val.get("access_token").and_then(|x| x.as_str()).ok_or("Thiếu access_token từ Google")?.to_string();
    let exp_in = val.get("expires_in").and_then(|x| x.as_i64()).unwrap_or(3600);
    let id_tok = val.get("id_token").and_then(|x| x.as_str()).map(|s| s.to_string());
    Ok((access, exp_in, id_tok))
}

/// Fetch quota from Google Cloud Code Pa and write to both:
/// 1. ~/.cockpit_tools/cache/quota_api_v1_desktop/authorized/{sha256(email)}.json
/// 2. ~/.cockpit_tools/accounts/{account_id}.json (AES-256-GCM envelope)
pub fn fetch_and_save_antigravity_quota(
    cockpit_dir: &Path,
    account_id: &str,
    email: &str,
    access_token: &str,
) -> Result<Value, String> {
    // 1. loadCodeAssist
    let metadata = json!({
        "ideName": "antigravity",
        "ideType": "ANTIGRAVITY",
        "ideVersion": "1.20.5",
        "pluginVersion": "0.1.0",
        "platform": "LINUX_AMD64",
        "updateChannel": "stable",
        "pluginType": "GEMINI"
    });
    let load_payload = json!({
        "metadata": metadata,
        "mode": "FULL_ELIGIBILITY_CHECK"
    });

    let load_resp = ureq::post(&format!("{}/v1internal:loadCodeAssist", CLOUD_CODE_BASE_URL))
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Content-Type", "application/json")
        .set("User-Agent", LOAD_CODE_ASSIST_UA)
        .set("x-goog-api-client", "gl-node/22.21.1 gdcl/10.3.0")
        .send_json(load_payload)
        .map_err(|e| format!("loadCodeAssist thất bại: {e}"))?;

    let load_data: Value = load_resp.into_json().map_err(|e| format!("loadCodeAssist JSON lỗi: {e}"))?;

    let paid_tier_id = load_data.pointer("/paidTier/id").and_then(|x| x.as_str());
    let current_tier_id = load_data.pointer("/currentTier/id").and_then(|x| x.as_str());
    let detected_tier = paid_tier_id
        .or(current_tier_id)
        .unwrap_or("free-tier")
        .to_string();

    let project_id = load_data.pointer("/project/id")
        .or_else(|| load_data.get("project"))
        .and_then(|x| x.as_str());

    let mut normalized_credits = Vec::new();
    if let Some(arr) = load_data.pointer("/paidTier/availableCredits").and_then(|c| c.as_array()) {
        for c in arr {
            let c_type = c.get("creditType").or_else(|| c.get("credit_type")).and_then(|x| x.as_str());
            let c_amt = c.get("creditAmount").or_else(|| c.get("credit_amount")).and_then(|x| x.as_str());
            let min_amt = c.get("minimumCreditAmountForUsage").or_else(|| c.get("minimum_credit_amount_for_usage")).and_then(|x| x.as_str());
            if let Some(ct) = c_type {
                if let Some(amt) = c_amt {
                    normalized_credits.push(json!({
                        "credit_type": ct,
                        "credit_amount": amt,
                        "minimum_credit_amount_for_usage": min_amt
                    }));
                }
            }
        }
    }

    // 2. fetchAvailableModels
    let models_payload = if let Some(pid) = project_id {
        json!({ "project": pid })
    } else {
        json!({})
    };

    let models_resp = ureq::post(&format!("{}/v1internal:fetchAvailableModels", CLOUD_CODE_BASE_URL))
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Content-Type", "application/json")
        .set("User-Agent", CLOUD_CODE_UA)
        .send_json(&models_payload);

    let mut cache_payload = if let Ok(resp) = models_resp {
        resp.into_json::<Value>().unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    // 3. retrieveUserQuotaSummary
    let summary_resp = ureq::post(&format!("{}/v1internal:retrieveUserQuotaSummary", CLOUD_CODE_BASE_URL))
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Content-Type", "application/json")
        .set("User-Agent", CLOUD_CODE_UA)
        .send_json(&models_payload);

    let summary_val: Option<Value> = if let Ok(resp) = summary_resp {
        resp.into_json::<Value>().ok()
    } else {
        None
    };

    if let Some(ref s_val) = summary_val {
        if let Some(obj) = cache_payload.as_object_mut() {
            obj.insert("quota_summary".to_string(), s_val.clone());
        }
    }

    // 4. Build models array for account.quota
    let mut quota_models = Vec::new();
    if let Some(models_obj) = cache_payload.get("models").and_then(|m| m.as_object()) {
        for (m_name, info) in models_obj {
            let display_name = info.get("displayName").and_then(|d| d.as_str());
            if let Some(qi) = info.get("quotaInfo") {
                let rf = qi.get("remainingFraction").and_then(|f| f.as_f64()).unwrap_or(0.0);
                let percentage = (rf * 100.0).round() as i64;
                let reset_time = qi.get("resetTime").and_then(|t| t.as_str()).unwrap_or("");
                if m_name.contains("gemini") || m_name.contains("claude") {
                    quota_models.push(json!({
                        "name": m_name,
                        "display_name": display_name,
                        "percentage": percentage,
                        "reset_time": reset_time
                    }));
                }
            }
        }
    }

    // Add buckets from summary
    if let Some(summary) = &summary_val {
        if let Some(groups) = summary.get("groups").and_then(|g| g.as_array()) {
            for group in groups {
                if let Some(buckets) = group.get("buckets").and_then(|b| b.as_array()) {
                    for bucket in buckets {
                        if let (Some(id), Some(fraction)) = (
                            bucket.get("bucketId").and_then(|x| x.as_str()),
                            bucket.get("remainingFraction").and_then(|x| x.as_f64())
                        ) {
                            let percentage = (fraction * 100.0).round() as i64;
                            let reset_time = bucket.get("resetTime").and_then(|x| x.as_str()).unwrap_or("");
                            let display_name = bucket.get("displayName").and_then(|x| x.as_str());
                            quota_models.push(json!({
                                "name": id,
                                "display_name": display_name,
                                "percentage": percentage,
                                "reset_time": reset_time
                            }));
                        }
                    }
                }
            }
        }
    }

    let now = Utc::now().timestamp();
    let quota_obj = json!({
        "subscription_tier": detected_tier,
        "models": quota_models,
        "credits": normalized_credits,
        "last_updated": now,
        "quota_summary_stale": false,
        "quota_summary_updated_at": now,
        "is_forbidden": false,
        "tier_id": null,
        "is_gcp_tos": null,
        "project_id": project_id
    });

    // 5. Write cache file: ~/.cockpit_tools/cache/quota_api_v1_desktop/authorized/{sha256(email)}.json
    let hash = format!("{:x}", Sha256::digest(email.trim().to_lowercase().as_bytes()));
    let cache_dir = cockpit_dir.join("cache/quota_api_v1_desktop/authorized");
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache_file = cache_dir.join(format!("{hash}.json"));
    let cache_envelope = json!({
        "version": 1,
        "source": "authorized",
        "customSource": "desktop",
        "email": email,
        "projectId": project_id,
        "updatedAt": now,
        "payload": cache_payload
    });
    if let Ok(c_str) = serde_json::to_string_pretty(&cache_envelope) {
        let _ = std::fs::write(&cache_file, c_str);
    }

    // 6. Update encrypted account doc: ~/.cockpit_tools/accounts/{account_id}.json
    let key_path = cockpit_dir.join("secure-account-storage.key");
    let acc_file = cockpit_dir.join("accounts").join(format!("{account_id}.json"));
    if let Ok(mut doc) = crate::core::secure_account_storage::read_account_file_readonly::<Value>(&acc_file, &key_path) {
        if let Some(obj) = doc.as_object_mut() {
            obj.insert("quota".to_string(), quota_obj.clone());
            obj.insert("last_used".to_string(), json!(now));
        }
        let _ = crate::core::secure_account_storage::save_account_envelope_atomic(&acc_file, "antigravity", &doc);
    }

    Ok(quota_obj)
}

/// Refresh quota on demand for an account, refreshing access token if needed.
pub fn refresh_antigravity_account_quota(
    cockpit_dir: &Path,
    account_id: &str,
) -> Result<Value, String> {
    let key_path = cockpit_dir.join("secure-account-storage.key");
    let acc_file = if cockpit_dir.join("accounts").join(format!("{account_id}.json")).exists() {
        cockpit_dir.join("accounts").join(format!("{account_id}.json"))
    } else if account_id.contains('@') {
        let hash = format!("antigravity_{:x}", Sha256::digest(account_id.trim().to_lowercase().as_bytes()));
        cockpit_dir.join("accounts").join(format!("{hash}.json"))
    } else {
        cockpit_dir.join("accounts").join(format!("{account_id}.json"))
    };
    let mut doc = crate::core::secure_account_storage::read_account_file_readonly::<Value>(&acc_file, &key_path)
        .map_err(|e| format!("Không thể đọc tệp tài khoản {account_id}: {e}"))?;

    let email = doc.get("email").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let actual_account_id = doc.get("id").and_then(|x| x.as_str()).unwrap_or(account_id).to_string();
    let mut access_token = doc.pointer("/token/access_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let refresh_token = doc.pointer("/token/refresh_token").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let expiry = doc.pointer("/token/expiry_timestamp").and_then(|x| x.as_i64()).unwrap_or(0);
    let now = Utc::now().timestamp();

    // If expired or expiring in <= 60s, refresh token first
    if !refresh_token.is_empty() && (expiry == 0 || expiry - now <= 60) {
        if let Ok((new_acc, new_exp, id_tok)) = refresh_google_access_token(&refresh_token) {
            access_token = new_acc.clone();
            if let Some(t_obj) = doc.get_mut("token").and_then(|t| t.as_object_mut()) {
                t_obj.insert("access_token".to_string(), json!(new_acc));
                t_obj.insert("expiry_timestamp".to_string(), json!(now + new_exp));
                t_obj.insert("expires_in".to_string(), json!(new_exp));
                if let Some(it) = id_tok {
                    t_obj.insert("id_token".to_string(), json!(it));
                }
            }
            let _ = crate::core::secure_account_storage::save_account_envelope_atomic(&acc_file, "antigravity", &doc);
        }
    }

    let res = fetch_and_save_antigravity_quota(cockpit_dir, &actual_account_id, &email, &access_token);
    if res.is_err() && !refresh_token.is_empty() {
        // Try refreshing token once and retry
        if let Ok((new_acc, new_exp, id_tok)) = refresh_google_access_token(&refresh_token) {
            access_token = new_acc.clone();
            if let Some(t_obj) = doc.get_mut("token").and_then(|t| t.as_object_mut()) {
                t_obj.insert("access_token".to_string(), json!(new_acc));
                t_obj.insert("expiry_timestamp".to_string(), json!(now + new_exp));
                t_obj.insert("expires_in".to_string(), json!(new_exp));
                if let Some(it) = id_tok {
                    t_obj.insert("id_token".to_string(), json!(it));
                }
            }
            let _ = crate::core::secure_account_storage::save_account_envelope_atomic(&acc_file, "antigravity", &doc);
            return fetch_and_save_antigravity_quota(cockpit_dir, &actual_account_id, &email, &access_token);
        }
    }
    res
}
