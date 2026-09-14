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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundTunnel {
    pub id: String,
    pub name: String,
    pub protocol: TunnelProtocol,
    pub endpoint: String, // e.g. "127.0.0.1:1080" or "socks5h://127.0.0.1:1080"
    pub enabled: bool,
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

    pub async fn test_tunnel(tunnel: &OutboundTunnel) -> TunnelTestResult {
        let client = Self::build_client(tunnel);
        let start = std::time::Instant::now();

        // Query ipify for exit IP check
        match client
            .get("https://api.ipify.org?format=json")
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
        {
            Ok(resp) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let ip = json.get("ip").and_then(|v| v.as_str()).map(|s| s.to_string());
                            TunnelTestResult {
                                tunnel_id: tunnel.id.clone(),
                                success: true,
                                exit_ip: ip,
                                latency_ms: Some(latency_ms),
                                error: None,
                            }
                        }
                        Err(e) => TunnelTestResult {
                            tunnel_id: tunnel.id.clone(),
                            success: false,
                            exit_ip: None,
                            latency_ms: Some(latency_ms),
                            error: Some(format!("Parse IP json error: {}", e)),
                        },
                    }
                } else {
                    TunnelTestResult {
                        tunnel_id: tunnel.id.clone(),
                        success: false,
                        exit_ip: None,
                        latency_ms: Some(latency_ms),
                        error: Some(format!("HTTP status {}", resp.status())),
                    }
                }
            }
            Err(e) => TunnelTestResult {
                tunnel_id: tunnel.id.clone(),
                success: false,
                exit_ip: None,
                latency_ms: None,
                error: Some(format!("Connection failed: {}", e)),
            },
        }
    }
}
