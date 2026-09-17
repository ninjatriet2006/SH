pub mod key_manager;
pub mod manager;
pub mod rotation;
pub mod routing;
pub mod upstream;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, Response, StatusCode, Uri},
    response::IntoResponse,
};
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::fingerprint::FingerprintAnalyzer;
use crate::monitor::{RawTrafficLog, RingBufferLog};
use crate::proxy::routing::{match_route, resolve_tunnel_id};
use crate::proxy::upstream::{send_and_handle_upstream, RequestLogContext};
use crate::vpn::TunnelManager;

pub use crate::config;
pub use crate::config::{GatewayConfig, RouteRule, RouteStatus};
pub use crate::proxy::routing::{build_target_url, prefix_matches};

/// Giới hạn độ dài chuỗi body lưu trong RAM buffer để chống OOM
pub fn truncate_log_body(body: &str, max_chars: usize) -> String {
    let total_chars = body.chars().count();
    if total_chars > max_chars {
        let truncated: String = body.chars().take(max_chars).collect();
        format!("{}... [Truncated {} chars]", truncated, total_chars - max_chars)
    } else {
        body.to_string()
    }
}

/// Sniff & Format Response Body
pub fn format_logged_body(resp_bytes: &[u8], headers: &reqwest::header::HeaderMap) -> String {
    let content_encoding = headers
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_lowercase());

    if let Some(enc) = content_encoding {
        if enc.contains("gzip") || enc.contains("br") || enc.contains("zstd") || enc.contains("deflate") {
            return format!("[Compressed Data: {} | {} bytes]", enc, resp_bytes.len());
        }
    }

    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    if content_type.contains("image/")
        || content_type.contains("audio/")
        || content_type.contains("video/")
        || content_type.contains("application/octet-stream")
    {
        return format!("[Binary Media: {} | {} bytes]", content_type, resp_bytes.len());
    }

    let text = String::from_utf8_lossy(resp_bytes);
    truncate_log_body(&text, 2048)
}

pub struct AppState {
    pub config: parking_lot::RwLock<GatewayConfig>,
    pub logs: Arc<RingBufferLog>,
    pub client_cache: parking_lot::RwLock<HashMap<String, reqwest::Client>>,
    pub tunnel_semaphores: parking_lot::RwLock<HashMap<String, Arc<tokio::sync::Semaphore>>>,
    pub app_handle: parking_lot::RwLock<Option<tauri::AppHandle>>,
}

impl AppState {
    pub fn new(config: GatewayConfig) -> Self {
        let max_logs = config.max_log_entries;
        let logs = Arc::new(RingBufferLog::new(max_logs));
        let mut client_cache = HashMap::new();
        let mut tunnel_semaphores = HashMap::new();

        for tunnel in &config.tunnels {
            let c = TunnelManager::build_client(tunnel);
            client_cache.insert(tunnel.id.clone(), c);
            if tunnel.max_concurrent_streams > 0 {
                tunnel_semaphores.insert(
                    tunnel.id.clone(),
                    Arc::new(tokio::sync::Semaphore::new(tunnel.max_concurrent_streams)),
                );
            }
        }

        Self {
            config: parking_lot::RwLock::new(config),
            logs,
            client_cache: parking_lot::RwLock::new(client_cache),
            tunnel_semaphores: parking_lot::RwLock::new(tunnel_semaphores),
            app_handle: parking_lot::RwLock::new(None),
        }
    }

