use super::types::{OutboundTunnel, TunnelProtocol, TunnelStatus, TunnelTestResult};

/// Dựng reqwest client đi qua tunnel (SOCKS5 / HTTP proxy) hoặc direct.
/// Đặt connect_timeout (M4): stall ở bắt tay TCP/TLS/proxy không được treo request
/// vĩnh viễn. KHÔNG đặt total timeout vì SSE stream hợp lệ có thể sống hàng phút —
/// stall giữa stream vẫn là giới hạn đã biết của reqwest (không có read timeout riêng).
pub fn build_client(tunnel: &OutboundTunnel) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .connect_timeout(std::time::Duration::from_secs(30));

    // Auth proxy (nếu có user): reqwest tự gắn Proxy-Authorization cho cả
    // CONNECT tunnel lẫn plain HTTP (basic_auth native, không nhồi URL).
    let apply_auth = |proxy: reqwest::Proxy| -> reqwest::Proxy {
        match tunnel.auth_user.as_deref() {
            Some(u) if !u.trim().is_empty() => {
                proxy.basic_auth(u, tunnel.auth_pass.as_deref().unwrap_or(""))
            }
            _ => proxy,
        }
    };

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
                builder = builder.proxy(apply_auth(proxy));
            }
        }
        TunnelProtocol::Socks5h => {
            // Ép DNS qua proxy (remote resolve), khác Socks5 cho phép cả 2 scheme
            let proxy_url = if tunnel.endpoint.starts_with("socks5h://") {
                tunnel.endpoint.clone()
            } else {
                format!(
                    "socks5h://{}",
                    tunnel
                        .endpoint
                        .trim_start_matches("socks5://")
                )
            };

            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                builder = builder.proxy(apply_auth(proxy));
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
                builder = builder.proxy(apply_auth(proxy));
            }
        }
        TunnelProtocol::Https => {
            // Proxy qua kết nối TLS (CONNECT mã hóa), khác Http cho phép cả 2 scheme
            let proxy_url = if tunnel.endpoint.starts_with("https://") {
                tunnel.endpoint.clone()
            } else {
                format!(
                    "https://{}",
                    tunnel.endpoint.trim_start_matches("http://")
                )
            };

            if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                builder = builder.proxy(apply_auth(proxy));
            }
        }
    }

    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

/// Bóc userinfo `user:pass@` khỏi endpoint (user hay paste nguyên format
/// `scheme://user:pass@host:port` vào ô endpoint). Trả phần host:port sạch.
fn strip_userinfo(endpoint: &str) -> &str {
    let no_scheme = endpoint
        .trim_start_matches("socks5://")
        .trim_start_matches("socks5h://")
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    match no_scheme.rfind('@') {
        Some(i) => &no_scheme[i + 1..],
        None => no_scheme,
    }
}

/// Kiểm tra nhanh kết nối TCP socket tới endpoint (ví dụ 127.0.0.1:1080)
pub async fn check_tcp_reachability(endpoint: &str) -> Result<(), String> {
    let clean_addr = strip_userinfo(endpoint);

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
    let clean = strip_userinfo(endpoint);
    let host_port = clean.split('/').next().unwrap_or(clean);
    if host_port.is_empty() {
        return None;
    }
    if let Ok(addr) = host_port.parse::<std::net::SocketAddr>() {
        return Some(addr);
    }
    host_port.to_socket_addrs().ok()?.next()
}

/// Chọn tunnel khỏe nhất cho route: ưu tiên tunnel được gán nếu dùng được,
/// ngược lại failover sang tunnel enabled khác (ưu tiên Online trước Unknown).
/// Trả None khi không còn tunnel nào dùng được (caller sẽ báo 503).
///
/// Quy tắc dùng được (siết sau audit Stale Boot Status): tunnel CLI (có start_command)
/// ở trạng thái Unknown — tức chưa từng Online để chứng minh process sống — thì
/// KHÔNG được nhận traffic. Route vào đó chắc chắn rớt 502; thà 503 trung thực
/// ("tunnels unavailable, chống leak IP") còn hơn thử rồi rớt. Tunnel không cần
/// process (Direct / lệnh rỗng) thì Unknown vẫn dùng được.
pub fn select_healthy_tunnel_id(tunnels: &[OutboundTunnel], assigned_id: &str) -> Option<String> {
    if assigned_id == "direct_bypass" {
        return Some("direct_bypass".to_string());
    }
    let needs_process = |t: &OutboundTunnel| {
        t.start_command
            .as_deref()
            .map(|c| !c.trim().is_empty())
            .unwrap_or(false)
    };
    let is_usable = |t: &&OutboundTunnel| {
        if !t.enabled || t.status == TunnelStatus::Offline {
            return false;
        }
        t.status == TunnelStatus::Online || !needs_process(t)
    };
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
