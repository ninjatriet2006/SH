use std::collections::HashMap;

use super::types::FingerprintProfile;
use crate::fingerprint::FingerprintAnalyzer;

impl FingerprintAnalyzer {
    /// Dọn dẹp headers theo profile: strip SDK/IDE/Sec-CH-UA headers,
    /// bỏ header rỗng, thay User-Agent và áp dụng spoof headers.
    pub fn sanitize_headers(
        headers: &HashMap<String, String>,
        profile: &FingerprintProfile,
    ) -> HashMap<String, String> {
        let mut clean = HashMap::new();

        for (k, v) in headers {
            let key_lower = k.to_lowercase();

            // Strip SDK
            if profile.strip_sdk_headers && (key_lower.starts_with("x-stainless-") || key_lower.starts_with("anthropic-client-")) {
                continue;
            }

            // Strip IDE
            if profile.strip_ide_headers
                && (key_lower.contains("cursor")
                    || key_lower.contains("vscode")
                    || key_lower.contains("machine-id")
                    || key_lower.contains("session-id"))
            {
                continue;
            }

            // Strip Sec-CH-UA
            if profile.strip_sec_ch_ua && key_lower.starts_with("sec-ch-ua") {
                continue;
            }

            // Skip empty
            if profile.remove_empty_headers && v.trim().is_empty() {
                continue;
            }

            // User-Agent transform
            if key_lower == "user-agent" {
                if let Some(ref ua) = profile.custom_user_agent {
                    clean.insert("user-agent".to_string(), ua.clone());
                    continue;
                }
            }

            clean.insert(key_lower, v.clone());
        }

        // Ensure User-Agent is set if custom_user_agent exists and wasn't in original headers
        if let Some(ref ua) = profile.custom_user_agent {
            clean.entry("user-agent".to_string()).or_insert_with(|| ua.clone());
        }

        // Spoof headers: overwrite or remove empty/stripped headers
        for (spoof_k, spoof_v) in &profile.spoof_headers {
            let k_lower = spoof_k.to_lowercase();
            if !spoof_v.is_empty() {
                clean.insert(k_lower, spoof_v.clone());
            } else {
                clean.remove(&k_lower);
            }
        }

        clean
    }
}
