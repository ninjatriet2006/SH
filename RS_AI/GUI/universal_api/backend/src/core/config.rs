//! Gateway config — ported from Go cmd/server/config.go.

use serde::{Deserialize, Serialize};
use crate::core::external::ExternalConfig;

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
    /// External provider bridge (anti-api compatible). `#[serde(default)]`
    /// để config.json cũ (chưa có field) vẫn parse được — chống lỗi ngầm.
    /// File cũ có field lạ (vd `vpn` đã gỡ) vẫn parse: serde bỏ qua unknown fields.
    #[serde(default)]
    pub external: ExternalConfig,
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
            external: ExternalConfig::default(),
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
    /// Egress proxy duy nhất (trống = direct). Per-account routing đã gỡ.
    #[serde(default)]
    pub proxy_url: String,
    pub timeout_seconds: i32,
    pub header_timeout_seconds: i32,
    pub idle_timeout_seconds: i32,
    /// User-Agent toàn cục (trống = UA mặc định của client).
    #[serde(default)]
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
    let path = crate::core::paths::config_file_path();
    crate::core::paths::ensure_parent(&path)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, json).map_err(|e| format!("Write config error: {e}"))?;
    std::fs::rename(&temp, &path).map_err(|e| format!("Commit config error: {e}"))?;
    Ok(())
}

/// Vị trí file config do `paths::config_file_path()` quyết (legacy CWD hoặc
/// XDG) — save và boot load PHẢI dùng chung resolver, không hardcode.
///
/// Lịch sử lỗi ngầm:
/// 1. save ghi CWD nhưng boot không đọc lại → mất setting mỗi lần mở app.
/// 2. Mọi path tương đối theo CWD → chạy từ đâu data rớt ở đó.

/// Đọc config từ file. File thiếu field mới (vd `external`) vẫn parse
/// nhờ `#[serde(default)]` từng field; field lạ (vd `vpn` đã gỡ) bị bỏ qua.
/// nhờ `#[serde(default)]` từng field.
pub fn load_from_file(path: &str) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("Cannot read config: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid config: {e}"))
}

