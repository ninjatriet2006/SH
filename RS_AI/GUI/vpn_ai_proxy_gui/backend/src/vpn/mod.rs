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

pub struct TunnelManager;

impl TunnelManager {
    pub fn build_client(tunnel: &OutboundTunnel) -> reqwest::Client {
        let mut builder = reqwest::Client::builder().use_rustls_tls();

        match tunnel.protocol {
            TunnelProtocol::Direct => {}
            TunnelProtocol::Socks5 => {
                let proxy_url = if tunnel.endpoint.starts_with("socks5://")
                    || tunnel.endpoint.starts_with("socks5h://")
                {
                    tunnel.endpoint.clone()
                } else {
                    format!("socks5h://{}", tunnel.endpoint)
                };

                if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                    builder = builder.proxy(proxy);
                }
            }
            TunnelProtocol::Http => {
                let proxy_url = if tunnel.endpoint.starts_with("http://")
                    || tunnel.endpoint.starts_with("https://")
                {
                    tunnel.endpoint.clone()
                } else {
                    format!("http://{}", tunnel.endpoint)
                };

                if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                    builder = builder.proxy(proxy);
                }
            }
        }

        builder.build().unwrap_or_else(|_| reqwest::Client::new())
    }

    /// Kiểm tra nhanh kết nối TCP socket tới endpoint (ví dụ 127.0.0.1:1080)
    pub async fn check_tcp_reachability(endpoint: &str) -> Result<(), String> {
        let clean_addr = endpoint
            .trim_start_matches("socks5://")
            .trim_start_matches("socks5h://")
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        if clean_addr.is_empty() {
            return Ok(());
        }

        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::TcpStream::connect(clean_addr),
        )
        .await
        .map_err(|_| format!("TCP connection timeout to {}", clean_addr))?
        .map_err(|e| format!("Cannot reach endpoint {}: {}", clean_addr, e))?;

        Ok(())
    }

    pub async fn test_tunnel(tunnel: &OutboundTunnel) -> TunnelTestResult {
        // Nếu là protocol SOCKS/HTTP thì kiểm tra TCP socket trước (Dependency Check)
        if tunnel.protocol != TunnelProtocol::Direct && !tunnel.endpoint.is_empty() {
            if let Err(tcp_err) = Self::check_tcp_reachability(&tunnel.endpoint).await {
                return TunnelTestResult {
                    tunnel_id: tunnel.id.clone(),
                    success: false,
                    exit_ip: None,
                    latency_ms: None,
                    error: Some(format!("Proxy unreachable: {}. Please check if VPN is running.", tcp_err)),
                };
            }
        }

        let client = Self::build_client(tunnel);
        let start = std::time::Instant::now();

        // Query ipify for exit IP check (fallback to api.myip.com)
        let endpoints = ["https://api.ipify.org?format=json", "https://api.myip.com"];
        let mut last_err = String::new();

        for url in endpoints {
            match client
                .get(url)
                .timeout(std::time::Duration::from_secs(6))
                .send()
                .await
            {
                Ok(resp) => {
                    let latency_ms = start.elapsed().as_millis() as u64;
                    if resp.status().is_success() {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            let ip = json.get("ip").and_then(|v| v.as_str()).map(|s| s.to_string());
                            return TunnelTestResult {
                                tunnel_id: tunnel.id.clone(),
                                success: true,
                                exit_ip: ip,
                                latency_ms: Some(latency_ms),
                                error: None,
                            };
                        }
                    }
                }
                Err(e) => {
                    last_err = format!("Connection failed: {}", e);
                }
            }
        }

        TunnelTestResult {
            tunnel_id: tunnel.id.clone(),
            success: false,
            exit_ip: None,
            latency_ms: None,
            error: Some(if last_err.is_empty() { "Failed to resolve external exit IP".to_string() } else { last_err }),
        }
    }

    /// Chạy lệnh khởi động hoặc dừng tiến trình VPN
    pub fn run_tunnel_command(cmd_str: &str) -> Result<String, String> {
        let trimmed = cmd_str.trim();
        if trimmed.is_empty() {
            return Err("Command is empty".to_string());
        }

        #[cfg(target_os = "windows")]
        let output = std::process::Command::new("cmd")
            .args(["/C", trimmed])
            .output()
            .map_err(|e| format!("Failed to run command: {}", e))?;

        #[cfg(not(target_os = "windows"))]
        let output = std::process::Command::new("sh")
            .args(["-c", trimmed])
            .output()
            .map_err(|e| format!("Failed to run command: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("Command failed with status {}: {}", output.status, stderr))
        }
    }
}