    pub fn get_client(&self, tunnel_id: &str) -> reqwest::Client {
        let cache = self.client_cache.read();
        if let Some(c) = cache.get(tunnel_id) {
            return c.clone();
        }
        drop(cache);

        let mut cache = self.client_cache.write();
        let conf = self.config.read();
        let tunnel = conf
            .tunnels
            .iter()
            .find(|t| t.id == tunnel_id)
            .cloned()
            .unwrap_or_else(|| crate::vpn::OutboundTunnel {
                id: "direct_bypass".to_string(),
                name: "Direct Internet (Failover Fallback)".to_string(),
                protocol: crate::vpn::TunnelProtocol::Direct,
                endpoint: "0.0.0.0:0".to_string(),
                enabled: true,
                tags: vec![],
                max_concurrent_streams: 0,
                start_command: None,
                stop_command: None,
                status: crate::vpn::TunnelStatus::Online,
                last_checked_at: None,
                last_error: None,
                last_exit_ip: None,
                last_latency_ms: None,
            });

        let client = TunnelManager::build_client(&tunnel);
        cache.insert(tunnel_id.to_string(), client.clone());
        client
    }

    pub fn refresh_clients(&self) {
        let conf = self.config.read();
        let mut cache = self.client_cache.write();
        let mut semaphores = self.tunnel_semaphores.write();
        cache.clear();
        semaphores.clear();

        for tunnel in &conf.tunnels {
            let c = TunnelManager::build_client(tunnel);
            cache.insert(tunnel.id.clone(), c);
            if tunnel.max_concurrent_streams > 0 {
                semaphores.insert(
                    tunnel.id.clone(),
                    Arc::new(tokio::sync::Semaphore::new(tunnel.max_concurrent_streams)),
                );
            }
        }
    }

