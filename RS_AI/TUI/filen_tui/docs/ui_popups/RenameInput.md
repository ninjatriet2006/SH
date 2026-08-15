# Rename Input Popup (TUI)

Tài liệu quy định Popup nhập tên mới cho File/Folder hiện tại.

## 1. Trách nhiệm
- Hiển thị tên hiện tại của tệp/thư mục được bôi đen sẵn trong `tui-textarea` để người dùng có thể xóa hoặc sửa nhanh.
- Khi người dùng ấn `Enter`, lấy chuỗi mới, ghép vào đường dẫn cha, và gọi lệnh di chuyển (Move/Rename).

## 2. Tiêu chuẩn Phân rã
- Hàm render phải đặt tại `src/ui/popups/rename.rs`.
- Cần có hàm tiện ích (Utility function) để lấy phần tên mở rộng (Extension) và bôi đen (select) phần tên gốc, không bôi đen phần đuôi mở rộng khi vừa bật Popup.
- Kích thước tương tự Mkdir Popup.
