pub mod key_manager;
pub mod manager;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, Response, StatusCode, Uri},
    response::IntoResponse,
};
use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::fingerprint::{FingerprintAnalyzer, FingerprintProfile};
use crate::monitor::{RawTrafficLog, RingBufferLog};
use crate::proxy::key_manager::EndpointKeyManager;
use crate::vpn::{OutboundTunnel, TunnelManager};

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
}

// NOTE: field `strip_prefix` cũ đã xóa — build_target_url luôn strip prefix theo
// ranh giới segment (prefix_matches), không còn cấu hình nào đọc field đó.

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

/// So khớp prefix theo ranh giới segment: "/v1" khớp "/v1/models" nhưng KHÔNG khớp "/v1beta/x".
/// Dùng cho cả route matching và strip prefix để tránh nuốt nhầm route.
pub fn prefix_matches(path: &str, prefix: &str) -> bool {
    if prefix == "/" || prefix.is_empty() {
        return true;
    }
    if !path.starts_with(prefix) {
        return false;
    }
    matches!(path.as_bytes().get(prefix.len()), None | Some(b'/'))
}

/// Xây dựng URL đích thông minh (Auto-detect + Overlap Failsafe):
/// Cắt bỏ `path_prefix` tương ứng nếu request path bắt đầu bằng prefix đó.
/// Nếu segment cuối của `target_base_url` trùng với segment đầu của phần sub_path còn lại,
/// loại bỏ đoạn trùng lặp đó để chống lỗi lặp /v1/v1.
pub fn build_target_url(target_base_url: &str, path_prefix: &str, req_path: &str, query: &str) -> String {
    let clean_base = target_base_url.trim_end_matches('/');
    let clean_prefix = if path_prefix == "/" { "" } else { path_prefix.trim_end_matches('/') };
    
    let sub_path = if !clean_prefix.is_empty()
        && (req_path == clean_prefix
            || req_path.as_bytes().get(clean_prefix.len()) == Some(&b'/'))
    {
        &req_path[clean_prefix.len()..]
    } else {
        req_path
    };
    
    let mut clean_sub = sub_path.trim_start_matches('/');

    // Overlap Failsafe: Trích xuất segment cuối của target_base_url (VD: "v1" trong "https://api.openai.com/v1")
    if let Some(last_segment) = clean_base.rsplit('/').next() {
        if !last_segment.is_empty() {
            // Kiểm tra xem clean_sub có bắt đầu bằng segment đó không (VD: "v1/models" hoặc "v1")
            if clean_sub == last_segment {
                clean_sub = "";
            } else if clean_sub.starts_with(&format!("{}/", last_segment)) {
                clean_sub = &clean_sub[last_segment.len() + 1..];
            }
        }
    }

    if clean_sub.is_empty() {
        format!("{}{}", clean_base, query)
    } else {
        format!("{}/{}{}", clean_base, clean_sub, query)
    }
}

fn default_max_disk_log_entries() -> usize {
    5000
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
}

impl GatewayConfig {
    pub fn get_active_fingerprint(&self) -> FingerprintProfile {
        if !self.fingerprint_pool.is_empty() {
            self.fingerprint_pool[self.active_fingerprint_index % self.fingerprint_pool.len()].clone()
        } else {
            self.fingerprint_profile.clone()
        }
    }

    pub fn config_path() -> PathBuf {
        if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
            return PathBuf::from(dir).join("config.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home)
                .join(".config")
                .join("vpn_ai_proxy_gui");
            let _ = std::fs::create_dir_all(&path);
            return path.join("config.json");
        }
        PathBuf::from("vpn_ai_proxy_config.json")
    }

