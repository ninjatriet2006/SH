//! Gateway config — ported from Go cmd/server/config.go.

use serde::{Deserialize, Serialize};
use crate::core::vpn::VpnConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen: String,
    pub api_key: String,
    pub auth_dir: String,
    pub state_file: String,
    pub server: ServerConfig,
    pub cooldown: CooldownConfig,
    pub schedule: ScheduleConfig,
    pub upstream: UpstreamConfig,
    pub features: FeaturesConfig,
    pub prompt: PromptConfig,
    pub pool: PoolConfig,
    pub session_sticky: SessionStickyConfig,
    pub vpn: VpnConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: ":7863".to_string(),
            api_key: String::new(),
            auth_dir: "./auths".to_string(),
            state_file: "./data/state.json".to_string(),
            server: ServerConfig { max_body_mb: 8 },
            cooldown: CooldownConfig {
                soft_rate: "600s".to_string(),
                soft_rate_max: "2h".to_string(),
            },
            schedule: ScheduleConfig {
                checkin_hours: vec![9, 21],
                travel_hours: vec![9, 21],
                activity_hours: vec![10],
                keepalive_hours: vec![22],
                checkin_enabled: true,
                travel_enabled: true,
                activity_enabled: true,
                keepalive_enabled: true,
            },
            upstream: UpstreamConfig {
                proxy_url: String::new(),
                timeout_seconds: 120,
                header_timeout_seconds: 0,
                idle_timeout_seconds: 0,
                user_agent: String::new(),
                realm: String::new(),
            },
            features: FeaturesConfig {
                sanitize_blacklist_fingerprints: true,
            },
            prompt: PromptConfig {
                mode: "custom".to_string(),
                file: String::new(),
            },
            pool: PoolConfig {
                max_in_flight: 3,
                breaker_threshold: 3,
                breaker_cooldown: "30m".to_string(),
                breaker_cooldown_max: "6h".to_string(),
                idle_weight_per_hour: 0.5,
                idle_weight_max: 5.0,
            },
            session_sticky: SessionStickyConfig {
                enabled: true,
                ttl: "30m".to_string(),
                gc_interval: "5m".to_string(),
            },
            vpn: VpnConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub max_body_mb: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownConfig {
    pub soft_rate: String,
    pub soft_rate_max: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub checkin_hours: Vec<i32>,
    pub travel_hours: Vec<i32>,
    pub activity_hours: Vec<i32>,
    pub keepalive_hours: Vec<i32>,
    pub checkin_enabled: bool,
    pub travel_enabled: bool,
    pub activity_enabled: bool,
    pub keepalive_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub proxy_url: String,
    pub timeout_seconds: i32,
    pub header_timeout_seconds: i32,
    pub idle_timeout_seconds: i32,
    pub user_agent: String,
    pub realm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub sanitize_blacklist_fingerprints: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub mode: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub max_in_flight: i32,
    pub breaker_threshold: i32,
    pub breaker_cooldown: String,
    pub breaker_cooldown_max: String,
    pub idle_weight_per_hour: f64,
    pub idle_weight_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStickyConfig {
    pub enabled: bool,
    pub ttl: String,
    pub gc_interval: String,
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Serialize error: {e}"))?;
    let path = std::path::Path::new("config.json");
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, json).map_err(|e| format!("Write config error: {e}"))?;
    std::fs::rename(&temp, path).map_err(|e| format!("Commit config error: {e}"))?;
    Ok(())
}
