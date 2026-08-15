# Status Bar Component (TUI)

Thanh trạng thái nằm ở dưới cùng của giao diện.

## 1. Trách nhiệm (Responsibilities)
- Hiển thị tài khoản đang đăng nhập (hoặc Local).
- Hiển thị dung lượng lưu trữ (Free / Total) bên góc phải.
- Hiển thị đường dẫn đầy đủ của thư mục đang mở.
- Hiển thị số lượng tệp đang được chọn (Selected items).
- Hiển thị thông báo (Notification/Toast) nếu có (vd: "Tạo thư mục thành công").

## 2. Tiêu chuẩn Phân rã Code
- Bắt buộc tạo `src/ui/components/status_bar.rs`.
- Thanh Status Bar nên dùng `ratatui::widgets::Paragraph` hoặc kết hợp `Layout::horizontal` để chia các vùng thông tin Trái/Phải rõ ràng.
