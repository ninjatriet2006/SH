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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolAdapter {
    #[default]
    None,
    #[serde(rename = "openai_to_1min")]
    OpenAiTo1Min,
    #[serde(rename = "openai_to_anthropic")]
    OpenAiToAnthropic,
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
    /// Fingerprint profile riêng của endpoint (index vào `fingerprint_pool`).
    /// None = dùng global default (`get_active_fingerprint`). Serde default None
    /// nên config cũ thiếu field vẫn parse bình thường.
    #[serde(default)]
    pub fingerprint_index: Option<usize>,
    /// Giãn cách tối thiểu giữa 2 request khởi phát qua endpoint này (ms).
    /// 0 = dùng ngưỡng của tunnel. Effective = max(route, tunnel).
    #[serde(default)]
    pub min_request_interval_ms: u64,
    /// Bộ chuyển đổi giao thức (Protocol Conversion):
    /// - None: Direct Passthrough (giữ nguyên payload)
    /// - OpenAiTo1Min: OpenAI Completion <-> 1min.AI
    /// - OpenAiToAnthropic: OpenAI Completion <-> Anthropic Messages
    #[serde(default)]
    pub protocol_adapter: ProtocolAdapter,
}

fn default_max_disk_log_entries() -> usize {
    5000
}

fn default_max_key_failures() -> usize {
    3
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
    /// Số lần lỗi 401/403 liên tiếp tối đa của một key trước khi bị loại khỏi file
    /// (chống đốt key oan vì lỗi thoáng qua như WAF/rate-limit; chỉnh trong Settings).
    #[serde(default = "default_max_key_failures")]
    pub max_key_failures: usize,
}

/// Reset toàn bộ trạng thái sức khỏe runtime về chưa-xác-minh (Stale Boot Status).
/// Invariant: status/IP/latency/error/check-time KHÔNG được tin từ ổ cứng lúc boot —
/// process VPN đã chết từ phiên trước, worker cần vài giây mới xác minh lại.
/// Hàm pure, không đụng đĩa → unit-test được; `load_or_default` gọi sau deserialize.
/// Lệch khỏi spec gốc ở 2 điểm: (1) reset luôn last_checked_at (nếu không timestamp
/// cũ vẫn treo cạnh badge Unknown); (2) reset cả routes (Active/Inactive persisted
/// cũng stale y hệt, sync_listeners sẽ ghi trạng thái thật ngay sau bind).
pub fn reset_runtime_health(cfg: &mut GatewayConfig) {
    for t in &mut cfg.tunnels {
        t.status = if t.enabled {
            crate::vpn::TunnelStatus::Unknown
        } else {
            crate::vpn::TunnelStatus::Offline
        };
        t.last_exit_ip = None;
        t.last_latency_ms = None;
        t.last_error = None;
        t.last_checked_at = None;
    }
    for r in &mut cfg.routes {
        r.status = RouteStatus::Unknown;
        r.last_error = None;
    }
}

impl GatewayConfig {
    pub fn get_active_fingerprint(&self) -> FingerprintProfile {
        if !self.fingerprint_pool.is_empty() {
            self.fingerprint_pool[self.active_fingerprint_index % self.fingerprint_pool.len()].clone()
        } else {
            self.fingerprint_profile.clone()
        }
    }

    /// Fingerprint áp dụng cho 1 route: Some(i) + pool non-empty → pool[i % len],
    /// còn lại (None, pool rỗng) fallback về global default.
    /// Dùng modulo để index cũ/lệch sau khi xóa profile không bao giờ panic.
    pub fn get_route_fingerprint(&self, route: &RouteRule) -> FingerprintProfile {
        if let Some(i) = route.fingerprint_index {
            if !self.fingerprint_pool.is_empty() {
                return self.fingerprint_pool[i % self.fingerprint_pool.len()].clone();
            }
        }
        self.get_active_fingerprint()
    }

    /// Kẹp mọi index về tầm hợp lệ của pool hiện tại (persist gọn, UI khỏi modulo).
    /// Pool rỗng → giữ nguyên (getter đã fallback về global default an toàn).
    pub fn clamp_fingerprint_indices(&mut self) {
        if self.fingerprint_pool.is_empty() {
            return;
        }
        let len = self.fingerprint_pool.len();
        self.active_fingerprint_index %= len;
        for r in &mut self.routes {
            if let Some(i) = r.fingerprint_index {
                r.fingerprint_index = Some(i % len);
            }
        }
    }

    /// Thư mục config mới: ưu tiên `GATEWAY_FILTER_CONFIG_DIR` (portable),
    /// fallback `VPN_AI_PROXY_CONFIG_DIR` cũ để tương thích.
    pub fn config_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("GATEWAY_FILTER_CONFIG_DIR") {
            return PathBuf::from(dir);
        }
        if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
            return PathBuf::from(dir);
        }
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home)
                .join(".config")
                .join("gateway_filter");
            let _ = std::fs::create_dir_all(&path);
            return path;
        }
        PathBuf::from(".")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("gateway_filter_config.json")
    }

    /// Các đường dẫn config cũ (pre-rename) để tự migrate sang mới.
    fn legacy_config_paths() -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
            out.push(PathBuf::from(dir).join("config.json"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            out.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("vpn_ai_proxy_gui")
                    .join("config.json"),
            );
        }
        out.push(PathBuf::from("vpn_ai_proxy_config.json"));
        out
    }

    fn load_from_path(p: &std::path::Path) -> Option<Self> {
        let data = std::fs::read_to_string(p).ok()?;
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
                        map.entry("max_key_failures".to_string()).or_insert(serde_json::json!(3));
                    }
                }

                match serde_json::from_value::<GatewayConfig>(val) {
                    Ok(mut cfg) => {
                        for r in &mut cfg.routes {
                            r.key_manager.refresh_metadata();
                        }
                        cfg.clamp_fingerprint_indices();
                        reset_runtime_health(&mut cfg);
                        return Some(cfg);
                    }
                    Err(e) => {
                        eprintln!("Error deserializing migrated config: {}. Creating backup.", e);
                        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                        let fname = p
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "config.json".to_string());
                        let bak_path = p.with_file_name(format!("{}.bak.{}", fname, timestamp));
                        let _ = std::fs::rename(p, &bak_path);
                    }
                }
            }
            Err(e) => {
                eprintln!("Corrupt config file: {}. Creating backup.", e);
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let fname = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "config.json".to_string());
                let bak_path = p.with_file_name(format!("{}.bak.{}", fname, timestamp));
                let _ = std::fs::rename(p, &bak_path);
            }
        }
        None
    }

    pub fn load_or_default() -> Self {
        let p = Self::config_path();
        if p.exists() {
            if let Some(cfg) = Self::load_from_path(&p) {
                return cfg;
            }
        }
        // Migrate config cũ sang đường dẫn mới (chỉ khi file mới chưa có).
        for legacy in Self::legacy_config_paths() {
            if legacy == p || !legacy.exists() {
                continue;
            }
            if let Some(cfg) = Self::load_from_path(&legacy) {
                let _ = cfg.save_to_disk();
                return cfg;
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
            max_key_failures: 3,
        }
    }
}
