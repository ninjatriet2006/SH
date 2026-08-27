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

pub mod api;
pub mod core;
pub mod logic;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(logic::app_state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            // ==================
            // FILES API
            // ==================
            api::files::list_files,
            api::files::fs_mkdir,
            api::files::fs_touch,
            api::files::fs_delete,
            api::files::fs_rename,
            api::files::fs_copy,
            api::files::fs_move,
            api::files::fs_cancel,
            api::files::fs_stat_advanced,
            api::files::fs_search,
            api::files::get_home_dir,
            api::files::open_in_terminal,
            api::files::fs_get_thumbnail,
            
            // ==================
            // SYS API (Trong core/sys.rs)
            // ==================
            core::sys::sys_open_with,
            core::sys::sys_list_apps,
            core::sys::os_clipboard_set,
            core::sys::os_clipboard_get,
            core::sys::sys_get_custom_actions,
            core::sys::sys_get_valid_actions,
            core::sys::sys_execute_custom_action,
            
            // ==================
            // TRASH API
            // ==================
            api::trash::fs_trash_list_local,
            api::trash::fs_trash_restore_local,
            api::trash::fs_trash_empty_local,
            api::trash::fs_trash_list_remote_terminal,
            api::trash::fs_trash_restore_remote_terminal,
            api::trash::fs_trash_delete_remote_terminal,
            api::trash::fs_trash_empty_remote_terminal,
            
            // ==================
            // AUTH API
            // ==================
            api::auth::auth_login_terminal,
            api::auth::auth_login_twofa_terminal,
            api::auth::auth_statfs_terminal,
            
            // ==================
            // REMOTES API
            // ==================
            api::remotes::get_providers,
            api::remotes::create_remote,
            api::remotes::update_remote,
            api::remotes::delete_remote,
            api::remotes::get_backend_features,
            api::remotes::check_transfer_capability,
            
            // ==================
            // MOUNT API
            // ==================
            api::mount::check_fuse_installed,
            api::mount::create_mount_service,
            api::mount::delete_mount_service,
            api::mount::manage_mount_service,
            api::mount::list_mount_services,
            api::mount::get_mount_service_config,
            
            // ==================
            // CONFIG API
            // ==================
            api::config::get_config_content,
            api::config::set_config_content,
            api::config::reorder_config,
        ])
        .run(tauri::generate_context!())
        .expect("Lỗi nghiêm trọng khi khởi chạy ứng dụng Tauri rcloneGUI");
}
