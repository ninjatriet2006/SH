use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::fingerprint::FingerprintProfile;
use crate::proxy::key_manager::EndpointKeyManager;
use crate::vpn::OutboundTunnel;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteStatus {
    Active,
    Inactive,
    Unknown,
}

impl Default for RouteStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,
    pub name: String,
    pub port: u16,              // Cổng lắng nghe riêng (VD: 3000, 3001, 3002...)
    pub path_prefix: String,     // Prefix ví dụ "/v1", "/mirror", "/"
    pub target_base_url: String, // Domain muốn chuyển tiếp: "https://abc.xyz/v1", "https://api.openai.com/v1"
    pub tunnel_id: String,       // Gán với OutboundTunnel ID nào (AdGuard SOCKS, WireGuard, Direct...)
    pub enabled: bool,
    #[serde(default)]
    pub status: RouteStatus,
    #[serde(default)]
    pub last_error: Option<String>,
    pub key_manager: EndpointKeyManager, // Quản lý file key riêng biệt cho endpoint này
    pub custom_auth_token: Option<String>,
}

fn default_max_disk_log_entries() -> usize {
    5000
}

fn default_config_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    pub tunnels: Vec<OutboundTunnel>,
    pub routes: Vec<RouteRule>,
    pub fingerprint_profile: FingerprintProfile,
    #[serde(default)]
    pub fingerprint_pool: Vec<FingerprintProfile>,
    #[serde(default)]
    pub active_fingerprint_index: usize,
    pub max_log_entries: usize,
    #[serde(default = "default_max_disk_log_entries")]
    pub max_disk_log_entries: usize,
}

impl GatewayConfig {
    pub fn get_active_fingerprint(&self) -> FingerprintProfile {
        if !self.fingerprint_pool.is_empty() {
            self.fingerprint_pool[self.active_fingerprint_index % self.fingerprint_pool.len()].clone()
        } else {
            self.fingerprint_profile.clone()
        }
    }

    pub fn config_path() -> PathBuf {
        if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
            return PathBuf::from(dir).join("config.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home)
                .join(".config")
                .join("vpn_ai_proxy_gui");
            let _ = std::fs::create_dir_all(&path);
            return path.join("config.json");
        }
        PathBuf::from("vpn_ai_proxy_config.json")
    }

    pub fn load_or_default() -> Self {
        let p = Self::config_path();
        if p.exists() {
            if let Ok(data) = std::fs::read_to_string(&p) {
                // Auto-Migration Pipeline: Parse thành Value để kiểm tra version
                match serde_json::from_str::<serde_json::Value>(&data) {
                    Ok(mut val) => {
                        let version = val.get("config_version").and_then(|v| v.as_u64()).unwrap_or(0);
                        if version < 1 {
                            // Cập nhật schema lên v1
                            if let Some(map) = val.as_object_mut() {
                                map.entry("config_version".to_string()).or_insert(serde_json::json!(1));
                                map.entry("fingerprint_pool".to_string()).or_insert(serde_json::json!([]));
                                map.entry("active_fingerprint_index".to_string()).or_insert(serde_json::json!(0));
                                map.entry("max_disk_log_entries".to_string()).or_insert(serde_json::json!(5000));
                            }
                        }

                        match serde_json::from_value::<GatewayConfig>(val) {
                            Ok(mut cfg) => {
                                for r in &mut cfg.routes {
                                    r.key_manager.refresh_metadata();
                                }
                                return cfg;
                            }
                            Err(e) => {
                                eprintln!("Error deserializing migrated config: {}. Creating backup.", e);
                                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                                let bak_path = p.with_file_name(format!("config.json.bak.{}", timestamp));
                                let _ = std::fs::rename(&p, &bak_path);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Corrupt config file: {}. Creating backup.", e);
                        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                        let bak_path = p.with_file_name(format!("config.json.bak.{}", timestamp));
                        let _ = std::fs::rename(&p, &bak_path);
                    }
                }
            }
        }
        let mut def = Self::default();
        for r in &mut def.routes {
            r.key_manager.refresh_metadata();
        }
        def
    }

    pub fn save_to_disk(&self) -> Result<(), String> {
        let p = Self::config_path();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&p, json)
            .map_err(|e| format!("Failed to write config file {:?}: {}", p, e))?;
        Ok(())
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            config_version: 1,
            tunnels: vec![],
            routes: vec![],
            fingerprint_profile: FingerprintProfile::default(),
            fingerprint_pool: vec![],
            active_fingerprint_index: 0,
            max_log_entries: 500,
            max_disk_log_entries: 5000,
        }
    }
}