fn env_str(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Đọc env theo tên mới `UAPI_*`, fallback tên cũ `WB2A_*` (tương thích
/// docker-compose/script cũ sau đổi tên project).
fn env_str2(new_key: &str, legacy_suffix: &str) -> Option<String> {
    env_str(new_key).or_else(|| env_str(&format!("WB2A_{legacy_suffix}")))
}

fn env_int2(new_key: &str, legacy_suffix: &str) -> Option<i32> {
    env_str2(new_key, legacy_suffix)?.parse::<i32>().ok()
}

/// Ghi đè config bằng biến môi trường `UAPI_*` (mới), fallback `WB2A_*` (cũ) —
/// port Go `cmd/server/env.go`. Dùng cho Docker/compose: set env là chạy.
pub fn apply_env(config: &mut Config) {
    if let Some(v) = env_str2("UAPI_LISTEN", "LISTEN") { config.listen = v; }
    if let Some(v) = env_str2("UAPI_API_KEY", "API_KEY") { config.api_key = v; }
    if let Some(v) = env_str2("UAPI_AUTH_DIR", "AUTH_DIR") { config.auth_dir = v; }
    if let Some(v) = env_str2("UAPI_STATE_FILE", "STATE_FILE") { config.state_file = v; }
    if let Some(n) = env_int2("UAPI_MAX_BODY_MB", "MAX_BODY_MB") { config.server.max_body_mb = n; }
    if let Some(v) = env_str2("UAPI_SOFT_RATE", "SOFT_RATE") { config.cooldown.soft_rate = v; }
    if let Some(v) = env_str2("UAPI_SOFT_RATE_MAX", "SOFT_RATE_MAX") { config.cooldown.soft_rate_max = v; }
    if let Some(v) = env_str2("UAPI_PROXY_URL", "PROXY_URL") { config.upstream.proxy_url = v; }
    if let Some(n) = env_int2("UAPI_TIMEOUT_SECONDS", "TIMEOUT_SECONDS") { config.upstream.timeout_seconds = n; }
    if let Some(n) = env_int2("UAPI_HEADER_TIMEOUT_SECONDS", "HEADER_TIMEOUT_SECONDS") { config.upstream.header_timeout_seconds = n; }
    if let Some(n) = env_int2("UAPI_IDLE_TIMEOUT_SECONDS", "IDLE_TIMEOUT_SECONDS") { config.upstream.idle_timeout_seconds = n; }
    if let Some(v) = env_str2("UAPI_USER_AGENT", "USER_AGENT") { config.upstream.user_agent = v; }
    if let Some(v) = env_str2("UAPI_REALM", "REALM") { config.upstream.realm = v; }
    if let Some(v) = env_str2("UAPI_SANITIZE_FINGERPRINTS", "SANITIZE_FINGERPRINTS") {
        if let Ok(b) = v.parse::<bool>() { config.features.sanitize_blacklist_fingerprints = b; }
    }
    if let Some(v) = env_str2("UAPI_PROMPT_MODE", "PROMPT_MODE") { config.prompt.mode = v; }
    if let Some(v) = env_str2("UAPI_PROMPT_FILE", "PROMPT_FILE") { config.prompt.file = v; }
}

/// Config khởi động: file (nếu có) → env đè lên → mặc định.
pub fn load_boot_config() -> Config {
    let path = crate::core::paths::config_file_path();
    let mut cfg = load_from_file(&path.to_string_lossy()).unwrap_or_default();
    apply_env(&mut cfg);
    // Chuẩn hóa path tương đối thành tuyệt đối NGAY tại boot để mọi module
    // (pool state, sqlite, metrics, auths) trỏ cùng một chỗ dù CWD là gì.
    // Legacy (CWD có config.json): giữ nguyên theo CWD. Mới: gom về XDG.
    let base = crate::core::paths::data_base();
    cfg.auth_dir = crate::core::paths::resolve_data_path(&base, &cfg.auth_dir)
        .to_string_lossy()
        .to_string();
    cfg.state_file = crate::core::paths::resolve_data_path(&base, &cfg.state_file)
        .to_string_lossy()
        .to_string();
    cfg
}

/// Parse duration dạng Go: "600s" | "30m" | "6h" | "1d" | số giây trần.
/// Trả None khi sai format — caller dùng default thay vì panic.
pub fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(v) = t.strip_suffix('s') {
        (v, 1u64)
    } else if let Some(v) = t.strip_suffix('m') {
        (v, 60u64)
    } else if let Some(v) = t.strip_suffix('h') {
        (v, 3600u64)
    } else if let Some(v) = t.strip_suffix('d') {
        (v, 86400u64)
    } else {
        (t, 1u64)
    };
    num.trim().parse::<u64>().ok()?.checked_mul(mult).map(std::time::Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_units() {
        use std::time::Duration as D;
        assert_eq!(parse_duration("600s"), Some(D::from_secs(600)));
        assert_eq!(parse_duration("30m"), Some(D::from_secs(1800)));
        assert_eq!(parse_duration("6h"), Some(D::from_secs(21600)));
        assert_eq!(parse_duration("2h"), Some(D::from_secs(7200)));
        assert_eq!(parse_duration("120"), Some(D::from_secs(120)));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("10x"), None);
    }

    #[test]
    fn env_overrides_config() {        std::env::set_var("WB2A_LISTEN_TEST_X", ":9999");
        let mut cfg = Config::default();
        // Dùng key thật rồi dọn để không rò sang test khác.
        std::env::set_var("WB2A_LISTEN", ":9999");
        std::env::set_var("WB2A_MAX_BODY_MB", "16");
        apply_env(&mut cfg);
        assert_eq!(cfg.listen, ":9999");
        assert_eq!(cfg.server.max_body_mb, 16);
        std::env::remove_var("WB2A_LISTEN");
        std::env::remove_var("WB2A_MAX_BODY_MB");
        std::env::remove_var("WB2A_LISTEN_TEST_X");
    }

    #[test]
    fn default_config_round_trip_with_new_fields() {
        // config.json cũ thiếu `external` (hoặc thừa `vpn` đã gỡ) vẫn parse.
        let raw = r#"{"listen":":7863","api_key":"","auth_dir":"./auths","state_file":"./data/state.json","server":{"max_body_mb":8},"cooldown":{"soft_rate":"600s","soft_rate_max":"2h"},"schedule":{"checkin_hours":[9],"travel_hours":[9],"activity_hours":[10],"keepalive_hours":[22],"checkin_enabled":true,"travel_enabled":true,"activity_enabled":true,"keepalive_enabled":true},"upstream":{"proxy_url":"","timeout_seconds":120,"header_timeout_seconds":0,"idle_timeout_seconds":0,"user_agent":"","realm":""},"features":{"sanitize_blacklist_fingerprints":true},"prompt":{"mode":"custom","file":""},"pool":{"max_in_flight":3,"breaker_threshold":3,"breaker_cooldown":"30m","breaker_cooldown_max":"6h","idle_weight_per_hour":0.5,"idle_weight_max":5.0},"session_sticky":{"enabled":true,"ttl":"30m","gc_interval":"5m"},"vpn":{"enabled":false,"location":"","port":1080}}"#;
        let cfg: Config = serde_json::from_str(raw).expect("old config must parse");
        assert_eq!(cfg.listen, ":7863");
        assert!(!cfg.external.enabled);
    }
}
