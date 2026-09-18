use axum::{
    body::Body,
    http::{Method, Response, StatusCode},
};
use bytes::Bytes;
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Instant;

use crate::fingerprint::LeakFinding;
use crate::monitor::RawTrafficLog;
use crate::proxy::rotation::{handle_error_session_rotation, handle_success_reset};
use crate::proxy::{format_logged_body, truncate_log_body, AppState};

pub struct RequestLogContext {
    pub req_id: String,
    pub route_id: String,
    pub method: Method,
    pub port: u16,
    pub path: String,
    pub target_url: String,
    pub tunnel_id: String,
    pub key_preview: Option<String>,
    pub resolved_key: Option<String>,
    /// true khi key lấy từ file (được đếm lỗi/advance/đốt), false khi là custom_auth_token (M1).
    pub key_from_file: bool,
    pub raw_request_headers: Vec<(String, String)>,
    pub raw_forwarded_headers: Vec<(String, String)>,
    pub raw_request_body: String,
    /// Body thực tế đã forward upstream (sau mask local paths, hoặc rỗng nếu không mask).
    pub forwarded_body: String,
    pub leaks: Vec<LeakFinding>,
    pub start_time: Instant,
    pub permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

/// Gửi request sang upstream server và xử lý phản hồi (hỗ trợ SSE Streaming hoặc Buffered Body)
pub async fn send_and_handle_upstream(
    req_builder: reqwest::RequestBuilder,
    state: Arc<AppState>,
    ctx: RequestLogContext,
) -> Response<Body> {
    let response_result = req_builder.send().await;
    let duration_ms = ctx.start_time.elapsed().as_millis() as u64;

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

            // 100% Raw Response Headers (Lọc bỏ hop-by-hop headers tránh lỗi parser HTTP client)
            let mut raw_response_headers = Vec::new();
            for (k, v) in upstream_resp.headers() {
                let v_str = v.to_str().unwrap_or("").to_string();
                let k_lower = k.as_str().to_lowercase();
                raw_response_headers.push((k.as_str().to_string(), v_str));

                if k_lower == "transfer-encoding"
                    || k_lower == "content-length"
                    || k_lower == "connection"
                    || k_lower == "keep-alive"
                {
                    continue;
                }
                resp_builder = resp_builder.header(k.as_str(), v.as_bytes());
            }

            if is_streaming {
                let stream = upstream_resp.bytes_stream();
                let accumulated = Arc::new(parking_lot::Mutex::new(Vec::<u8>::new()));
                let acc_clone = Arc::clone(&accumulated);
                let state_clone = Arc::clone(&state);
                let req_id_clone = ctx.req_id.clone();

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

                // Wrap stream drop/completion để cập nhật body log
                // Giữ permit trong guard để concurrency limit có hiệu lực suốt thời gian stream SSE
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
                    _permit: ctx.permit,
                };

                let guarded_stream = teed_stream.map(move |item| {
                    let _g = &guard;
                    item
                });

                let body = Body::from_stream(guarded_stream);
                let truncated_req_body = truncate_log_body(&ctx.raw_request_body, 2048);

                state.record_log(RawTrafficLog {
                    id: ctx.req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    route_id: ctx.route_id,
                    method: ctx.method.to_string(),
                    port: ctx.port,
                    path: ctx.path,
                    target_url: ctx.target_url,
                    tunnel_id: ctx.tunnel_id,
                    key_used_preview: ctx.key_preview,
                    status_code,
                    duration_ms,
                    is_streaming: true,
                    raw_request_headers: ctx.raw_request_headers,
                    raw_forwarded_headers: ctx.raw_forwarded_headers,
                    raw_request_body: truncated_req_body,
                    raw_response_headers,
                    raw_response_body: "[Streaming SSE Response Live Flow]".to_string(),
                    raw_forwarded_body: truncate_log_body(&ctx.forwarded_body, 2048),
                    leaked_findings: ctx.leaks,
                });

                resp_builder.body(body).unwrap()
            } else {
                let resp_headers = upstream_resp.headers().clone();
                let resp_bytes = upstream_resp.bytes().await.unwrap_or_else(|_| Bytes::new());
                let truncated_resp_body = format_logged_body(&resp_bytes, &resp_headers);
                let truncated_req_body = truncate_log_body(&ctx.raw_request_body, 2048);

                state.record_log(RawTrafficLog {
                    id: ctx.req_id,
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    route_id: ctx.route_id.clone(),
                    method: ctx.method.to_string(),
                    port: ctx.port,
                    path: ctx.path,
                    target_url: ctx.target_url,
                    tunnel_id: ctx.tunnel_id,
                    key_used_preview: ctx.key_preview,
                    status_code,
                    duration_ms,
                    is_streaming: false,
                    raw_request_headers: ctx.raw_request_headers,
                    raw_forwarded_headers: ctx.raw_forwarded_headers,
                    raw_request_body: truncated_req_body,
                    raw_response_headers,
                    raw_response_body: truncated_resp_body,
                    raw_forwarded_body: truncate_log_body(&ctx.forwarded_body, 2048),
                    leaked_findings: ctx.leaks,
                });

                // Key Filtering & Session Rotation khi lỗi 401/403 (đốt key chỉ khi
                // chạm ngưỡng max_key_failures); response 2xx reset bộ đếm của key.
                handle_error_session_rotation(
                    &state,
                    &ctx.route_id,
                    status_code,
                    ctx.resolved_key.as_deref(),
                    ctx.key_from_file,
                );
                handle_success_reset(
                    &state,
                    &ctx.route_id,
                    status_code,
                    ctx.resolved_key.as_deref(),
                    ctx.key_from_file,
                );

                resp_builder.body(Body::from(resp_bytes)).unwrap()
            }
        }
        Err(err) => {
            let err_msg = format!("Tunnel [{}] upstream connection error: {}", ctx.tunnel_id, err);

            // Failover bền vững: đánh dấu tunnel rớt mạng để request sau chuyển sang tunnel khác
            {
                let mut conf = state.config.write();
                if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == ctx.tunnel_id) {
                    t.status = crate::vpn::TunnelStatus::Offline;
                    t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                    t.last_error = Some(format!("Upstream connection error: {}", err));
                    // Tunnel đã rớt: IP cũ không còn giá trị, xóa để tránh ghost IP xanh trên UI.
                    t.last_exit_ip = None;
                    t.last_latency_ms = None;
                    let _ = conf.save_to_disk();
                }
            }
            state.log_tunnel_event(
                &ctx.tunnel_id,
                "error",
                format!("Upstream rớt, đánh Offline: {}", err),
            );
            if let Some(ref handle) = *state.app_handle.read() {
                use tauri::Emitter;
                let _ = handle.emit("tunnel-status-changed", serde_json::json!({ "tunnel_id": ctx.tunnel_id, "success": false }));
            }

            state.record_log(RawTrafficLog {
                id: ctx.req_id,
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                route_id: ctx.route_id,
                method: ctx.method.to_string(),
                port: ctx.port,
                path: ctx.path,
                target_url: ctx.target_url,
                tunnel_id: ctx.tunnel_id,
                key_used_preview: ctx.key_preview,
                status_code: 502,
                duration_ms,
                is_streaming: false,
                raw_request_headers: ctx.raw_request_headers,
                raw_forwarded_headers: ctx.raw_forwarded_headers,
                raw_request_body: truncate_log_body(&ctx.raw_request_body, 2048),
                raw_response_headers: vec![],
                raw_response_body: err_msg.clone(),
                raw_forwarded_body: truncate_log_body(&ctx.forwarded_body, 2048),
                leaked_findings: ctx.leaks,
            });

            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(err_msg))
                .unwrap()
        }
    }
}
