# File Viewer Popup (TUI)

Tài liệu quy định Popup xem nội dung văn bản (Text/Markdown/Code) của tệp.

## 1. Trách nhiệm
- Hiển thị nội dung dạng text của tệp đang được chọn (chỉ hoạt động với các tệp nhẹ < 1MB).
- Cho phép người dùng cuộn lên/xuống (Up/Down/PgUp/PgDn).
- Có thể kết hợp `syntect` để làm nổi bật cú pháp (Syntax Highlighting) nếu có thể.

## 2. Tiêu chuẩn Phân rã
- Hàm render phải đặt tại `src/ui/popups/viewer.rs`.
- Cần có hàm tiện ích để cắt gọt văn bản lớn (Pagination/Viewport) để tránh TUI bị treo khi cố render 1 file quá lớn.
- Bố cục: Chiếm 80% chiều rộng và 80% chiều cao màn hình. Hiển thị Title bar chứa tên tệp.
- Ký tự thoát: Ấn `Esc` hoặc `q` để đóng.
