use super::types::{OutboundTunnel, TunnelProtocol, TunnelStatus, TunnelTestResult};

/// Dựng reqwest client đi qua tunnel (SOCKS5 / HTTP proxy) hoặc direct.
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

/// Test tunnel: kiểm tra TCP reachability trước (Dependency Check), sau đó query
/// ipify để xác thực exit IP (fallback sang api.myip.com).
pub async fn test_tunnel(tunnel: &OutboundTunnel) -> TunnelTestResult {
    // Nếu là protocol SOCKS/HTTP thì kiểm tra TCP socket trước (Dependency Check)
    if tunnel.protocol != TunnelProtocol::Direct && !tunnel.endpoint.is_empty() {
        if let Err(tcp_err) = check_tcp_reachability(&tunnel.endpoint).await {
            return TunnelTestResult {
                tunnel_id: tunnel.id.clone(),
                success: false,
                exit_ip: None,
                latency_ms: None,
                error: Some(format!("Proxy unreachable: {}. Please check if VPN is running.", tcp_err)),
            };
        }
    }

    let client = build_client(tunnel);
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

/// Trích xuất địa chỉ IP:Port từ endpoint (bỏ qua scheme/path) để làm pre-flight port check.
/// Hỗ trợ cả hostname (VD: localhost:1080) qua ToSocketAddrs — trước đây parse thất bại
/// là bỏ qua check trong im lặng, tạo lỗ hổng cho trường hợp cổng bị chiếm.
pub fn endpoint_addr(endpoint: &str) -> Option<std::net::SocketAddr> {
    use std::net::ToSocketAddrs;
    let clean = endpoint
        .trim_start_matches("socks5://")
        .trim_start_matches("socks5h://")
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let host_port = clean.split('/').next().unwrap_or(clean);
    if host_port.is_empty() {
        return None;
    }
    if let Ok(addr) = host_port.parse::<std::net::SocketAddr>() {
        return Some(addr);
    }
    host_port.to_socket_addrs().ok()?.next()
}

/// Chọn tunnel khỏe nhất cho route: ưu tiên tunnel được gán nếu enabled & không Offline,
/// ngược lại failover sang tunnel enabled khác (ưu tiên Online trước Unknown).
/// Trả None khi không còn tunnel nào dùng được (caller sẽ báo 503).
pub fn select_healthy_tunnel_id(tunnels: &[OutboundTunnel], assigned_id: &str) -> Option<String> {
    if assigned_id == "direct_bypass" {
        return Some("direct_bypass".to_string());
    }
    let is_usable = |t: &&OutboundTunnel| t.enabled && t.status != TunnelStatus::Offline;
    if let Some(t) = tunnels
        .iter()
        .find(|t| t.id == assigned_id)
        .filter(|t| is_usable(t))
    {
        return Some(t.id.clone());
    }
    tunnels
        .iter()
        .filter(is_usable)
        .max_by_key(|t| match t.status {
            TunnelStatus::Online => 2,
            TunnelStatus::Unknown => 1,
            TunnelStatus::Offline => 0,
        })
        .map(|t| t.id.clone())
}
