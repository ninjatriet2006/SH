pub mod fingerprint;
pub mod monitor;
pub mod proxy;
pub mod vpn;

use std::sync::Arc;
use tauri::State;

use crate::monitor::RequestLog;
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
        ctx.app_state.config.read().clone()
    }

    #[tauri::command]
    pub fn save_config(new_config: GatewayConfig, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            *conf = new_config;
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
        }
        ctx.app_state.refresh_clients();
        Ok(())
    }

    #[tauri::command]
    pub fn delete_tunnel(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            conf.tunnels.retain(|t| t.id != tunnel_id);
        }
        ctx.app_state.refresh_clients();
        Ok(())
    }

    #[tauri::command]
    pub fn add_or_update_route(route: RouteRule, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(pos) = conf.routes.iter().position(|r| r.id == route.id) {
                conf.routes[pos] = route;
            } else {
                conf.routes.push(route);
            }
        }
        ctx.listener_mgr.sync_listeners(Arc::clone(&ctx.app_state));
        Ok(())
    }

    #[tauri::command]
    pub fn delete_route(route_id: String, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            conf.routes.retain(|r| r.id != route_id);
        }
        ctx.listener_mgr.sync_listeners(Arc::clone(&ctx.app_state));
        Ok(())
    }

    #[tauri::command]
    pub async fn test_single_tunnel(tunnel: OutboundTunnel) -> Result<TunnelTestResult, String> {
        Ok(TunnelManager::test_tunnel(&tunnel).await)
    }

    #[tauri::command]
    pub fn get_logs(ctx: State<'_, ServerContext>) -> Vec<RequestLog> {
        ctx.app_state.logs.get_all()
    }

    #[tauri::command]
    pub fn clear_logs(ctx: State<'_, ServerContext>) {
        ctx.app_state.logs.clear();
    }
}

pub fn run() {
    let initial_config = GatewayConfig::default();
    let app_state = Arc::new(AppState::new(initial_config));
    let listener_mgr = Arc::new(ListenerManager::new());

    // Spawn dynamic listeners for all initial routes
    listener_mgr.sync_listeners(Arc::clone(&app_state));

    let server_ctx = ServerContext {
        app_state,
        listener_mgr,
    };

    tauri::Builder::default()
        .manage(server_ctx)
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::add_or_update_tunnel,
            commands::delete_tunnel,
            commands::add_or_update_route,
            commands::delete_route,
            commands::test_single_tunnel,
            commands::get_logs,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
