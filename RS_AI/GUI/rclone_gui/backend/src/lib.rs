/*
[INTEGRITY NOTES]
- Mục đích: Thư viện chính (lib) của ứng dụng Tauri (Backend rclone_gui).
- Trách nhiệm:
  + Khởi tạo AppState và định tuyến toàn bộ API endpoint (Tauri Commands).
- Cấu trúc 3 tầng:
  + `api`: Các API endpoints giao tiếp với Frontend.
  + `logic`: Xử lý nghiệp vụ phức tạp.
  + `core`: Giao tiếp hệ điều hành, rclone thô.
*/

pub mod actions;
pub mod api;
pub mod core;
pub mod logic;
pub mod settings;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .manage(logic::app_state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            // ==================
            // FILES API
            // ==================
            api::files_view::list_files,
            api::files_edit::fs_mkdir,
            api::files_edit::fs_touch,
            api::files_edit::fs_delete,
            api::files_edit::fs_rename,
            // UNIVERSAL: fs_copy/fs_move/fs_cancel cũ đã gộp về jobs — xóa command cũ.
            api::files_view::fs_stat_advanced,
            api::files_view::fs_search,
            api::files_view::fs_check_conflicts,
            api::files_view::get_home_dir,
            api::files_view::get_user_places,
            api::files_view::open_in_terminal,
            api::files_view::fs_get_thumbnail,
            api::files_view::fs_temp_dir,
            api::files_edit::fs_chmod,
            api::files_edit::fs_chown,
            api::files_edit::get_permission_policy,
            api::files_edit::set_permission_policy,
            // ==================
            // SYS API (S2 tách vai: actions::view + logic::clipboard/custom_action)
            // ==================
            api::sys::sys_open_with,
            api::sys::sys_list_apps,
            api::sys::os_clipboard_set,
            api::sys::os_clipboard_get,
            api::sys::sys_get_custom_actions,
            api::sys::sys_get_valid_actions,
            api::sys::sys_execute_custom_action,
            // ==================
            // TRASH API
            // ==================
            api::trash_manager::fs_trash_list_local,
            api::trash_manager::fs_trash_restore_local,
            api::trash_manager::fs_trash_delete_local,
            api::trash_manager::fs_trash_empty_local,
            api::trash_manager::fs_trash_list_remote_terminal,
            api::trash_manager::fs_trash_restore_remote_terminal,
            api::trash_manager::fs_trash_delete_remote_terminal,
            api::trash_manager::fs_trash_empty_remote_terminal,
            // ==================
            // REMOTES API
            // ==================
            api::remote_manager::list_remotes,
            api::remote_manager::get_providers,
            api::remote_manager::create_remote,
            api::remote_manager::update_remote,
            api::remote_manager::delete_remote,
            api::remote_manager::get_backend_features,
            api::remote_manager::check_transfer_capability,
            api::remote_manager::rclone_about,
            api::remote_manager::rclone_size,
            // ==================
            // MOUNT API
            // ==================
            api::mount_manager::check_fuse_installed,
            api::mount_manager::create_mount_service,
            api::mount_manager::delete_mount_service,
            api::mount_manager::manage_mount_service,
            api::mount_manager::list_mount_services,
            api::mount_manager::get_mount_service_config,
            // ==================
            // CONFIG API
            // ==================
            api::config::get_config_content,
            api::config::set_config_content,
            api::config::reorder_config,
            api::config::list_config_snapshots,
            api::config::restore_config_snapshot,
            api::config::export_config_remote,
            api::config::import_config_remote,
            api::config::get_engine_flags,
            api::config::set_engine_flags,
            // ==================
            // LANG API
            // ==================
            api::langs_loader::get_available_langs,
            api::langs_loader::get_lang_content,
            api::themes_loader::get_available_themes,
            api::fonts_loader::get_available_fonts,
            // ==================
            // JOBS API (đường duy nhất cho copy/move/delete/list)
            // ==================
            api::jobs::job_enqueue,
            api::jobs::job_list,
            api::jobs::job_cancel,
        ])
        .setup(|app| {
            if let Ok(path) = app.path().resource_dir() {
                core::resources::init_resource_base(path);
            }
            // Inotify watcher cho thư mục Local đang xem — phát `local-dir-changed`
            // để Frontend tự nạp lại khi file đổi ngoài ứng dụng.
            logic::watcher::init(app.handle());
            // UNIVERSAL: dọn job mồ côi (phiên trước bị giết ngang) 1 lần lúc
            // khởi động — job Running → Error + xóa `*.partial` ở đích Local.
            let state = app.state::<logic::app_state::AppState>();
            state.jobs.reconcile_orphans();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Lỗi nghiêm trọng khi khởi chạy ứng dụng Tauri rcloneGUI");
}