    pub fn load_or_default() -> Self {
        let p = Self::config_path();
        if p.exists() {
            if let Ok(data) = std::fs::read_to_string(&p) {
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
                            }
                        }

                        match serde_json::from_value::<GatewayConfig>(val) {
                            Ok(mut cfg) => {
                                for r in &mut cfg.routes {
                                    r.key_manager.refresh_metadata();
                                }
                                return cfg;
                            }
                            Err(e) => {
                                eprintln!("Error deserializing migrated config: {}. Creating backup.", e);
                                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                                let bak_path = p.with_file_name(format!("config.json.bak.{}", timestamp));
                                let _ = std::fs::rename(&p, &bak_path);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Corrupt config file: {}. Creating backup.", e);
                        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                        let bak_path = p.with_file_name(format!("config.json.bak.{}", timestamp));
                        let _ = std::fs::rename(&p, &bak_path);
                    }
                }
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
        }
    }
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
            client_cache.insert(tunnel.id.clone(), TunnelManager::build_client(tunnel));
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
        if let Some(client) = cache.get(tunnel_id) {
            return client.clone();
        }
        drop(cache);

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
        let mut semaphores = self.tunnel_semaphores.write();
        write_cache.clear();
        semaphores.clear();
        for tunnel in &conf.tunnels {
            write_cache.insert(tunnel.id.clone(), TunnelManager::build_client(tunnel));
            if tunnel.max_concurrent_streams > 0 {
                semaphores.insert(
                    tunnel.id.clone(),
                    Arc::new(tokio::sync::Semaphore::new(tunnel.max_concurrent_streams)),
                );
            }
        }
    }

    pub fn get_semaphore(&self, tunnel_id: &str, limit: usize) -> Option<Arc<tokio::sync::Semaphore>> {
        if limit == 0 {
            return None;
        }
        let read = self.tunnel_semaphores.read();
        if let Some(sem) = read.get(tunnel_id) {
            return Some(Arc::clone(sem));
        }
        drop(read);
        let mut write = self.tunnel_semaphores.write();
        let sem = write.entry(tunnel_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(limit)));
        Some(Arc::clone(sem))
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
        // Chạy I/O ghi file disk trong background thread pool (spawn_blocking)
        // để không bao giờ block tokio worker thread của async HTTP request path
        tokio::task::spawn_blocking(move || {
            let _ = crate::monitor::append_disk_log(&log_path, &log, max_disk);
        });
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
    // Ưu tiên path_prefix DÀI NHẤT khớp được: nếu route "/" khai báo trước route "/v1",
    // dùng find() sẽ match "/" và đẩy request /v1/* sai target.
    // Khớp theo ranh giới segment (prefix_matches) để "/v1" không nuốt "/v1beta".
    // Dùng write-lock (block này không có .await nên an toàn): cho phép refresh key cache
    // từ đĩa ngay trong request, key sửa ngoài file có hiệu lực tức thì thay vì stale.
    let (route_id, target_url, tunnel_id, resolved_key, profile, max_streams) = {
        let mut conf = state.config.write();
        let matched = conf
            .routes
            .iter_mut()
            .filter(|r| r.enabled && r.port == port && prefix_matches(&path, &r.path_prefix))
            .max_by_key(|r| r.path_prefix.len());

        match matched {
            Some(rule) => {
                // Clone dữ liệu cần thiết ra trước để nhả borrow &mut rule,
                // cho phép đọc conf.tunnels / fingerprint ngay sau đó.
                let rule_id = rule.id.clone();
                let rule_target = rule.target_base_url.clone();
                let rule_prefix = rule.path_prefix.clone();
                let rule_tunnel = rule.tunnel_id.clone();
                let rule_auth = rule.custom_auth_token.clone();
                let key = rule.key_manager.resolve_active_key().or_else(|| rule_auth);

                // Failover chọn tunnel: tunnel được gán mà bị tắt/rớt/không tồn tại
                // → tự động nhảy sang tunnel khỏe nhất còn lại trong pool thay vì 503 chết đứng.
                let selected_id = match TunnelManager::select_healthy_tunnel_id(&conf.tunnels, &rule_tunnel) {
                    Some(id) => id,
                    None => {
                        return Response::builder()
                            .status(StatusCode::SERVICE_UNAVAILABLE)
                            .body(Body::from(format!(
                                "No healthy tunnel available: assigned tunnel [{}] is disabled/offline and no other enabled tunnel exists.",
                                rule_tunnel
                            )))
                            .unwrap();
                    }
                };

                let max_streams = conf.tunnels.iter().find(|t| t.id == selected_id).map(|t| t.max_concurrent_streams).unwrap_or(0);
                let full = build_target_url(&rule_target, &rule_prefix, &path, &query);

                (
                    rule_id,
                    full,
                    selected_id,
                    key,
                    conf.get_active_fingerprint(),
                    max_streams,
                )
            }
            None => {
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

    // 100% Raw Inbound Request Body
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

    // 100% Raw Inbound Headers
    let mut header_map_for_audit = HashMap::new();
    let mut raw_request_headers = Vec::new();
    for (k, v) in &headers {
        let val_str = v.to_str().unwrap_or("<binary>").to_string();
        header_map_for_audit.insert(k.as_str().to_string(), val_str.clone());
        raw_request_headers.push((k.as_str().to_string(), val_str));
    }

    // Leak analysis
    let leaks = FingerprintAnalyzer::analyze_inbound(&header_map_for_audit, Some(&raw_request_body));

    // Sanitized headers
    let clean_headers = FingerprintAnalyzer::sanitize_headers(&header_map_for_audit, &profile);

    // Tunnel client
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
        // Loại bỏ các hop-by-hop headers có thể làm sập socket hoặc xung đột http parser
        if k_lower == "host"
            || k_lower == "content-length"
            || k_lower == "connection"
            || k_lower == "keep-alive"
            || k_lower == "transfer-encoding"
            || k_lower == "upgrade"
        {
            continue;
        }

        // Nếu Endpoint có cấu hình xoay Key (resolved_key), PHẢI loại bỏ authorization header cũ của client để không bị duplicate header -> 400 Bad Request
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

    // Tunnel Concurrency Limiter (Semaphore per tunnel)
    let _permit = if max_streams > 0 {
        if let Some(sem) = state.get_semaphore(&tunnel_id, max_streams) {
            match sem.try_acquire_owned() {
                Ok(permit) => Some(permit),
                Err(_) => {
                    let err_msg = format!("Tunnel [{}] concurrency limit ({}) exceeded", tunnel_id, max_streams);
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

            // 100% Raw Response Headers (Lọc bỏ hop-by-hop headers nguy hiểm như transfer-encoding khi trả về Axum body)
            let mut raw_response_headers = Vec::new();
            for (k, v) in upstream_resp.headers() {
                let v_str = v.to_str().unwrap_or("").to_string();
                let k_lower = k.as_str().to_lowercase();
                raw_response_headers.push((k.as_str().to_string(), v_str));

                // Bỏ qua các hop-by-hop headers tránh làm sập parser HTTP của client
                if k_lower == "transfer-encoding" || k_lower == "content-length" || k_lower == "connection" || k_lower == "keep-alive" {
                    continue;
                }
                resp_builder = resp_builder.header(k.as_str(), v.as_bytes());
            }

            if is_streaming {
                let stream = upstream_resp.bytes_stream();
                let accumulated = Arc::new(parking_lot::Mutex::new(Vec::<u8>::new()));
                let acc_clone = Arc::clone(&accumulated);
                let state_clone = Arc::clone(&state);
                let req_id_clone = req_id.clone();

                let teed_stream = stream.map(move |chunk_res| {
                    if let Ok(ref chunk) = chunk_res {
                        let mut acc = acc_clone.lock();
                        if acc.len() < 4096 {
                            let remain = 4096 - acc.len();
                            let to_take = chunk.len().min(remain);
                            acc.extend_from_slice(&chunk[..to_take]);
                        }
                    }
                    chunk_res
                });

                // Wrap stream drop or completion to update log.
                // Giữ semaphore permit trong guard: với SSE, body truyền SAU khi handler return,
                // nếu permit drop ở cuối hàm thì giới hạn concurrency vô hiệu với streaming.
                struct StreamGuard {
                    req_id: String,
                    accumulated: Arc<parking_lot::Mutex<Vec<u8>>>,
                    state: Arc<AppState>,
                    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
                }
                impl Drop for StreamGuard {
                    fn drop(&mut self) {
                        let bytes = self.accumulated.lock().clone();
                        let text = String::from_utf8_lossy(&bytes);
                        let final_str = if bytes.is_empty() {
                            "[Streaming SSE Completed - Empty Response]".to_string()
                        } else {
                            truncate_log_body(&text, 2048)
                        };
                        self.state.logs.update_response_body(&self.req_id, final_str);
                    }
                }

                let guard = StreamGuard {
                    req_id: req_id_clone,
                    accumulated,
                    state: state_clone,
                    _permit,
                };

                let guarded_stream = teed_stream.map(move |item| {
                    let _g = &guard;
                    item
                });

                let body = Body::from_stream(guarded_stream);

                let truncated_req_body = truncate_log_body(&raw_request_body, 2048);

                // Ghi nhận đầy đủ dữ liệu cho SSE stream
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
                    status_code,
                    duration_ms,
                    is_streaming: true,
                    raw_request_headers,
                    raw_forwarded_headers,
                    raw_request_body: truncated_req_body,
                    raw_response_headers,
                    raw_response_body: "[Streaming SSE Response Live Flow]".to_string(),
                    leaked_findings: leaks,
                });

                resp_builder.body(body).unwrap()
            } else {
                // 100% Raw Response Body (Cho cả 200 OK lẫn 4xx, 5xx hoàn toàn nguyên vẹn)
                let resp_headers = upstream_resp.headers().clone();
                let resp_bytes = upstream_resp.bytes().await.unwrap_or_else(|_| Bytes::new());
                let truncated_resp_body = format_logged_body(&resp_bytes, &resp_headers);
                let truncated_req_body = truncate_log_body(&raw_request_body, 2048);

                state.record_log(RawTrafficLog {
                    id: req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    route_id: route_id.clone(),
                    method: method.to_string(),
                    port,
                    path,
                    target_url,
                    tunnel_id,
                    key_used_preview: key_preview,
                    status_code,
                    duration_ms,
                    is_streaming: false,
                    raw_request_headers,
                    raw_forwarded_headers,
                    raw_request_body: truncated_req_body,
                    raw_response_headers,
                    raw_response_body: truncated_resp_body,
                    leaked_findings: leaks,
                });

                // Xử lý vòng đời key (Feature Key Filtering):
                // - 401 Unauthorized: key sai/thu hồi -> tự động xóa vĩnh viễn khỏi file chính
                // - 403 Forbidden: key hết quota -> chuyển sang file failed_key_file_path
                let mut key_was_removed = false;
                if let Some(ref key_used) = resolved_key {
                    if status_code == 401 {
                        let mut conf = state.config.write();
                        if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
                            if let Ok(removed) = r.key_manager.remove_key_from_main_file(key_used) {
                                key_was_removed = removed;
                            }
                            let _ = conf.save_to_disk();
                        }
                    } else if status_code == 403 {
                        let mut conf = state.config.write();
                        if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
                            if let Ok(removed) = r.key_manager.move_key_to_failed_file(key_used) {
                                key_was_removed = removed;
                            }
                            let _ = conf.save_to_disk();
                        }
                    }

                    // Phát sóng sự kiện IPC báo cho Frontend cập nhật UI Real-time nếu danh sách Key có biến động
                    if key_was_removed {
                        if let Some(ref handle) = *state.app_handle.read() {
                            use tauri::Emitter;
                            let _ = handle.emit("route-keys-updated", serde_json::json!({ "route_id": route_id }));
                        }
                    }
                }

                // CRITICAL: Đổi Key đồng nghĩa với việc đổi Account. Bắt buộc phải xoay đồng bộ cả Fingerprint và Tunnel (IP VPN) để tạo một Identity hoàn toàn mới. Nếu không, hệ thống Anti-fraud của AI (OpenAI/Anthropic) sẽ truy vết được IP cũ và Shadowban toàn bộ pool keys!
                // Unified Session Rotation on 401/403: Rotate key, rotate tunnel (if multiple enabled tunnels exist), rotate fingerprint profile
                if status_code == 401 || status_code == 403 {
                    let mut conf = state.config.write();
                    let enabled_tunnel_ids: Vec<String> = conf.tunnels.iter().filter(|t| t.enabled).map(|t| t.id.clone()).collect();
                    // 1. Advance route key:
                    // TUYỆT ĐỐI KHÔNG gọi advance_to_next_key() nếu key đã bị xóa (key kế tiếp đã tự động trượt vào slot hiện tại)
                    if let Some(r) = conf.routes.iter_mut().find(|r| r.id == route_id) {
                        if !key_was_removed {
                            r.key_manager.advance_to_next_key();
                        }
                        // Also switch assigned tunnel if there are other enabled tunnels
                        if enabled_tunnel_ids.len() > 1 {
                            if let Some(curr_idx) = enabled_tunnel_ids.iter().position(|id| id == &r.tunnel_id) {
                                let next_idx = (curr_idx + 1) % enabled_tunnel_ids.len();
                                r.tunnel_id = enabled_tunnel_ids[next_idx].clone();
                            }
                        }
                    }
                    // 2. Rotate fingerprint profile
                    if !conf.fingerprint_pool.is_empty() {
                        conf.active_fingerprint_index = (conf.active_fingerprint_index + 1) % conf.fingerprint_pool.len();
                    }
                    let _ = conf.save_to_disk();
                }

                resp_builder.body(Body::from(resp_bytes)).unwrap()
            }
        }
        Err(err) => {
            let err_msg = format!("Tunnel [{}] upstream connection error: {}", tunnel_id, err);

            // Failover bền vững: đánh dấu tunnel rớt mạng để request kế tiếp tự nhảy
            // sang tunnel khỏe (selection-time failover), đồng thời báo UI cập nhật.
            {
                let mut conf = state.config.write();
                if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                    t.status = crate::vpn::TunnelStatus::Offline;
                    t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                    t.last_error = Some(format!("Upstream connection error: {}", err));
                    let _ = conf.save_to_disk();
                }
            }
            if let Some(ref handle) = *state.app_handle.read() {
                use tauri::Emitter;
                let _ = handle.emit("tunnel-status-changed", serde_json::json!({ "tunnel_id": tunnel_id, "success": false }));
            }

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
                status_code: 502,
                duration_ms,
                is_streaming: false,
                raw_request_headers,
                raw_forwarded_headers,
                raw_request_body: truncate_log_body(&raw_request_body, 2048),
                raw_response_headers: vec![],
                raw_response_body: err_msg.clone(),
                leaked_findings: leaks,
            });

            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(err_msg))
                .unwrap()
        }
    }
}
