/*
[INTEGRITY NOTES]
- Mục đích: Định nghĩa trạng thái toàn cục (App State) cho ứng dụng.
- Trách nhiệm: Lưu trữ danh sách PIDs của các tiến trình rclone ngầm để hỗ trợ việc Hủy (Cancel).
- Tương tác: Được sử dụng bởi `logic/transfer.rs` và `api/files.rs`.
*/

use std::collections::HashMap;
use std::sync::Mutex;

/// Cấu trúc lưu trữ trạng thái toàn cục của ứng dụng
pub struct AppState {
    // Lưu các tiến trình (PIDs) đang chạy để quản lý hủy tác vụ (kill)
    pub pids: Mutex<HashMap<u32, u32>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pids: Mutex::new(HashMap::new()),
        }
    }
}
