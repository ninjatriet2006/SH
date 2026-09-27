use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelProtocol {
    #[serde(rename = "socks5")]
    Socks5,
    #[serde(rename = "socks5h")]
    Socks5h,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
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
    /// Auth proxy thông dụng (ip:port + user/pass). None = không auth.
    /// Để riêng khỏi endpoint để UI nhập liệu rõ ràng, không phải nhồi URL.
    #[serde(default)]
    pub auth_user: Option<String>,
    #[serde(default)]
    pub auth_pass: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub status: TunnelStatus,
    #[serde(default = "default_concurrency_limit")]
    pub max_concurrent_streams: usize, // 0 = unlimited
    /// Giãn cách tối thiểu giữa 2 request khởi phát qua tunnel này (ms).
    /// 0 = tắt. Chống upstream rate-limit 429 oan khi burst (đã debate + đồng thuận).
    #[serde(default)]
    pub min_request_interval_ms: u64,
    #[serde(default)]
    pub start_command: Option<String>,
    #[serde(default)]
    pub stop_command: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub last_exit_ip: Option<String>,
    pub last_latency_ms: Option<u64>,
    pub tags: Vec<String>,
    /// Vị trí kết nối cho AdGuard VPN:
    /// - "random" (mặc định): ngẫu nhiên từ danh sách vị trí lấy được
    /// - "fastest": vị trí nhanh nhất (-f)
    /// - "<location_id>": vị trí cụ thể do người dùng chọn (ví dụ "US", "DE", "SG", "Tokyo")
    #[serde(default)]
    pub adguard_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdguardLocationItem {
    pub id: String,
    pub name: String,
    pub ping_ms: Option<u64>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelTestResult {
    pub tunnel_id: String,
    pub success: bool,
    pub exit_ip: Option<String>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// 1 dòng log debug per-tunnel (RAM-only, không persist): theo dõi vòng đời
/// start/stop/test/login/failover để thay hộp IP/error tĩnh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelEvent {
    /// Giờ local "HH:MM:SS".
    pub ts: String,
    /// "info" | "ok" | "error".
    pub kind: String,
    pub msg: String,
}
