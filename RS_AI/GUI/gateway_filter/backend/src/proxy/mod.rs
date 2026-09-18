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

/// Tính slot pacing GCRA cho tunnel: target = max(now, last + interval).
/// Trả (thời gian phải chờ, target để lưu). Pure, unit-test được.
/// Dùng max() thay vì interval-elapsed để burst dồn dập (last > now) không underflow.
pub fn compute_pacing_slot(
    last: Option<tokio::time::Instant>,
    interval: std::time::Duration,
    now: tokio::time::Instant,
) -> (std::time::Duration, tokio::time::Instant) {
    let target = match last {
        Some(t) => {
            let candidate = t + interval;
            if candidate > now {
                candidate
            } else {
                now
            }
        }
        None => now,
    };
    let wait = target.checked_duration_since(now).unwrap_or_default();
    (wait, target)
}

/// Đặt slot pacing 2 tầng cho 1 request (GCRA mỗi tầng, lấy wait LỚN nhất):
/// - tầng endpoint (key = route_id): bảo vệ quota riêng của endpoint/key.
/// - tầng tunnel (key = "tunnel:{id}"): bảo vệ egress chung khi NHIỀU route
///   chung 1 tunnel (mỗi route hàng riêng thì tổng tunnel vẫn dội gấp N lần).
/// Trả thời gian phải chờ. Pure logic + mutate map truyền vào → unit-test được.
pub fn reserve_pacing_slot(
    slots: &mut HashMap<String, tokio::time::Instant>,
    route_id: &str,
    tunnel_id: &str,
    route_ms: u64,
    tunnel_ms: u64,
    now: tokio::time::Instant,
) -> std::time::Duration {
    let mut wait = std::time::Duration::ZERO;
    if route_ms > 0 {
        let (w, target) = compute_pacing_slot(
            slots.get(route_id).copied(),
            std::time::Duration::from_millis(route_ms),
            now,
        );
        slots.insert(route_id.to_string(), target);
        wait = wait.max(w);
    }
    if tunnel_ms > 0 {
        let tkey = format!("tunnel:{}", tunnel_id);
        let (w, target) = compute_pacing_slot(
            slots.get(&tkey).copied(),
            std::time::Duration::from_millis(tunnel_ms),
            now,
        );
        slots.insert(tkey, target);
        wait = wait.max(w);
    }
    wait
}

pub struct AppState {
    pub config: parking_lot::RwLock<GatewayConfig>,
    pub logs: Arc<RingBufferLog>,
    pub client_cache: parking_lot::RwLock<HashMap<String, reqwest::Client>>,
    pub tunnel_semaphores: parking_lot::RwLock<HashMap<String, Arc<tokio::sync::Semaphore>>>,
    pub app_handle: parking_lot::RwLock<Option<tauri::AppHandle>>,
    /// Slot pacing đã cấp theo tunnel (GCRA): request đồng thời xếp hàng qua đây.
    /// Khóa ngắn hạn, thả trước khi sleep — xem handle_route_request.
    pub pacing_slots: parking_lot::Mutex<HashMap<String, tokio::time::Instant>>,
    /// Nhật ký debug per-tunnel (RAM-only, cap 30/tunnel): thay hộp IP/error tĩnh
    /// bằng lịch sử theo dõi được (test/start/stop/login/failover).
    pub tunnel_events: parking_lot::Mutex<HashMap<String, std::collections::VecDeque<crate::vpn::TunnelEvent>>>,
    /// Tiến trình login CLI đang chạy nền theo tunnel (device-code flow cần poll lâu).
    /// Waiter poll try_wait mỗi giây (không giữ lock qua await); cancel kill trực tiếp.
    /// LƯU Ý: adguardvpn-cli bị kill thường dump core (lỗi của nó, systemd giữ) —
    /// chỉ kill khi user bấm Hủy hoặc quá hạn, không kill bừa.
    pub login_children:
        parking_lot::Mutex<HashMap<String, tokio::process::Child>>,
    /// URL + mã device-code trích từ output login (hiện UI + mở trình duyệt).
    pub login_info: parking_lot::Mutex<HashMap<String, String>>,
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
            pacing_slots: parking_lot::Mutex::new(HashMap::new()),
            tunnel_events: parking_lot::Mutex::new(HashMap::new()),
            login_children: parking_lot::Mutex::new(HashMap::new()),
            login_info: parking_lot::Mutex::new(HashMap::new()),
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
                auth_user: None,
                auth_pass: None,
                enabled: true,
                tags: vec![],
                max_concurrent_streams: 0,
                min_request_interval_ms: 0,
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
        drop(cache);
        drop(semaphores);
        // Dọn slot pacing mồ côi (key route_id hoặc "tunnel:{id}").
        // Chạy ở đây để save_config/add/delete tunnel (đều gọi refresh) tự dọn.
        let mut live_ids: Vec<String> =
            conf.routes.iter().map(|r| r.id.clone()).collect();
        live_ids.extend(conf.tunnels.iter().map(|t| format!("tunnel:{}", t.id)));
        drop(conf);
        self.pacing_slots
            .lock()
            .retain(|id, _| live_ids.contains(id));
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

