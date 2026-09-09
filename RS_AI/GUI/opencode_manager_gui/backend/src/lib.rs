/*
[INTEGRITY NOTES]
- Mục đích: Thư viện chính của OpenCode Manager GUI (backend Tauri).
- Trách nhiệm: Đăng ký toàn bộ Tauri command và khởi tạo app.
- Cấu trúc:
  + `api`: các endpoint giao tiếp với Frontend.
  + `core`: nghiệp vụ hợp nhất cấu hình + dò tài nguyên.
  Logic đọc/ghi file và gọi API provider dùng lại crate `opencode_manager` (TUI)
  để TUI và GUI không thể lệch nhau về định dạng.
*/

pub mod api;
pub mod core;

/// Hạ tầng cho test integration đổi `OPENCODE_TEST_HOME`.
///
/// Env là process-wide: các test ghi file cấu hình theo home phải lấy chung
/// lock này, nếu không `cargo test` chạy song song sẽ cướp home của nhau
/// (test A set dirA, test B set dirB, save của A rơi vào dirB).
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::sync::Mutex;

    pub static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Tạo thư mục home test riêng (theo tag) rồi trỏ `OPENCODE_TEST_HOME` vào.
    pub fn isolate_home(tag: &str) -> PathBuf {
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("test_homes")
            .join(tag);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("OPENCODE_TEST_HOME", &dir);
        dir
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        // WebKitGTK trên một số máy Linux lỗi render khi bật DMABUF/compositing.
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    let builder = tauri::Builder::default();
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
            use tauri::Manager;

            if let Ok(resource_dir) = app.path().resource_dir() {
                // Resources của Tauri installer không luôn nằm cạnh binary.
                // Truyền root thực qua cùng resolver mà bản portable đang dùng.
                std::env::set_var("OPENCODE_MANAGER_RESOURCE_DIR", resource_dir);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ==================
            // PROVIDER API
            // ==================
            api::provider::list_providers,
            api::provider::list_presets,
            api::provider::get_provider_secret,
            api::provider::save_provider,
            api::provider::delete_provider,
            api::provider::delete_providers,
            api::provider::test_provider,
            api::provider::test_connection,
            api::provider::test_all_providers,
            api::provider::scan_provider_models,
            api::provider::set_provider_models,
            api::provider::find_bad_providers,
            // ==================
            // BULK ADD
            // ==================
            api::bulk::bulk_add_providers,
            // ==================
            // CKEY (ckey.vn)
            // ==================
            api::ckey::list_ckey_profiles,
            api::ckey::save_ckey_profile,
            api::ckey::delete_ckey_profile,
            api::ckey::set_active_ckey_profile,
            api::ckey::fetch_ckey_dashboard,
            api::ckey::fetch_ckey_usage,
            api::ckey::fetch_ckey_deposit,
            api::ckey::list_ckey_import_items,
            api::ckey::import_ckey_models,
            // ==================
            // MODELS (ma trận so sánh + model chính)
            // ==================
            api::models::list_model_matrix,
            api::models::set_primary_model,
            api::models::sync_limits_from_dev,
            // ==================
            // ARBITER (trọng tài chấm điểm model)
            // ==================
            api::arbiter::get_arbiter_state,
            api::arbiter::run_arbiter_evaluation,
            api::arbiter::clear_arbiter_history,
            api::arbiter::recommend_models,
            // ==================
            // SETTINGS / LANG / THEME / FONT
            // ==================
            api::settings::get_gui_settings,
            api::settings::save_gui_settings,
            api::settings::get_config_paths,
            api::settings::open_external_url,
            api::lang::get_available_langs,
            api::lang::get_lang_content,
            api::theme::get_available_themes,
            api::font::get_available_fonts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
