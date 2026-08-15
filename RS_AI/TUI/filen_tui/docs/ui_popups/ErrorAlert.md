# Error Alert Popup (TUI)

Tài liệu quy định cách hiển thị thông báo lỗi (Exception/Error) lên màn hình TUI mà không làm crash ứng dụng.

## 1. Trách nhiệm
- Bắt (Catch) các lỗi trả về từ API (ví dụ: Không đủ quyền, Token hết hạn, Thư mục không tồn tại).
- Hiển thị thông báo lỗi màu đỏ rực (kết hợp với style `ratatui::style::Color::Red`).
- Đợi người dùng nhấn `Enter` hoặc `Esc` để đóng bảng lỗi.

## 2. Tiêu chuẩn Phân rã
- Hàm render phải đặt tại `src/ui/popups/error.rs`.
- `AppState` phải duy trì một trường `pub last_error: Option<String>`.
- Nếu có lỗi, tự động chuyển `active_popup = Popup::Error`.
- Khi đóng Popup, `last_error` phải được gán về `None`.
