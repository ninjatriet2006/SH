pub mod manager;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, Response, StatusCode, Uri},
    response::IntoResponse,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::fingerprint::{FingerprintAnalyzer, FingerprintProfile};
use crate::monitor::{RequestLog, RingBufferLog};
use crate::vpn::{OutboundTunnel, TunnelManager, TunnelProtocol};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,
    pub name: String,
    pub port: u16,              // Cổng lắng nghe riêng (VD: 3000, 3001, 3002...)
    pub path_prefix: String,     // Prefix ví dụ "/v1", "/mirror", "/"
    pub target_base_url: String, // Domain muốn chuyển tiếp: "https://abc.xyz/v1", "https://api.openai.com/v1"
    pub tunnel_id: String,       // Gán với OutboundTunnel ID nào (AdGuard SOCKS, WireGuard, Direct...)
    pub enabled: bool,
    pub strip_prefix: bool,
    pub custom_auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub tunnels: Vec<OutboundTunnel>,
    pub routes: Vec<RouteRule>,
    pub fingerprint_profile: FingerprintProfile,
    pub max_log_entries: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            tunnels: vec![
                OutboundTunnel {
                    id: "adguard_default".to_string(),
                    name: "AdGuard VPN (Local SOCKS)".to_string(),
                    protocol: TunnelProtocol::Socks5,
                    endpoint: "127.0.0.1:1080".to_string(),
                    enabled: true,
                    last_exit_ip: None,
                    last_latency_ms: None,
                    tags: vec!["adguard".to_string(), "vpn".to_string()],
                },
                OutboundTunnel {
                    id: "direct_bypass".to_string(),
                    name: "Direct Internet (No VPN)".to_string(),
                    protocol: TunnelProtocol::Direct,
                    endpoint: "".to_string(),
                    enabled: true,
                    last_exit_ip: None,
                    last_latency_ms: None,
                    tags: vec!["direct".to_string()],
                },
            ],
            routes: vec![
                RouteRule {
                    id: "openai_rule".to_string(),
                    name: "OpenAI Proxy (Port 3000)".to_string(),
                    port: 3000,
                    path_prefix: "/v1".to_string(),
                    target_base_url: "https://api.openai.com/v1".to_string(),
                    tunnel_id: "adguard_default".to_string(),
                    enabled: true,
                    strip_prefix: true,
                    custom_auth_token: None,
                },
                RouteRule {
                    id: "anthropic_rule".to_string(),
                    name: "Anthropic Proxy (Port 3000)".to_string(),
                    port: 3000,
                    path_prefix: "/anthropic".to_string(),
                    target_base_url: "https://api.anthropic.com".to_string(),
                    tunnel_id: "adguard_default".to_string(),
                    enabled: true,
                    strip_prefix: true,
                    custom_auth_token: None,
                },
                RouteRule {
                    id: "custom_mirror_rule".to_string(),
                    name: "Custom Mirror (Port 3001)".to_string(),
                    port: 3001,
                    path_prefix: "/".to_string(),
                    target_base_url: "https://abc.xyz/v1".to_string(),
                    tunnel_id: "adguard_default".to_string(),
                    enabled: true,
                    strip_prefix: false,
                    custom_auth_token: None,
                },
            ],
            fingerprint_profile: FingerprintProfile::default(),
            max_log_entries: 500,
        }
    }
}

pub struct AppState {
    pub config: parking_lot::RwLock<GatewayConfig>,
    pub logs: Arc<RingBufferLog>,
    pub client_cache: parking_lot::RwLock<HashMap<String, reqwest::Client>>,
}

impl AppState {
    pub fn new(config: GatewayConfig) -> Self {
        let max_logs = config.max_log_entries;
        let logs = Arc::new(RingBufferLog::new(max_logs));
        let mut client_cache = HashMap::new();

        for tunnel in &config.tunnels {
            client_cache.insert(tunnel.id.clone(), TunnelManager::build_client(tunnel));
        }

        Self {
            config: parking_lot::RwLock::new(config),
            logs,
            client_cache: parking_lot::RwLock::new(client_cache),
        }
    }

    pub fn get_client(&self, tunnel_id: &str) -> reqwest::Client {
        let cache = self.client_cache.read();
        if let Some(client) = cache.get(tunnel_id) {
            return client.clone();
        }
        drop(cache);

        // Fallback: build or return direct
        let conf = self.config.read();
        if let Some(tunnel) = conf.tunnels.iter().find(|t| t.id == tunnel_id) {
            let client = TunnelManager::build_client(tunnel);
            let mut write_cache = self.client_cache.write();
            write_cache.insert(tunnel_id.to_string(), client.clone());
            client
        } else {
            reqwest::Client::new()
        }
    }

    pub fn refresh_clients(&self) {
        let conf = self.config.read();
        let mut write_cache = self.client_cache.write();
        write_cache.clear();
        for tunnel in &conf.tunnels {
            write_cache.insert(tunnel.id.clone(), TunnelManager::build_client(tunnel));
        }
    }
}

