use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakFinding {
    pub category: String, // "os_info", "ide_fingerprint", "sdk_tracking", "path_leak", "device_id"
    pub field: String,    // "header: User-Agent", "header: x-stainless-os", "body: prompt"
    pub value: String,
    pub severity: String, // "low", "medium", "high"
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintProfile {
    pub mode: String, // "chrome_windows", "chrome_macos", "curl", "raw", "custom" (nhãn UI, sanitize không đọc)
    pub custom_user_agent: Option<String>,
    pub strip_sdk_headers: bool,
    pub strip_ide_headers: bool,
    pub strip_sec_ch_ua: bool,
    pub remove_empty_headers: bool,
    pub mask_local_paths_in_body: bool,
    #[serde(default)]
    pub spoof_headers: HashMap<String, String>, // Custom headers to spoof / overwrite
}

impl Default for FingerprintProfile {
    fn default() -> Self {
        Self {
            mode: "chrome_windows".to_string(),
            custom_user_agent: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36".to_string()),
            strip_sdk_headers: true,
            strip_ide_headers: true,
            strip_sec_ch_ua: true,
            remove_empty_headers: true,
            mask_local_paths_in_body: true,
            spoof_headers: HashMap::new(),
        }
    }
}

// NOTE: struct FingerprintPool cũ đã xóa — GatewayConfig dùng trực tiếp
// `fingerprint_pool: Vec<FingerprintProfile>` + `active_fingerprint_index`.
