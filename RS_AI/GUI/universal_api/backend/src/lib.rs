/*
[INTEGRITY NOTES]
- Purpose: Main library for Universal API (Tauri backend).
- Responsibility: Register all Tauri commands and initialize the app.
- Structure:
  + `api`: Tauri command endpoints communicating with Frontend.
  + `core`: Domain logic (CodeBuddy gateway + anti-api bridge).
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
                std::env::set_var("UNIVERSAL_API_RESOURCE_DIR", resource_dir);
            }
            // Initialize gateway state (will be populated from config)
            app.manage(core::gateway_state::GatewayState::default());
            // Nạp config đã lưu (config.json + WB2A_* env). Trước đây RuntimeState
            // luôn dùng default nên mọi setting mất sau mỗi lần mở app.
            app.manage(core::runtime::RuntimeState::with_config(
                core::config::load_boot_config(),
            ));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ==================
            // GATEWAY API
            // ==================
            api::gateway::get_gateway_status,
            api::gateway::start_gateway,
            api::gateway::stop_gateway,
            api::gateway::get_debug_traces,
            api::gateway::get_serving_account,
            api::gateway::refresh_gateway_models,
            // ==================
            // ACCOUNTS API
            // ==================
            api::accounts::list_accounts,
            api::accounts::get_account_status,
            api::accounts::add_account,
            api::accounts::remove_account,
            api::accounts::disable_account,
            api::accounts::enable_account,
            api::accounts::test_account,
            api::accounts::probe_account,
            api::accounts::inject_account_to_local_ide,
            api::accounts::import_from_local_ide,
            // ==================
            // BROWSER DEVICE-LOGIN (port Go cmd/login)
            // ==================
            api::login::login_start,
            api::login::login_poll,
            api::login::login_cancel,
            api::login::open_login_url,
            // ==================
            // ACCESS KEYS API
            // ==================
            api::access_keys::list_access_keys,
            api::access_keys::create_access_key,
            api::access_keys::revoke_access_key,
            // ==================
            // USAGE API
            // ==================
            api::usage::list_usage_logs,
            api::usage::get_usage_summary,
            // ==================
            // AUDIT LOG API
            // ==================
            api::audit_api::get_traffic_logs,
            api::audit_api::clear_traffic_logs,
            // ==================
            // CONFIG API
            // ==================
            api::config_api::get_config,
            api::config_api::save_config,
            api::config_api::get_default_config,
            // ==================
            // EXTERNAL PROVIDERS (multi-provider: Account + Configuration)
            // ==================
            api::external::get_external_config,
            api::external::save_external_config,
            api::external::get_external_status,
            api::external::refresh_external_models,
            api::external::check_port_available,
            api::external::suggest_free_port,
            api::external::normalize_external_url,
            api::external::get_providers,
            api::external::save_providers,
            api::external::get_provider_status,
            api::external::refresh_provider_models,
            // ==================
            // ZED NATIVE (Account + Configuration, no scheduler)
            // ==================
            api::zed::list_zed_accounts,
            api::zed::import_zed_account,
            api::zed::set_zed_enabled,
            api::zed::remove_zed_account,
            api::zed::test_zed_account,
            api::zed::refresh_zed_models,
            api::zed::get_zed_config,
            api::zed::save_zed_config,
            // ==================
            // SCHEDULER API
            // ==================
            api::scheduler_api::run_keepalive_now,
            api::scheduler_api::get_schedule,
            api::scheduler_api::save_schedule,
            // ==================
            // SETTINGS / LANG / THEME / FONT
            // ==================
            api::settings::get_gui_settings,
            api::settings::save_gui_settings,
            api::lang::get_available_langs,
            api::lang::get_lang_content,
            api::theme::get_available_themes,
            api::font::get_available_fonts,
            // ==================
            // PROFILES & INSTANCES API
            // ==================
            api::profiles::list_profiles,
            api::profiles::create_new_profile,
            api::profiles::remove_profile,
            api::profiles::duplicate_profile,
            api::profiles::bind_account_to_profile,
            api::profiles::launch_profile_instance,
            api::profiles::kill_running_instance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