    /// Ghi 1 dòng vào nhật ký debug của tunnel (cap 30 dòng mới nhất).
    /// kind: "info" | "ok" | "error". Msg nên ngắn (<200 chars).
    pub fn log_tunnel_event(&self, tunnel_id: &str, kind: &str, msg: String) {
        let ev = crate::vpn::TunnelEvent {
            ts: chrono::Local::now().format("%H:%M:%S").to_string(),
            kind: kind.to_string(),
            msg: msg.chars().take(300).collect(),
        };
        let mut map = self.tunnel_events.lock();
        let queue = map.entry(tunnel_id.to_string()).or_default();
        queue.push_back(ev);
        while queue.len() > 30 {
            queue.pop_front();
        }
    }

    /// Snapshot toàn bộ event log cho IPC (clone nhẹ, gọi khi UI cần).
    pub fn tunnel_events_snapshot(
        &self,
    ) -> HashMap<String, Vec<crate::vpn::TunnelEvent>> {
        self.tunnel_events
            .lock()
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect()
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

    // Phase 1: match route + resolve key dưới write-lock NGẮN HẠN.
    // CRITICAL: guard PHẢI được thả trước mọi record_log/return phía dưới —
    // record_log lấy config.read(), cùng thread đang giữ write sẽ deadlock
    // parking_lot::RwLock (treo worker vĩnh viễn mỗi request 503).
    let (route_id, rule_target, rule_prefix, rule_tunnel, route_pace_ms, resolved_key, key_from_file, profile) = {
        let mut conf = state.config.write();
        let matched = match_route(&mut conf.routes, port, &path);

        match matched {
            Some(rule) => {
                // Phân biệt nguồn key (M1): key từ file mới được đếm lỗi/advance/đốt;
                // custom_auth_token cấu hình tay thì bỏ qua mọi thao tác file.
                let (key, from_file) = match rule.key_manager.resolve_active_key() {
                    Some(k) => (Some(k), true),
                    None => (rule.custom_auth_token.clone(), false),
                };
                // Snapshot owned để gọi get_route_fingerprint(&conf, &snapshot) mà không
                // giữ borrow &mut vào conf.routes cùng lúc với borrow &conf (E0502).
                let snapshot = rule.clone();
                let out = (
                    rule.id.clone(),
                    rule.target_base_url.clone(),
                    rule.path_prefix.clone(),
                    rule.tunnel_id.clone(),
                    rule.min_request_interval_ms,
                    key,
                    from_file,
                    conf.get_route_fingerprint(&snapshot),
                );
                out
            }
            None => {
                let err_msg = format!("Route Not Found on port {} for path {}", port, path);
                // Inspector thấy cả request lạc: log 404 với headers inbound (không đọc body
                // để tránh OOM vector; body để rỗng và UI hiển thị "<empty body>").
                let mut inbound_headers = Vec::new();
                for (k, v) in &headers {
                    inbound_headers.push((
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("<binary>").to_string(),
                    ));
                }
                state.record_log(RawTrafficLog {
                    id: req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    route_id: "unmatched".to_string(),
                    method: method.to_string(),
                    port,
                    path,
                    target_url: String::new(),
                    tunnel_id: String::new(),
                    key_used_preview: None,
                    status_code: 404,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    is_streaming: false,
                    raw_request_headers: inbound_headers,
                    raw_forwarded_headers: vec![],
                    raw_request_body: String::new(),
                    raw_response_headers: vec![],
                    raw_response_body: err_msg.clone(),
                    raw_forwarded_body: String::new(),
                    leaked_findings: vec![],
                });
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from(err_msg))
                    .unwrap();
            }
        }
    };

