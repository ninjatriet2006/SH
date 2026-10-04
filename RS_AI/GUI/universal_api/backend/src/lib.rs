/*
[INTEGRITY NOTES]
- Purpose: Main library for Universal API (Tauri backend).
- Responsibility: Register all Tauri commands and initialize the app.
- Structure:
  + `api`: Tauri command endpoints communicating with Frontend.
  + `core`: Domain logic (CodeBuddy gateway + anti-api bridge).
*/

pub mod actions;
pub mod core;
pub mod ipc;
pub mod providers;

pub use providers as provider;

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

            // Khởi động tiến trình Token Keeper quét ngầm tự động làm mới token
            core::token_keeper::start_token_keeper_daemon();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ==================
            // ACTIONS: SYSTEM TOOLS (COCKPIT PARITY)
            // ==================
            actions::system_tools::trigger_token_keeper,
            actions::system_tools::get_auto_checkin_status,
            actions::system_tools::toggle_auto_checkin,
            actions::system_tools::run_auto_checkin_now,
            actions::system_tools::scan_instance_storage,
            actions::system_tools::execute_instance_storage_clean,
            actions::system_tools::get_webdav_config,
            actions::system_tools::save_webdav_config,
            actions::system_tools::test_webdav_connection_cmd,
            actions::system_tools::backup_to_webdav_now,
            actions::system_tools::list_webdav_backups_cmd,
            actions::system_tools::restore_from_webdav_cmd,
            actions::system_tools::execute_wakeup_tasks,
            actions::system_tools::single_account_wakeup,
            actions::system_tools::sync_platform_sessions,
            actions::system_tools::clean_platform_sessions,
            // ==================
            // ACTIONS: GATEWAY
            // ==================
            actions::gateway::get_gateway_status,
            actions::gateway::start_gateway,
            actions::gateway::stop_gateway,
            actions::gateway::get_debug_traces,
            actions::gateway::get_serving_account,
            actions::gateway::refresh_gateway_models,
            // ==================
            // ACTIONS: ACCESS KEYS
            // ==================
            actions::access_keys::list_access_keys,
            actions::access_keys::create_access_key,
            actions::access_keys::revoke_access_key,
            // ==================
            // ACTIONS: USAGE & AUDIT LOGS
            // ==================
            actions::usage::list_usage_logs,
            actions::usage::get_usage_summary,
            actions::audit::get_traffic_logs,
            actions::audit::clear_traffic_logs,
            // ==================
            // ACTIONS: CONFIG & SCHEDULER
            // ==================
            actions::config::get_config,
            actions::config::save_config,
            actions::config::get_default_config,
            actions::scheduler::run_keepalive_now,
            actions::scheduler::get_schedule,
            actions::scheduler::save_schedule,
            // ==================
            // ACTIONS: SETTINGS / APPEARANCE / LANG / THEME / FONT
            // ==================
            actions::settings::get_gui_settings,
            actions::settings::save_gui_settings,
            actions::ide_detector::auto_detect_ide_path,
            actions::settings::open_data_folder,
            actions::lang::get_available_langs,
            actions::lang::get_lang_content,
            actions::theme::get_available_themes,
            actions::font::get_available_fonts,
            // ==================
            // ACTIONS: PROFILES & INSTANCES
            // ==================
            actions::profiles::list_profiles,
            actions::profiles::create_new_profile,
            actions::profiles::remove_profile,
            actions::profiles::duplicate_profile,
            actions::profiles::bind_account_to_profile,
            actions::profiles::launch_profile_instance,
            actions::profiles::kill_running_instance,
            actions::profiles::list_platform_instances,
            actions::profiles::create_platform_instance,
            actions::profiles::update_platform_instance,
            actions::profiles::delete_platform_instance,
            actions::profiles::launch_platform_instance,
            actions::profiles::stop_platform_instance,
            actions::profiles::open_instance_folder,
            // ==================
            // ACTIONS: EXTERNAL PROVIDERS
            // ==================
            actions::external::get_external_config,
            actions::external::save_external_config,
            actions::external::get_external_status,
            actions::external::refresh_external_models,
            actions::external::check_port_available,
            actions::external::suggest_free_port,
            actions::external::normalize_external_url,
            actions::external::get_providers,
            actions::external::save_providers,
            actions::external::get_provider_status,
            actions::external::refresh_provider_models,
            // ==================
            // PROVIDERS: GENERAL ACCOUNTS & AGGREGATOR
            // ==================
            providers::list_accounts,
            providers::get_account_status,
            providers::add_account,
            providers::remove_account,
            providers::disable_account,
            providers::enable_account,
            providers::test_account,
            providers::antigravity::quota::refresh_antigravity_quota,
            providers::probe_account,
            providers::inject_account_to_local_ide,
            providers::import_from_local_ide,
            providers::get_antigravity_overview,
            providers::get_providers_overview,
            providers::get_antigravity_installed_version_info,
            providers::get_installed_app_version_info,
            providers::get_provider_current_account_id,
            providers::load_account_groups,
            providers::save_account_groups,
            providers::load_platform_account_groups,
            providers::save_platform_account_groups,
            // ==================
            // PROVIDERS: ANTIGRAVITY
            // ==================
            providers::antigravity::switch::load_antigravity_switch_history,
            providers::antigravity::switch::clear_antigravity_switch_history,
            providers::antigravity::accounts::update_account_tags,
            providers::antigravity::accounts::update_account_notes,
            providers::antigravity::accounts::update_account_note,
            providers::antigravity::accounts::reorder_accounts,
            providers::antigravity::accounts::export_accounts,
            providers::antigravity::accounts::import_from_json,
            // ==================
            // ACTIONS: LOGIN & OAUTH (Browser device-login & OAuth)
            // ==================
            actions::login::login_start,
            actions::login::login_poll,
            actions::login::login_cancel,
            actions::login::open_login_url,
            // ==================
            // PROVIDERS: ZED (Native account & config)
            // ==================
            providers::zed::accounts::list_zed_accounts,
            providers::zed::accounts::import_zed_account,
            providers::zed::accounts::set_zed_enabled,
            providers::zed::accounts::remove_zed_account,
            providers::zed::accounts::test_zed_account,
            providers::zed::config::refresh_zed_models,
            providers::zed::config::get_zed_config,
            providers::zed::config::save_zed_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
