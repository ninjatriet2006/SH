# state.rs
Tài liệu tham chiếu cấu trúc trạng thái chung (State) và các Payload Events trong Bridge.

## Trạng thái toàn cục (AppState)
Được đăng ký qua `.manage()` khi khởi tạo ứng dụng Tauri. Cung cấp bộ nhớ dùng chung thread-safe cho tất cả các handlers.

- **`transfer`**: `Mutex<TransferManager>`
  Quản lý danh sách các tiến trình chuyển file đang đợi (queue) và đang chạy (running), cũng như số luồng đồng thời cho phép.
- **`local_watcher`**: `Mutex<Option<notify::RecommendedWatcher>>`
  Lưu trữ đối tượng thư viện `notify` dùng để theo dõi thay đổi thư mục cục bộ.
- **`watched_path`**: `Mutex<Option<String>>`
  Đường dẫn hiện tại đang được local_watcher bám sát để kịp thời reload UI nếu thư mục đó bị người dùng đổi từ nơi khác.

## Event Payloads
Cấu trúc dữ liệu JSON để giao tiếp giữa Rust và TypeScript (dùng cho hàm emit).

- **`WhoAmIPayload`**: Chứa `email` hoặc `error`. Phát ngay sau khi khởi động app (`auth:whoami-finished`).
- **`TransferProgressPayload`**: Báo cáo tiến trình tải theo thời gian thực (ID, %, bytes). Phát liên tục (`transfer:progress`).
- **`TransferFinishedPayload`**: Báo cáo hoàn tất thao tác tải (ID, thành công hay lỗi). Phát khi kết thúc 1 luồng (`transfer:finished`).
- **`OSClipboardData`**: Cấu trúc trả về cho `os_clipboard_get` chứa mode (cut/copy) và danh sách đường dẫn `paths`.
