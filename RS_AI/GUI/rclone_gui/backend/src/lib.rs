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
pub mod ipc;
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
            ipc::list_files,
            ipc::fs_mkdir,
            ipc::fs_touch,
            ipc::fs_delete,
            ipc::fs_rename,
            ipc::fs_copy,
            ipc::fs_move,
            ipc::fs_cancel,
            ipc::fs_stat_advanced,
            ipc::fs_search,
            ipc::fs_check_conflicts,
            ipc::get_home_dir,
            ipc::get_user_places,
            ipc::open_in_terminal,
            ipc::fs_get_thumbnail,
            ipc::fs_temp_dir,
            ipc::fs_chmod,
            ipc::fs_chown,
            ipc::get_permission_policy,
            ipc::set_permission_policy,
            // ==================
            // SYS API (Trong core/sys.rs)
            // ==================
            ipc::sys_open_with,
            ipc::sys_list_apps,
            ipc::os_clipboard_set,
            ipc::os_clipboard_get,
            ipc::sys_get_custom_actions,
            ipc::sys_get_valid_actions,
            ipc::sys_execute_custom_action,
            // ==================
            // TRASH API
            // ==================
            ipc::fs_trash_list_local,
            ipc::fs_trash_restore_local,
            ipc::fs_trash_delete_local,
            ipc::fs_trash_empty_local,
            ipc::fs_trash_list_remote_terminal,
            ipc::fs_trash_restore_remote_terminal,
            ipc::fs_trash_delete_remote_terminal,
            ipc::fs_trash_empty_remote_terminal,
            // ==================
            // REMOTES API
            // ==================
            ipc::list_remotes,
            ipc::get_providers,
            ipc::create_remote,
            ipc::update_remote,
            ipc::delete_remote,
            ipc::get_backend_features,
            ipc::check_transfer_capability,
            ipc::rclone_about,
            ipc::rclone_size,
            // ==================
            // MOUNT API
            // ==================
            ipc::check_fuse_installed,
            ipc::create_mount_service,
            ipc::delete_mount_service,
            ipc::manage_mount_service,
            ipc::list_mount_services,
            ipc::get_mount_service_config,
            // ==================
            // CONFIG API
            // ==================
            ipc::get_config_content,
            ipc::set_config_content,
            ipc::reorder_config,
            ipc::list_config_snapshots,
            ipc::restore_config_snapshot,
            ipc::export_config_remote,
            ipc::import_config_remote,
            ipc::get_engine_flags,
            ipc::set_engine_flags,
            // ==================
            // LANG API
            // ==================
            ipc::get_available_langs,
            ipc::get_lang_content,
            ipc::get_available_themes,
            ipc::get_available_fonts,
        ])
        .setup(|app| {
            if let Ok(path) = app.path().resource_dir() {
                core::resources::init_resource_base(path);
            }
            // Inotify watcher cho thư mục Local đang xem — phát `local-dir-changed`
            // để Frontend tự nạp lại khi file đổi ngoài ứng dụng.
            logic::watcher::init(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Lỗi nghiêm trọng khi khởi chạy ứng dụng Tauri rcloneGUI");
}
