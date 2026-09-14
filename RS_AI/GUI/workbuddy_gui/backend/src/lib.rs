/*
[INTEGRITY NOTES]
- Purpose: Main library for WorkBuddy GUI (Tauri backend).
- Responsibility: Register all Tauri commands and initialize the app.
- Structure:
  + `api`: Tauri command endpoints communicating with Frontend.
  + `core`: Domain logic ported from Go WorkBuddy2API.
*/

pub mod api;
pub mod core;
pub mod ipc;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    let builder = tauri::Builder::default();
    let builder = builder.plugin(tauri_plugin_dialog::init());
    #[cfg(debug_assertions)]
    let builder = builder.setup(|app| {
        app.handle().plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )?;
        Ok(())
    });

    builder
        .setup(|app| {
            if let Ok(resource_dir) = app.path().resource_dir() {
                std::env::set_var("WORKBUDDY_RESOURCE_DIR", resource_dir);
            }
            // Initialize gateway state (will be populated from config)
            app.manage(core::gateway_state::GatewayState::default());
            app.manage(core::runtime::RuntimeState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ==================
            // GATEWAY API
            // ==================
            api::gateway::get_gateway_status,
            api::gateway::start_gateway,
            api::gateway::stop_gateway,
            // ==================
            // ACCOUNTS API
            // ==================
            api::accounts::list_accounts,
            api::accounts::get_account_status,
            api::accounts::add_account,
            api::accounts::remove_account,
            api::accounts::disable_account,
            api::accounts::enable_account,
            // ==================
            // CONFIG API
            // ==================
            api::config_api::get_config,
            api::config_api::save_config,
            api::config_api::get_default_config,
            // ==================
            // SCHEDULER API
            // ==================
            api::scheduler_api::run_checkin_now,
            api::scheduler_api::run_travel_now,
            api::scheduler_api::run_activity_now,
            api::scheduler_api::run_keepalive_now,
            // ==================
            // SETTINGS / LANG / THEME / FONT
            // ==================
            api::settings::get_gui_settings,
            api::settings::save_gui_settings,
            api::lang::get_available_langs,
            api::lang::get_lang_content,
            api::theme::get_available_themes,
            api::font::get_available_fonts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
