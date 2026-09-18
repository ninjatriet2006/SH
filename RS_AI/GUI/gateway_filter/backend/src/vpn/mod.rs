pub mod network;
pub mod process;
pub mod types;

pub use types::{
    default_concurrency_limit, OutboundTunnel, TunnelEvent, TunnelProtocol, TunnelStatus,
    TunnelTestResult, TUNNEL_CMD_TIMEOUT_SECS,
};

/// Facade giữ nguyên API `TunnelManager::...` cho toàn bộ callers hiện tại
/// (commands, workers, proxy, tests): logic thực tế nằm ở `network` và `process`.
pub struct TunnelManager;

impl TunnelManager {
    pub fn build_client(tunnel: &OutboundTunnel) -> reqwest::Client {
        network::build_client(tunnel)
    }

    pub async fn check_tcp_reachability(endpoint: &str) -> Result<(), String> {
        network::check_tcp_reachability(endpoint).await
    }

    pub async fn test_tunnel(tunnel: &OutboundTunnel) -> TunnelTestResult {
        network::test_tunnel(tunnel).await
    }

    pub fn endpoint_addr(endpoint: &str) -> Option<std::net::SocketAddr> {
        network::endpoint_addr(endpoint)
    }

    pub fn select_healthy_tunnel_id(tunnels: &[OutboundTunnel], assigned_id: &str) -> Option<String> {
        network::select_healthy_tunnel_id(tunnels, assigned_id)
    }

    // NOTE: bản sync cũ (std::process::Command, block vô thời hạn) đã xóa —
    // toàn bộ production dùng run_tunnel_command_timeout bên dưới.

    pub async fn run_tunnel_command_timeout(cmd_str: &str, timeout_secs: u64) -> Result<String, String> {
        process::run_tunnel_command_timeout(cmd_str, timeout_secs).await
    }
}