pub async fn handle_route_request(
    port: u16,
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    request: Request<Body>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let req_id = uuid::Uuid::new_v4().to_string();

    let path = uri.path().to_string();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();

    // Match route based on (port, path_prefix)
    let (target_url, tunnel_id, custom_token, profile) = {
        let conf = state.config.read();
        let matched = conf.routes.iter().find(|r| {
            r.enabled && r.port == port && path.starts_with(&r.path_prefix)
        });

        match matched {
            Some(rule) => {
                let sub_path = if rule.strip_prefix && rule.path_prefix != "/" {
                    path.strip_prefix(&rule.path_prefix).unwrap_or(&path)
                } else {
                    &path
                };
                let base = rule.target_base_url.trim_end_matches('/');
                let sub = sub_path.trim_start_matches('/');
                let full = if sub.is_empty() {
                    format!("{}{}", base, query)
                } else {
                    format!("{}/{}{}", base, sub, query)
                };
                (
                    full,
                    rule.tunnel_id.clone(),
                    rule.custom_auth_token.clone(),
                    conf.fingerprint_profile.clone(),
                )
            }
            None => {
                // If not matched directly on custom port, fallback error
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from(format!(
                        "No matching route configured for port {} and path {}",
                        port, path
                    )))
                    .unwrap();
            }
        }
    };

    // Read inbound request body
    let body_bytes = match request.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Failed to read request body: {}", e)))
                .unwrap();
        }
    };

    let body_str = String::from_utf8_lossy(&body_bytes);
    let prompt_preview = if body_str.len() > 300 {
        Some(format!("{}...", &body_str[..300]))
    } else if !body_str.is_empty() {
        Some(body_str.to_string())
    } else {
        None
    };

    // Map headers for inspection
    let mut header_map_for_audit = HashMap::new();
    let mut client_headers_vec = Vec::new();
    for (k, v) in &headers {
        let val_str = v.to_str().unwrap_or("<binary>").to_string();
        header_map_for_audit.insert(k.as_str().to_string(), val_str.clone());
        client_headers_vec.push((k.as_str().to_string(), val_str));
    }

    // 1. Analyze Inbound Fingerprint Leaks
    let leaks = FingerprintAnalyzer::analyze_inbound(&header_map_for_audit, Some(&body_str));

    // 2. Sanitize & Spoof Outbound Headers
    let clean_headers = FingerprintAnalyzer::sanitize_headers(&header_map_for_audit, &profile);

    // Pick client corresponding to assigned tunnel
    let client = state.get_client(&tunnel_id);

    let mut req_builder = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
        &target_url,
    );

    // Insert sanitized headers
    let mut forwarded_headers_vec = Vec::new();
    for (k, v) in clean_headers {
        if k == "host" || k == "content-length" {
            continue;
        }
        forwarded_headers_vec.push((k.clone(), v.clone()));
        req_builder = req_builder.header(k, v);
    }

    if let Some(token) = custom_token {
        req_builder = req_builder.header("authorization", format!("Bearer {}", token));
    }

    req_builder = req_builder.body(body_bytes.clone());

    // Send upstream request
    let response_result = req_builder.send().await;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    match response_result {
        Ok(upstream_resp) => {
            let status = upstream_resp.status();
            let status_code = status.as_u16();
            let is_streaming = upstream_resp
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .map(|ct| ct.contains("text/event-stream"))
                .unwrap_or(false);

            let mut resp_builder = Response::builder().status(status_code);

            for (k, v) in upstream_resp.headers() {
                resp_builder = resp_builder.header(k.as_str(), v.as_bytes());
            }

            if is_streaming {
                let stream = upstream_resp.bytes_stream();
                let body = Body::from_stream(stream);

                state.logs.push(RequestLog {
                    id: req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    method: method.to_string(),
                    path: format!(":{}{}", port, path),
                    target_url,
                    status_code,
                    duration_ms,
                    leaked_findings: leaks,
                    client_headers: client_headers_vec,
                    forwarded_headers: forwarded_headers_vec,
                    prompt_preview,
                    response_preview: Some("[Streaming SSE Response]".to_string()),
                    is_streaming: true,
                    bytes_sent: body_bytes.len(),
                    bytes_received: 0,
                });

                resp_builder.body(body).unwrap()
            } else {
                let resp_bytes = upstream_resp.bytes().await.unwrap_or_else(|_| Bytes::new());
                let resp_str = String::from_utf8_lossy(&resp_bytes);
                let response_preview = if resp_str.len() > 300 {
                    Some(format!("{}...", &resp_str[..300]))
                } else {
                    Some(resp_str.to_string())
                };

                state.logs.push(RequestLog {
                    id: req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    method: method.to_string(),
                    path: format!(":{}{}", port, path),
                    target_url,
                    status_code,
                    duration_ms,
                    leaked_findings: leaks,
                    client_headers: client_headers_vec,
                    forwarded_headers: forwarded_headers_vec,
                    prompt_preview,
                    response_preview,
                    is_streaming: false,
                    bytes_sent: body_bytes.len(),
                    bytes_received: resp_bytes.len(),
                });

                resp_builder.body(Body::from(resp_bytes)).unwrap()
            }
        }
        Err(err) => {
            let err_msg = format!("Tunnel [{}] upstream error: {}", tunnel_id, err);
            state.logs.push(RequestLog {
                id: req_id,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                method: method.to_string(),
                path: format!(":{}{}", port, path),
                target_url,
                status_code: 502,
                duration_ms,
                leaked_findings: leaks,
                client_headers: client_headers_vec,
                forwarded_headers: forwarded_headers_vec,
                prompt_preview,
                response_preview: Some(err_msg.clone()),
                is_streaming: false,
                bytes_sent: body_bytes.len(),
                bytes_received: 0,
            });

            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(err_msg))
                .unwrap()
        }
    }
}
