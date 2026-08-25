//! [INTEGRITY NOTES]
//! Mục đích: App Shell v2 cho filen_gui sử dụng Tauri.
//! Trách nhiệm: Liên kết các module (auth, fs, transfer, sys) với các Tauri commands/events.
//! Khởi tạo builder và chạy ứng dụng.
//! Tương tác: Điểm truy cập chính, gọi `filen_gui_tauri::run()` từ `bridge/src/main.rs`.

pub mod state;
pub mod auth_cmds;
pub mod fs_cmds;
pub mod transfer_cmds;
pub mod sys_cmds;

use state::{AppState, WhoAmIPayload};
use tauri::{Emitter, Manager};
use notify::{Watcher, EventKind};

/// Điểm khởi chạy chính của ứng dụng Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Kích hoạt plugin hỗ trợ kéo thả
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_shell::init())
        // Nạp trạng thái toàn cục (AppState) vào bộ nhớ quản lý của Tauri
        .manage(AppState::default())
        // Đăng ký toàn bộ các command để Frontend có thể gọi thông qua `invoke()`
        .invoke_handler(tauri::generate_handler![
            // ----------------- Xác thực (Auth) -----------------
            auth_cmds::auth_login_terminal,
            auth_cmds::auth_login_twofa_terminal,
            auth_cmds::auth_logout_terminal,
            auth_cmds::auth_whoami_terminal,
            auth_cmds::auth_statfs_terminal,
            auth_cmds::accounts_load,
            auth_cmds::accounts_save,
            
            // ----------------- File System -----------------
            fs_cmds::fs_list_remote_terminal,
            fs_cmds::fs_list_remote_stream_terminal,
            fs_cmds::fs_list_local,
            fs_cmds::fs_get_thumbnail,
            fs_cmds::fs_mkdir_terminal,
            fs_cmds::fs_rm_terminal,
            fs_cmds::fs_mv_terminal,
            fs_cmds::fs_cp_terminal,
            fs_cmds::fs_cp_local,
            fs_cmds::fs_mv_local,
            fs_cmds::fs_rm_local,
            fs_cmds::fs_mkdir_local,
            fs_cmds::fs_rename_local,
            fs_cmds::fs_cp_batch,
            fs_cmds::fs_upload_terminal,
            fs_cmds::fs_download_terminal,
            fs_cmds::fs_cat_terminal,
            fs_cmds::fs_link_create_terminal,
            fs_cmds::fs_links_list_terminal,
            fs_cmds::fs_write_terminal,
            fs_cmds::fs_write_local,
            fs_cmds::fs_sudo_exec,
            
            // --------- Alias thao tác File System ---------
            fs_cmds::fs_rename_terminal,
            fs_cmds::fs_delete_terminal,
            fs_cmds::fs_copy_terminal,
            fs_cmds::fs_move_terminal,
            fs_cmds::fs_open,
            fs_cmds::fs_stat_advanced,
            fs_cmds::fs_chmod,
            fs_cmds::fs_chown,
            fs_cmds::fs_get_free_space,
            fs_cmds::fs_search_local,
            
            // ----------------- Thùng rác (Trash) -----------------
            fs_cmds::fs_trash_list_local,
            fs_cmds::fs_trash_restore_local,
            fs_cmds::fs_trash_empty_local,
            fs_cmds::fs_trash_list_remote_terminal,
            fs_cmds::fs_trash_restore_remote_terminal,
            fs_cmds::fs_trash_delete_remote_terminal,
            fs_cmds::fs_trash_empty_remote_terminal,
            
            // ----------------- Truyền tải (Transfer) -----------------
            transfer_cmds::transfer_enqueue,
            transfer_cmds::transfer_start,
            transfer_cmds::transfer_cancel,
            transfer_cmds::transfer_cancel_all,
            transfer_cmds::transfer_remove_finished,
            
            // ----------------- Lệnh hệ thống (System) -----------------
            sys_cmds::os_clipboard_set,
            sys_cmds::os_clipboard_get,
            sys_cmds::sys_list_apps,
            sys_cmds::sys_get_custom_actions,
            sys_cmds::sys_execute_custom_action,
            sys_cmds::sys_open_with,
            sys_cmds::open_in_terminal
        ])
        .setup(|app| {
            // Cài đặt Inotify Watcher để theo dõi biến động thư mục nội bộ (Local Pane)
            let (tx, rx) = std::sync::mpsc::channel();
            let app_handle_for_watch = app.handle().clone();
            
            // Luồng phụ nhận sự kiện thay đổi file và báo lên UI
            std::thread::spawn(move || {
                for res in rx {
                    match res {
                        Ok(event) => {
                            let event: notify::Event = event;
                            // Chỉ phát tín hiệu (emit) cho các sự kiện tạo, sửa, xóa để tránh nhiễu
                            match event.kind {
                                EventKind::Access(_) => continue, // Bỏ qua sự kiện chỉ đọc/mở file
                                _ => {
                                    // Báo cho UI biết có thư mục thay đổi để reload lại Local Pane
                                    let _ = app_handle_for_watch.emit("local-dir-changed", ());
                                }
                            }
                        },
                        Err(e) => println!("Lỗi watcher: {:?}", e),
                    }
                }
            });

            // Khởi tạo watcher và lưu vào AppState
            if let Ok(watcher) = notify::RecommendedWatcher::new(
                move |res| {
                    let _ = tx.send(res);
                },
                notify::Config::default(),
            ) {
                let state = app.state::<AppState>();
                *state.local_watcher.lock().unwrap() = Some(watcher);
            }

            // Khởi chạy tiến trình kiểm tra phiên đăng nhập ngay khi khởi động.
            // Frontend lắng nghe sự kiện `auth:whoami-finished` để quyết định xem có tải Cloud Pane hay không.
            let app_clone = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result = filen_gui::auth::whoami_terminal(&None).await;
                let (email, error) = match result {
                    Ok(email) => {
                        let email_clean = email.trim().to_string();
                        // Trích xuất email hợp lệ
                        if !email_clean.is_empty()
                            && !email_clean.contains("Please enter")
                            && !email_clean.contains("credentials")
                            && email_clean != "anonymous@filen.io"
                        {
                            (Some(email_clean), None)
                        } else {
                            (None, None)
                        }
                    }
                    Err(err) => (None, Some(err)),
                };
                // Gửi kết quả kiểm tra phiên về Frontend
                let _ = app_clone.emit("auth:whoami-finished", WhoAmIPayload { email, error });
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Có lỗi xảy ra trong quá trình chạy ứng dụng Tauri");
}
