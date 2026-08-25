pub mod utils;
pub mod models;
pub mod storage;
pub mod user_api;
pub mod package_api;
pub mod subscription_api;
pub mod transaction_api;
pub mod settings_api;
pub mod lang_api;
pub mod theme_api;
pub mod font_api;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  #[cfg(target_os = "linux")]
  {
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
  }

  tauri::Builder::default()
    .setup(|app| {
      utils::init_time_sync();
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      user_api::add_user,
      user_api::update_user,
      user_api::delete_user,
      user_api::list_users,
      package_api::add_package,
      package_api::update_package,
      package_api::delete_package,
      package_api::list_packages,
      subscription_api::add_subscription_to_user,
      subscription_api::update_subscription_expiry,
      subscription_api::remove_subscription_from_user,
      subscription_api::list_user_subscriptions,
      subscription_api::check_subscription_status,
      subscription_api::list_all_subscriptions,
      transaction_api::list_user_transactions,
      transaction_api::list_all_transactions,
      transaction_api::delete_transaction,
      settings_api::get_settings,
      settings_api::save_settings,
      lang_api::get_available_langs,
      lang_api::get_lang_content,
      theme_api::get_available_themes,
      font_api::get_available_fonts,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
