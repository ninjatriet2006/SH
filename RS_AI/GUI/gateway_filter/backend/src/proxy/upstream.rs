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
    /// Bộ chuyển đổi giao thức đang kích hoạt (Protocol Adapter)
    pub active_adapter: crate::config::ProtocolAdapter,
    /// Tên model client đã yêu cầu (để format chuẩn lại OpenAI response)
    pub requested_model: Option<String>,
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

            let is_adapted = ctx.active_adapter != crate::config::ProtocolAdapter::None;

            // Error Code Masking & Preservation:
            // Nếu upstream trả 401 hoặc 403, chuyển mã lỗi thành 502 Bad Gateway khi trả về client
            // để bảo vệ IDE/Client (Cursor/Cline) không tự động xóa credential hoặc popup bắt login lại,
            // trong khi vẫn giữ nguyên vẹn 100% nội dung body JSON chi tiết từ upstream cho người dùng đọc.
            let client_status = if status_code == 401 || status_code == 403 {
                StatusCode::BAD_GATEWAY
            } else {
                StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            };

            let mut resp_builder = Response::builder().status(client_status);

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
                    || (is_adapted && (k_lower == "content-encoding" || k_lower == "content-type"))
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

                // Chunk Idle Timeout: Quá 45s không có chunk mới (VPN rớt ngầm/stall) -> tự động ngắt
                // an toàn để giải phóng StreamGuard và Semaphore permit, chống tê liệt gateway!
                let timed_stream: std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + Send>> =
                    Box::pin(futures_util::stream::unfold(stream, |mut s| async move {
                        match tokio::time::timeout(std::time::Duration::from_secs(45), s.next()).await {
                            Ok(Some(chunk_res)) => Some((chunk_res, s)),
                            Ok(None) => None,
                            Err(_) => {
                                eprintln!(">> Upstream SSE stream idle timeout (45s) - aborting to prevent permit leak");
                                None
                            }
                        }
                    }));

                // Chuyển đổi SSE qua Protocol Adapter tương ứng
                let stream_to_tee: std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + Send>> = match ctx.active_adapter {
                    crate::config::ProtocolAdapter::OpenAiTo1Min => {
                        let model = ctx.requested_model.clone().unwrap_or_else(|| "gpt-4o".to_string());
                        let transformer = crate::proxy::adapters::onemin::OneMinSseTransformer::new(&model);
                        struct OneMinState {
                            stream: std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
                            transformer: crate::proxy::adapters::onemin::OneMinSseTransformer,
                            finished: bool,
                        }
                        let init_state = OneMinState {
                            stream: timed_stream,
                            transformer,
                            finished: false,
                        };
                        let converted = futures_util::stream::unfold(init_state, |mut s| async move {
                            if s.finished {
                                return None;
                            }
                            loop {
                                match s.stream.next().await {
                                    Some(Ok(chunk)) => {
                                        let converted_bytes = s.transformer.feed_bytes(&chunk);
                                        if !converted_bytes.is_empty() {
                                            return Some((Ok(Bytes::from(converted_bytes)), s));
                                        }
                                    }
                                    Some(Err(e)) => {
                                        s.finished = true;
                                        return Some((Err(e), s));
                                    }
                                    None => {
                                        s.finished = true;
                                        let final_bytes = s.transformer.finish();
                                        if !final_bytes.is_empty() {
                                            return Some((Ok(Bytes::from(final_bytes)), s));
                                        } else {
                                            return None;
                                        }
                                    }
                                }
                            }
                        });
                        Box::pin(converted)
                    }
                    crate::config::ProtocolAdapter::OpenAiToAnthropic => {
                        let model = ctx.requested_model.clone().unwrap_or_else(|| "claude-3-5-sonnet".to_string());
                        let transformer = crate::proxy::adapters::anthropic::AnthropicSseTransformer::new(&model);
                        struct AnthropicState {
                            stream: std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
                            transformer: crate::proxy::adapters::anthropic::AnthropicSseTransformer,
                            finished: bool,
                        }
                        let init_state = AnthropicState {
                            stream: timed_stream,
                            transformer,
                            finished: false,
                        };
                        let converted = futures_util::stream::unfold(init_state, |mut s| async move {
                            if s.finished {
                                return None;
                            }
                            loop {
                                match s.stream.next().await {
                                    Some(Ok(chunk)) => {
                                        let converted_bytes = s.transformer.feed_bytes(&chunk);
                                        if !converted_bytes.is_empty() {
                                            return Some((Ok(Bytes::from(converted_bytes)), s));
                                        }
                                    }
                                    Some(Err(e)) => {
                                        s.finished = true;
                                        return Some((Err(e), s));
                                    }
                                    None => {
                                        s.finished = true;
                                        let final_bytes = s.transformer.finish();
                                        if !final_bytes.is_empty() {
                                            return Some((Ok(Bytes::from(final_bytes)), s));
                                        } else {
                                            return None;
                                        }
                                    }
                                }
                            }
                        });
                        Box::pin(converted)
                    }
                    crate::config::ProtocolAdapter::None => timed_stream,
                };

                let teed_stream = stream_to_tee.map(move |chunk_res| {
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
                        self.state.logs.update_response_body(&self.req_id, final_str.clone());
                        let log_path = crate::monitor::get_traffic_log_path();
                        crate::monitor::spawn_disk_update_response_body(log_path, self.req_id.clone(), final_str);
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

                if is_adapted {
                    resp_builder = resp_builder.header("content-type", "text/event-stream; charset=utf-8");
                    resp_builder = resp_builder.header("cache-control", "no-cache");
                }

                resp_builder.body(body).unwrap()
            } else {
                let resp_headers = upstream_resp.headers().clone();
                let resp_bytes = upstream_resp.bytes().await.unwrap_or_else(|_| Bytes::new());

                // Chuyển đổi Non-Streaming response từ Protocol Adapter sang OpenAI JSON
                let final_bytes = match ctx.active_adapter {
                    crate::config::ProtocolAdapter::OpenAiTo1Min if status_code == 200 => {
                        let model = ctx.requested_model.as_deref().unwrap_or("gpt-4o");
                        match crate::proxy::adapters::onemin::transform_response_json(&resp_bytes, model) {
                            Ok(transformed) => Bytes::from(transformed),
                            Err(e) => {
                                eprintln!(">> 1min.ai transform_response_json fallback: {}, returning raw bytes", e);
                                resp_bytes
                            }
                        }
                    }
                    crate::config::ProtocolAdapter::OpenAiToAnthropic if status_code == 200 => {
                        let model = ctx.requested_model.as_deref().unwrap_or("claude-3-5-sonnet");
                        match crate::proxy::adapters::anthropic::transform_response_json(&resp_bytes, model) {
                            Ok(transformed) => Bytes::from(transformed),
                            Err(e) => {
                                eprintln!(">> Anthropic transform_response_json fallback: {}, returning raw bytes", e);
                                resp_bytes
                            }
                        }
                    }
                    _ => resp_bytes,
                };

                let truncated_resp_body = format_logged_body(&final_bytes, &resp_headers);
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

                if is_adapted && status_code == 200 {
                    resp_builder = resp_builder.header("content-type", "application/json; charset=utf-8");
                }

                resp_builder.body(Body::from(final_bytes)).unwrap()
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
