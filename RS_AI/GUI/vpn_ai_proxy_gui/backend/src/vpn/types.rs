use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelProtocol {
    #[serde(rename = "socks5")]
    Socks5,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "direct")]
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelStatus {
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for TunnelStatus {
    fn default() -> Self {
        TunnelStatus::Unknown
    }
}

pub fn default_concurrency_limit() -> usize {
    0
}

/// Timeout mặc định cho lệnh CLI tunnel — chống deadlock khi tiến trình con bị kẹt (VD: prompt Y/N).
pub const TUNNEL_CMD_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundTunnel {
    pub id: String,
    pub name: String,
    pub protocol: TunnelProtocol,
    pub endpoint: String, // e.g. "127.0.0.1:1080" or "socks5h://127.0.0.1:1080"
    pub enabled: bool,
    #[serde(default)]
    pub status: TunnelStatus,
    #[serde(default = "default_concurrency_limit")]
    pub max_concurrent_streams: usize, // 0 = unlimited
    #[serde(default)]
    pub start_command: Option<String>,
    #[serde(default)]
    pub stop_command: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub last_exit_ip: Option<String>,
    pub last_latency_ms: Option<u64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelTestResult {
    pub tunnel_id: String,
    pub success: bool,
    pub exit_ip: Option<String>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}
