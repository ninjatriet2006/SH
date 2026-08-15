# Mkdir Input Popup (TUI)

Tài liệu quy định Popup nhập tên thư mục mới.

## 1. Trách nhiệm
- Hiển thị hộp thoại nổi nhập liệu (1 dòng) sử dụng `tui-textarea`.
- Hiển thị đường dẫn gốc (`/current/path/`) để người dùng biết họ đang tạo thư mục ở đâu.
- Khi người dùng ấn `Enter`, lấy chuỗi (string) và gọi lệnh tạo thư mục.
- Ngăn người dùng nhập các ký tự đặc biệt không được phép (`/`, `\`, `:`).

## 2. Tiêu chuẩn Phân rã
- Hàm render phải đặt tại `src/ui/popups/mkdir.rs`.
- Logic xử lý phím đặt tại `src/app/key_handlers/popups.rs` (như đã quy định ở `popup_keys.md`).
- Kích thước Popup nên nhỏ gọn (chiều rộng khoảng 50-60 characters, chiều cao 3 dòng).
