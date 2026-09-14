use std::collections::HashSet;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::proxy::{handle_route_request, AppState};

pub struct ListenerManager {
    handles: parking_lot::RwLock<Vec<JoinHandle<()>>>,
}

impl ListenerManager {
    pub fn new() -> Self {
        Self {
            handles: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn sync_listeners(&self, app_state: Arc<AppState>) {
        // Stop current listeners
        let mut guard = self.handles.write();
        for handle in guard.drain(..) {
            handle.abort();
        }

        // Collect distinct ports from active routes
        let ports: HashSet<u16> = {
            let conf = app_state.config.read();
            conf.routes.iter().filter(|r| r.enabled).map(|r| r.port).collect()
        };

        for port in ports {
            let state_clone = Arc::clone(&app_state);
            let handle = tokio::spawn(async move {
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
                        let _ = axum::serve(listener, app).await;
                    }
                    Err(e) => {
                        eprintln!(">> Failed to bind gateway port {}: {}", addr, e);
                    }
                }
            });
            guard.push(handle);
        }
    }
}
