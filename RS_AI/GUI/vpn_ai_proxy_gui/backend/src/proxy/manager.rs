use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::async_runtime::JoinHandle;
use tauri::Emitter;

use crate::proxy::{handle_route_request, AppState};

pub struct ListenerManager {
    /// Mỗi port một task listener. Chỉ restart khi tập port thực sự thay đổi —
    /// tránh abort toàn bộ listener (làm đứt request đang chạy) mỗi lần save config.
    listeners: parking_lot::RwLock<HashMap<u16, JoinHandle<()>>>,
}

impl ListenerManager {
    pub fn new() -> Self {
        Self {
            listeners: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    pub fn sync_listeners(&self, app_state: Arc<AppState>, app_handle: Option<tauri::AppHandle>) {
        // Collect distinct ports from active routes
        let ports: HashSet<u16> = {
            let conf = app_state.config.read();
            conf.routes.iter().filter(|r| r.enabled).map(|r| r.port).collect()
        };

        let mut guard = self.listeners.write();

        // 1. Dừng listener của port không còn route enabled nào dùng
        let stale: Vec<u16> = guard.keys().copied().filter(|p| !ports.contains(p)).collect();
        for port in stale {
            if let Some(handle) = guard.remove(&port) {
                handle.abort();
            }
            // Route bị tắt/không bind được → phản ánh trạng thái thật xuống config
            let mut conf = app_state.config.write();
            let mut changed = false;
            for r in conf.routes.iter_mut().filter(|r| r.port == port) {
                if r.status != crate::proxy::RouteStatus::Inactive {
                    r.status = crate::proxy::RouteStatus::Inactive;
                    r.last_error = Some("Listener stopped (route disabled or port released)".to_string());
                    changed = true;
                }
            }
            if changed {
                let _ = conf.save_to_disk();
            }
        }

        // 2. Chỉ spawn listener cho port CHƯA chạy — port đang chạy giữ nguyên
        //    (handler đọc config trực tiếp nên đổi target/key/tunnel không cần rebind)
        for port in ports {
            if guard.contains_key(&port) {
                continue;
            }

            let state_clone = Arc::clone(&app_state);
            let handle_clone = app_handle.clone();
            let handle = tauri::async_runtime::spawn(async move {
                let app = axum::Router::new()
                    .fallback({
                        let state = Arc::clone(&state_clone);
                        move |method, uri, headers, req| {
                            handle_route_request(port, axum::extract::State(state), method, uri, headers, req)
                        }
                    });

                let addr = format!("127.0.0.1:{}", port);
                match tokio::net::TcpListener::bind(&addr).await {
                    Ok(listener) => {
                        println!(">> Gateway port bound successfully: http://{}", addr);
                        // Đánh dấu Route là Active nếu bind thành công
                        {
                            let mut conf = state_clone.config.write();
                            let mut changed = false;
                            for r in conf.routes.iter_mut().filter(|r| r.port == port) {
                                r.status = crate::proxy::RouteStatus::Active;
                                r.last_error = None;
                                changed = true;
                            }
                            if changed {
                                let _ = conf.save_to_disk();
                            }
                        }
                        if let Some(h) = &handle_clone {
                            let _ = h.emit("route-status-changed", serde_json::json!({ "port": port }));
                        }
                        let _ = axum::serve(listener, app).await;
                    }
                    Err(e) => {
                        eprintln!(">> Failed to bind gateway port {}: {}", addr, e);
                        // Đánh dấu Route là Inactive nếu bind thất bại (trùng port...)
                        {
                            let mut conf = state_clone.config.write();
                            let mut changed = false;
                            for r in conf.routes.iter_mut().filter(|r| r.port == port) {
                                r.status = crate::proxy::RouteStatus::Inactive;
                                r.last_error = Some(e.to_string());
                                changed = true;
                            }
                            if changed {
                                let _ = conf.save_to_disk();
                            }
                        }
                        if let Some(h) = handle_clone {
                            let _ = h.emit(
                                "endpoint-bind-error",
                                serde_json::json!({
                                    "port": port,
                                    "error": e.to_string(),
                                }),
                            );
                        }
                    }
                }
            });
            guard.insert(port, handle);
        }
    }
}
