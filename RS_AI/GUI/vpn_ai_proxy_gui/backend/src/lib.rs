pub mod commands;
pub mod config;
pub mod fingerprint;
pub mod monitor;
pub mod proxy;
pub mod state;
pub mod vpn;
pub mod workers;

use std::sync::Arc;

use crate::config::GatewayConfig;
use crate::proxy::manager::ListenerManager;
use crate::proxy::AppState;
pub use crate::state::ServerContext;

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
        .setup(move |app| {
            let handle = app.handle().clone();
            *app_state.app_handle.write() = Some(handle.clone());
            listener_mgr.sync_listeners(Arc::clone(&app_state), Some(handle.clone()));

            // Khởi động Boot auto-start tunnel worker
            let boot_ctx = ServerContext {
                app_state: Arc::clone(&app_state),
                listener_mgr: Arc::clone(&listener_mgr),
            };
            workers::spawn_boot_tunnel_worker(boot_ctx, handle.clone());

            // Khởi động Proactive VPN check worker (đi tuần mỗi 60s)
            workers::spawn_proactive_vpn_worker(Arc::clone(&app_state), handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::add_or_update_tunnel,
            commands::toggle_tunnel,
            commands::toggle_route,
            commands::delete_tunnel,
            commands::add_or_update_route,
            commands::delete_route,
            commands::advance_endpoint_key,
            commands::start_tunnel_process,
            commands::stop_tunnel_process,
            commands::get_active_cli_tunnel,
            commands::force_stop_tunnel,
            commands::test_single_tunnel,
            commands::get_raw_traffic,
            commands::clear_logs,
            commands::generate_key_file,
            commands::open_key_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
