/*
[INTEGRITY NOTES]
- Mục đích: Định nghĩa trạng thái toàn cục (App State) cho ứng dụng.
- Trách nhiệm:
  + Lưu danh sách PIDs của các tiến trình rclone ngầm để hỗ trợ việc Hủy (Cancel).
  + Giữ inotify watcher và đường dẫn Local đang được theo dõi cho từng pane.
- Tương tác: Được sử dụng bởi `logic/transfer.rs`, `logic/watcher.rs` và `api/files.rs`.
*/

use notify::RecommendedWatcher;
use std::collections::HashMap;
use std::sync::Mutex;

/// Cấu trúc lưu trữ trạng thái toàn cục của ứng dụng
pub struct AppState {
    // Lưu các tiến trình (PIDs) đang chạy để quản lý hủy tác vụ (kill)
    pub pids: Mutex<HashMap<u32, u32>>,

    /// Trình theo dõi biến động hệ thống file nội bộ (inotify trên Linux).
    /// `None` nếu khởi tạo thất bại — khi đó tính năng tự làm mới sẽ tắt.
    pub local_watcher: Mutex<Option<RecommendedWatcher>>,

    /// Đường dẫn Local đang được theo dõi của từng pane (`"left"` / `"right"`).
    /// Cần tách theo pane vì hai pane có thể mở hai thư mục khác nhau.
    pub watched_paths: Mutex<HashMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pids: Mutex::new(HashMap::new()),
            local_watcher: Mutex::new(None),
            watched_paths: Mutex::new(HashMap::new()),
        }
    }
}