    // Phase 2: chọn tunnel dưới read-lock riêng (không còn write guard nào đang giữ).
    let selected_id = {
        let conf = state.config.read();
        match resolve_tunnel_id(&conf.tunnels, &rule_tunnel) {
            Some(id) => id,
            None => {
                let final_url = build_target_url(&rule_target, &rule_prefix, &path, &query);
                let err_msg = format!("All VPN tunnels are offline or unavailable. Request rejected to prevent real IP leak.");
                drop(conf);
                state.record_log(RawTrafficLog {
                    id: req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    route_id,
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
                    raw_forwarded_body: String::new(),
                    leaked_findings: vec![],
                });
                return Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::from(err_msg))
                    .unwrap();
            }
        }
    };

    let max_streams = {
        let conf = state.config.read();
        conf.tunnels
            .iter()
            .find(|t| t.id == selected_id)
            .map(|t| t.max_concurrent_streams)
            .unwrap_or(0)
    };
    let target_url = build_target_url(&rule_target, &rule_prefix, &path, &query);
    let (route_id, tunnel_id) = (route_id, selected_id);

    // Pacing 2 tầng (xem reserve_pacing_slot): endpoint riêng + tunnel chung.
    // Chạy TRƯỚC semaphore (không giữ permit khi sleep); sleep ngoài lock.
    // Cả 2 đều 0 (default) → bỏ qua hoàn toàn, zero overhead hành vi cũ.
    let tunnel_ms: u64 = {
        let conf = state.config.read();
        conf.tunnels
            .iter()
            .find(|t| t.id == tunnel_id)
            .map(|t| t.min_request_interval_ms)
            .unwrap_or(0)
    };
    if route_pace_ms > 0 || tunnel_ms > 0 {
        let wait = {
            let mut slots = state.pacing_slots.lock();
            reserve_pacing_slot(
                &mut slots,
                &route_id,
                &tunnel_id,
                route_pace_ms,
                tunnel_ms,
                tokio::time::Instant::now(),
            )
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

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

    // P1: che path local trên body TRƯỚC khi forward (mask_local_paths_in_body).
    // Chỉ re-encode khi thực sự có thay thế — body sạch đi nguyên bytes gốc.
    let masked_body = if profile.mask_local_paths_in_body {
        crate::fingerprint::patterns::mask_local_paths(&raw_request_body)
    } else {
        None
    };
    let forward_bytes: bytes::Bytes = match &masked_body {
        Some(masked) => bytes::Bytes::from(masked.clone().into_bytes()),
        None => body_bytes.clone(),
    };

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
    // Method lạ/không parse được thì 501 chứ KHÔNG ép thành GET trong im lặng —
    // ép GET có thể biến DELETE thành GET (nguy hiểm semantics), proxy phải trung thực.
    let upstream_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            let err_msg = format!("Unsupported HTTP method: {}", method);
            return Response::builder()
                .status(StatusCode::NOT_IMPLEMENTED)
                .body(Body::from(err_msg))
                .unwrap();
        }
    };
    let mut req_builder = client.request(upstream_method, &target_url);

    let key_preview = resolved_key
        .as_deref()
        .map(crate::proxy::key_manager::EndpointKeyManager::preview_key);

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
                        raw_forwarded_body: masked_body.clone().unwrap_or_default(),
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

    req_builder = req_builder.body(forward_bytes);

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
        key_from_file,
        raw_request_headers,
        raw_forwarded_headers,
        raw_request_body,
        // Body thực tế đã forward (sau mask, hoặc rỗng nếu không mask) — UI hiện
        // riêng mục này để không nhầm với raw_request_body (những gì client gửi).
        forwarded_body: masked_body.unwrap_or_default(),
        leaks,
        start_time,
        permit,
    };

    send_and_handle_upstream(req_builder, Arc::clone(&state), ctx).await
}
