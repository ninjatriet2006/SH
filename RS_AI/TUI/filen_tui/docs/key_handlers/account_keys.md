# Account Key Handlers (TUI)

Tài liệu quy định cách tách biệt mã xử lý phím cho ngữ cảnh Tài khoản (Account/Login).
Mã nguồn phải được đặt tại `src/app/key_handlers/account.rs` hoặc phân rã nhỏ hơn.

## 1. Trách nhiệm (Responsibilities)
- Xử lý phím khi người dùng đang ở màn hình Đăng nhập (Auth).
- Xử lý phím khi người dùng đang ở InputBox nhập Email, Password, hoặc 2FA Code.
- Điều hướng lên/xuống giữa các Account đã lưu (Quick Login).

## 2. Các phím cơ bản
- `Enter`: Submit form đăng nhập.
- `Tab` / `Shift+Tab`: Di chuyển tiêu điểm (Focus) giữa ô Email, Password và nút Login.
- `Up` / `Down`: Chọn account trong danh sách tài khoản đã lưu.
- `Esc`: Thoát chế độ nhập liệu (thoát khỏi `tui-textarea`).

## 3. Tiêu chuẩn Phân rã Code (Thực tế)
Hiện tại `account.rs` đang nặng 21KB. Bắt buộc phải chia tách thành các hàm nhỏ hơn dựa trên Input đang focus:
- `handle_email_input(app, key)`
- `handle_password_input(app, key)`
- `handle_twofa_input(app, key)`
- `handle_account_list_nav(app, key)`
Không viết một hàm `handle_account_keys` dài hàng trăm dòng chứa toàn lệnh match lồng nhau.
