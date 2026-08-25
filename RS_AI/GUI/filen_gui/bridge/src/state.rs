//! [INTEGRITY NOTES]
//! Mục đích: Quản lý AppState và định nghĩa các payload cho Event.
//! Trách nhiệm: Lưu trữ trạng thái dùng chung (TransferManager, Watchers), làm data struct cho IPC.
//! Tương tác: Được inject vào các Tauri command và Event emitter qua `tauri::State`.

use std::sync::Mutex;
use filen_gui::transfer::TransferManager;
use notify::RecommendedWatcher;
use serde::Serialize;

/// Trạng thái toàn cục của ứng dụng được chia sẻ giữa các lệnh (commands) của Tauri.
pub struct AppState {
    /// Quản lý tiến trình truyền tải file (upload, download, copy, move).
    pub transfer: Mutex<TransferManager>,
    /// Đối tượng theo dõi hệ thống file nội bộ (inotify watcher) để cập nhật UI tự động.
    pub local_watcher: Mutex<Option<RecommendedWatcher>>,
    /// Đường dẫn thư mục nội bộ đang được theo dõi hiện tại.
    pub watched_path: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            // Khởi tạo trình quản lý truyền tải với cấu hình mặc định
            transfer: Mutex::new(TransferManager::new()),
            // Ban đầu chưa khởi tạo trình theo dõi
            local_watcher: Mutex::new(None),
            // Ban đầu chưa theo dõi thư mục nào
            watched_path: Mutex::new(None),
        }
    }
}

/// Dữ liệu gửi lên Frontend khi sự kiện kiểm tra tài khoản (whoami) hoàn tất.
#[derive(Clone, Serialize)]
pub struct WhoAmIPayload {
    /// Email của tài khoản đang đăng nhập, hoặc None nếu chưa đăng nhập.
    pub email: Option<String>,
    /// Lỗi trả về (nếu có) trong quá trình kiểm tra.
    pub error: Option<String>,
}

/// Dữ liệu tiến độ truyền tải file gửi lên Frontend (phát liên tục).
#[derive(Clone, Serialize)]
pub struct TransferProgressPayload {
    /// ID của tác vụ truyền tải.
    pub id: usize,
    /// Phần trăm tiến độ (từ 0.0 đến 1.0), None nếu chưa rõ.
    pub progress: Option<f32>,
    /// Số byte đã xử lý.
    pub bytes_done: u64,
    /// Tổng số byte của tác vụ.
    pub total_bytes: u64,
}

/// Dữ liệu gửi lên Frontend khi tác vụ truyền tải hoàn tất.
#[derive(Clone, Serialize)]
pub struct TransferFinishedPayload {
    /// ID của tác vụ truyền tải vừa hoàn tất.
    pub id: usize,
    /// Cờ đánh dấu thành công (true) hay thất bại (false).
    pub ok: bool,
    /// Lỗi chi tiết (nếu thất bại).
    pub error: Option<String>,
}

/// Dữ liệu bộ nhớ tạm (clipboard) để sao chép/cắt dán file từ hệ điều hành.
#[derive(Serialize, Debug)]
pub struct OSClipboardData {
    /// Chế độ sao chép ("copy" hoặc "cut").
    pub mode: String,
    /// Danh sách các đường dẫn file.
    pub paths: Vec<String>,
}
