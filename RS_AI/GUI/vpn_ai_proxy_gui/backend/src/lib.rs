pub mod fingerprint;
pub mod monitor;
pub mod proxy;
pub mod vpn;

use std::sync::Arc;
use tauri::State;

use crate::monitor::RawTrafficLog;
use crate::proxy::manager::ListenerManager;
use crate::proxy::{AppState, GatewayConfig, RouteRule};
use crate::vpn::{OutboundTunnel, TunnelManager, TunnelTestResult};

pub struct ServerContext {
    pub app_state: Arc<AppState>,
    pub listener_mgr: Arc<ListenerManager>,
}

mod commands {
    use super::*;

    #[tauri::command]
    pub fn get_config(ctx: State<'_, ServerContext>) -> GatewayConfig {
        let mut conf = ctx.app_state.config.write();
        for r in &mut conf.routes {
            r.key_manager.refresh_metadata();
        }
        conf.clone()
    }

    #[tauri::command]
    pub fn save_config(new_config: GatewayConfig, ctx: State<'_, ServerContext>) -> Result<(), String> {
        let _ = new_config.save_to_disk();
        {
            let mut conf = ctx.app_state.config.write();
            *conf = new_config;
            for r in &mut conf.routes {
                r.key_manager.refresh_metadata();
            }
        }
        ctx.app_state.refresh_clients();
        ctx.listener_mgr.sync_listeners(Arc::clone(&ctx.app_state));
        Ok(())
    }

    #[tauri::command]
    pub fn add_or_update_tunnel(tunnel: OutboundTunnel, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(pos) = conf.tunnels.iter().position(|t| t.id == tunnel.id) {
                conf.tunnels[pos] = tunnel;
            } else {
                conf.tunnels.push(tunnel);
            }
            let _ = conf.save_to_disk();
        }
        ctx.app_state.refresh_clients();
        Ok(())
    }

    #[tauri::command]
    pub fn delete_tunnel(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            conf.tunnels.retain(|t| t.id != tunnel_id);
            let _ = conf.save_to_disk();
        }
        ctx.app_state.refresh_clients();
        Ok(())
    }

    #[tauri::command]
    pub fn add_or_update_route(mut route: RouteRule, ctx: State<'_, ServerContext>) -> Result<(), String> {
        route.key_manager.refresh_metadata();
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(pos) = conf.routes.iter().position(|r| r.id == route.id) {
                conf.routes[pos] = route;
            } else {
                conf.routes.push(route);
            }
            let _ = conf.save_to_disk();
        }
        ctx.listener_mgr.sync_listeners(Arc::clone(&ctx.app_state));
        Ok(())
    }

    #[tauri::command]
    pub fn delete_route(route_id: String, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            conf.routes.retain(|r| r.id != route_id);
            let _ = conf.save_to_disk();
        }
        ctx.listener_mgr.sync_listeners(Arc::clone(&ctx.app_state));
        Ok(())
    }

    #[tauri::command]
    pub fn advance_endpoint_key(route_id: String, ctx: State<'_, ServerContext>) -> Result<Option<String>, String> {
        Ok(ctx.app_state.advance_route_key(&route_id))
    }

    #[tauri::command]
    pub async fn test_single_tunnel(tunnel: OutboundTunnel) -> Result<TunnelTestResult, String> {
        Ok(TunnelManager::test_tunnel(&tunnel).await)
    }

    #[tauri::command]
    pub fn get_raw_traffic(ctx: State<'_, ServerContext>) -> Vec<RawTrafficLog> {
        ctx.app_state.logs.get_all()
    }

    #[tauri::command]
    pub fn clear_logs(ctx: State<'_, ServerContext>) {
        ctx.app_state.logs.clear();
    }
}

pub fn run() {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    let initial_config = GatewayConfig::load_or_default();
    let app_state = Arc::new(AppState::new(initial_config));
    let listener_mgr = Arc::new(ListenerManager::new());

    let server_ctx = ServerContext {
        app_state: Arc::clone(&app_state),
        listener_mgr: Arc::clone(&listener_mgr),
    };

    tauri::Builder::default()
        .manage(server_ctx)
        .setup(move |_app| {
            // Spawn dynamic listeners safely inside Tauri async runtime
            listener_mgr.sync_listeners(Arc::clone(&app_state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::add_or_update_tunnel,
            commands::delete_tunnel,
            commands::add_or_update_route,
            commands::delete_route,
            commands::advance_endpoint_key,
            commands::test_single_tunnel,
            commands::get_raw_traffic,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
