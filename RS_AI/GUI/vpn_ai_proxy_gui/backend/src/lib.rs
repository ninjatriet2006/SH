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
    pub fn toggle_tunnel(tunnel_id: String, enabled: bool, ctx: State<'_, ServerContext>) -> Result<(), String> {
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(pos) = conf.tunnels.iter().position(|t| t.id == tunnel_id) {
                conf.tunnels[pos].enabled = enabled;
                let _ = conf.save_to_disk();
            } else {
                return Err(format!("Tunnel [{}] not found", tunnel_id));
            }
        }
        ctx.app_state.refresh_clients();
        Ok(())
    }

    #[tauri::command]
    pub async fn start_tunnel_process(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
        let (cmd_opt, tunnel_clone) = {
            let conf = ctx.app_state.config.read();
            let t = conf.tunnels.iter().find(|t| t.id == tunnel_id).cloned();
            match t {
                Some(tunnel) => (tunnel.start_command.clone(), tunnel),
                None => return Err(format!("Tunnel [{}] not found", tunnel_id)),
            }
        };

        let cmd = cmd_opt.ok_or_else(|| "No start command configured for this tunnel".to_string())?;
        let output = TunnelManager::run_tunnel_command(&cmd)?;

        // Chờ 500ms để process bind cổng rồi test kết nối
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let test_res = TunnelManager::test_tunnel(&tunnel_clone).await;

        {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                t.status = if test_res.success {
                    crate::vpn::TunnelStatus::Online
                } else {
                    crate::vpn::TunnelStatus::Offline
                };
                t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                t.last_error = test_res.error.clone();
                if test_res.success {
                    t.last_exit_ip = test_res.exit_ip;
                    t.last_latency_ms = test_res.latency_ms;
                }
                let _ = conf.save_to_disk();
            }
        }

        Ok(format!("Command executed. Result: {}", output.trim()))
    }

    #[tauri::command]
    pub async fn stop_tunnel_process(tunnel_id: String, ctx: State<'_, ServerContext>) -> Result<String, String> {
        let cmd_opt = {
            let conf = ctx.app_state.config.read();
            conf.tunnels.iter().find(|t| t.id == tunnel_id).and_then(|t| t.stop_command.clone())
        };

        let cmd = cmd_opt.ok_or_else(|| "No stop command configured for this tunnel".to_string())?;
        let output = TunnelManager::run_tunnel_command(&cmd)?;

        {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel_id) {
                t.status = crate::vpn::TunnelStatus::Offline;
                t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                t.last_error = Some("Stopped by user".to_string());
                let _ = conf.save_to_disk();
            }
        }

        Ok(format!("Command executed. Result: {}", output.trim()))
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
    pub async fn test_single_tunnel(tunnel: OutboundTunnel, ctx: State<'_, ServerContext>) -> Result<TunnelTestResult, String> {
        let res = TunnelManager::test_tunnel(&tunnel).await;
        // Persist kết quả test vào config state & file json
        {
            let mut conf = ctx.app_state.config.write();
            if let Some(t) = conf.tunnels.iter_mut().find(|t| t.id == tunnel.id) {
                t.status = if res.success {
                    crate::vpn::TunnelStatus::Online
                } else {
                    crate::vpn::TunnelStatus::Offline
                };
                t.last_checked_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                t.last_error = res.error.clone();
                if res.success {
                    t.last_exit_ip = res.exit_ip.clone();
                    t.last_latency_ms = res.latency_ms;
                }
                let _ = conf.save_to_disk();
            }
        }
        Ok(res)
    }

    #[tauri::command]
    pub fn get_raw_traffic(ctx: State<'_, ServerContext>) -> Vec<RawTrafficLog> {
        ctx.app_state.logs.get_all()
    }

    #[tauri::command]
    pub fn clear_logs(ctx: State<'_, ServerContext>) {
        ctx.app_state.logs.clear();
    }

    #[tauri::command]
    pub fn generate_key_file(endpoint_name: String) -> Result<String, String> {
        let base_dir = if let Some(home) = std::env::var_os("HOME") {
            std::path::PathBuf::from(home)
                .join(".config")
                .join("vpn_ai_proxy_gui")
                .join("keys")
        } else {
            std::path::PathBuf::from("keys")
        };

        std::fs::create_dir_all(&base_dir)
            .map_err(|e| format!("Failed to create keys directory: {}", e))?;

        let safe_name: String = endpoint_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let file_name = format!("{}_keys.txt", if safe_name.is_empty() { "default" } else { &safe_name });
        let file_path = base_dir.join(file_name);

        if !file_path.exists() {
            std::fs::write(&file_path, "# Paste API keys here, one per line\n")
                .map_err(|e| format!("Failed to write key file: {}", e))?;
        }

        Ok(file_path.to_string_lossy().to_string())
    }

    #[tauri::command]
    pub fn open_key_file(file_path: String) -> Result<(), String> {
        let p = std::path::Path::new(&file_path);
        if !p.exists() {
            return Err(format!("File does not exist: {}", file_path));
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(&file_path).spawn()
                .map_err(|e| format!("Failed to open file with xdg-open: {}", e))?;
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd").args(["/C", "start", "", &file_path]).spawn()
                .map_err(|e| format!("Failed to open file with cmd start: {}", e))?;
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(&file_path).spawn()
                .map_err(|e| format!("Failed to open file with open: {}", e))?;
        }
        Ok(())
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
            commands::toggle_tunnel,
            commands::delete_tunnel,
            commands::add_or_update_route,
            commands::delete_route,
            commands::advance_endpoint_key,
            commands::test_single_tunnel,
            commands::start_tunnel_process,
            commands::stop_tunnel_process,
            commands::get_raw_traffic,
            commands::clear_logs,
            commands::generate_key_file,
            commands::open_key_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