    pub fn advance_route_key(&self, route_id: &str) -> Option<String> {
        let mut conf = self.config.write();
        if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
            r.key_manager.advance_to_next_key()
        } else {
            None
        }
    }

    pub fn record_log(&self, log: RawTrafficLog) {
        let max_disk = {
            let conf = self.config.read();
            conf.max_disk_log_entries
        };
        let log_path = crate::monitor::get_traffic_log_path();
        self.logs.push(log.clone());
        // Đẩy I/O đĩa xuống worker riêng của monitor (blocking pool),
        // không bao giờ block tokio worker thread của async HTTP request path.
        crate::monitor::spawn_disk_append(log_path, log, max_disk);
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
    let (route_id, target_url, tunnel_id, resolved_key, profile, max_streams) = {
        let mut conf = state.config.write();
        let matched = match_route(&mut conf.routes, port, &path);

        match matched {
            Some(rule) => {
                let rule_id = rule.id.clone();
                let rule_target = rule.target_base_url.clone();
                let rule_prefix = rule.path_prefix.clone();
                let rule_tunnel = rule.tunnel_id.clone();
                let rule_auth = rule.custom_auth_token.clone();
                let key = rule.key_manager.resolve_active_key().or_else(|| rule_auth);

                let selected_id = match resolve_tunnel_id(&conf.tunnels, &rule_tunnel) {
                    Some(id) => id,
                    None => {
                        let final_url = build_target_url(&rule_target, &rule_prefix, &path, &query);
                        let err_msg = format!("All VPN tunnels are offline or unavailable. Request rejected to prevent real IP leak.");
                        state.record_log(RawTrafficLog {
                            id: req_id,
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            route_id: rule_id,
                            method: method.to_string(),
                            port,
                            path,
                            target_url: final_url,
                            tunnel_id: rule_tunnel,
                            key_used_preview: None,
                            status_code: 503,
                            duration_ms: start_time.elapsed().as_millis() as u64,
                            is_streaming: false,
                            raw_request_headers: vec![],
                            raw_forwarded_headers: vec![],
                            raw_request_body: "".to_string(),
                            raw_response_headers: vec![],
                            raw_response_body: err_msg.clone(),
                            leaked_findings: vec![],
                        });
                        return Response::builder()
                            .status(StatusCode::SERVICE_UNAVAILABLE)
                            .body(Body::from(err_msg))
                            .unwrap();
                    }
                };

                let max_streams = conf
                    .tunnels
                    .iter()
                    .find(|t| t.id == selected_id)
                    .map(|t| t.max_concurrent_streams)
                    .unwrap_or(0);

                let profile = conf.get_active_fingerprint();
                let final_url = build_target_url(&rule_target, &rule_prefix, &path, &query);

                (rule_id, final_url, selected_id, key, profile, max_streams)
            }
            None => {
                let err_msg = format!("Route Not Found on port {} for path {}", port, path);
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from(err_msg))
                    .unwrap();
            }
        }
    };

    // Đọc raw request body
    let body_bytes = match request.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Failed to read request body: {}", e)))
                .unwrap();
        }
    };

    let raw_request_body = String::from_utf8_lossy(&body_bytes).to_string();

    // Thu thập raw request headers
    let mut header_map_for_audit = HashMap::new();
    let mut raw_request_headers = Vec::new();
    for (k, v) in &headers {
        let val_str = v.to_str().unwrap_or("<binary>").to_string();
        header_map_for_audit.insert(k.as_str().to_string(), val_str.clone());
        raw_request_headers.push((k.as_str().to_string(), val_str));
    }

    // Leak analysis & Sanitization
    let leaks = FingerprintAnalyzer::analyze_inbound(&header_map_for_audit, Some(&raw_request_body));
    let clean_headers = FingerprintAnalyzer::sanitize_headers(&header_map_for_audit, &profile);

    let client = state.get_client(&tunnel_id);
    let mut req_builder = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
        &target_url,
    );

    let key_preview = resolved_key.as_ref().map(|k| {
        if k.len() > 10 {
            format!("{}...{}", &k[..6], &k[k.len() - 4..])
        } else {
            k.clone()
        }
    });

    let mut raw_forwarded_headers = Vec::new();
    for (k, v) in clean_headers {
        let k_lower = k.to_lowercase();
        if k_lower == "host"
            || k_lower == "content-length"
            || k_lower == "connection"
            || k_lower == "keep-alive"
            || k_lower == "transfer-encoding"
            || k_lower == "upgrade"
        {
            continue;
        }

        if resolved_key.is_some() && k_lower == "authorization" {
            continue;
        }

        raw_forwarded_headers.push((k.clone(), v.clone()));
        req_builder = req_builder.header(k, v);
    }

    if let Some(ref token) = resolved_key {
        let auth_val = format!("Bearer {}", token);
        raw_forwarded_headers.push(("authorization".to_string(), format!("Bearer {}", key_preview.as_deref().unwrap_or("sk-***"))));
        req_builder = req_builder.header("authorization", auth_val);
    }

    // Concurrency Limiter
    let permit = if max_streams > 0 {
        let sem = {
            let s_map = state.tunnel_semaphores.read();
            s_map.get(&tunnel_id).cloned()
        };
        if let Some(sem) = sem {
            match sem.try_acquire_owned() {
                Ok(p) => Some(p),
                Err(_) => {
                    let err_msg = format!("Tunnel [{}] concurrency limit reached (max: {})", tunnel_id, max_streams);
                    state.record_log(RawTrafficLog {
                        id: req_id,
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        route_id,
                        method: method.to_string(),
                        port,
                        path,
                        target_url,
                        tunnel_id,
                        key_used_preview: key_preview,
                        status_code: 429,
                        duration_ms: start_time.elapsed().as_millis() as u64,
                        is_streaming: false,
                        raw_request_headers,
                        raw_forwarded_headers: vec![],
                        raw_request_body: truncate_log_body(&raw_request_body, 2048),
                        raw_response_headers: vec![],
                        raw_response_body: err_msg.clone(),
                        leaked_findings: leaks,
                    });
                    return Response::builder()
                        .status(StatusCode::TOO_MANY_REQUESTS)
                        .header("retry-after", "2")
                        .body(Body::from(err_msg))
                        .unwrap();
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    req_builder = req_builder.body(body_bytes.clone());

    let ctx = RequestLogContext {
        req_id,
        route_id,
        method,
        port,
        path,
        target_url,
        tunnel_id,
        key_preview,
        resolved_key,
        raw_request_headers,
        raw_forwarded_headers,
        raw_request_body,
        leaks,
        start_time,
        permit,
    };

    send_and_handle_upstream(req_builder, Arc::clone(&state), ctx).await
}
