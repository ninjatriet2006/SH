# Popup: Khung Đăng Nhập (LoginInput & TwoFactorInput)

## 1. Trách nhiệm (Responsibility)
Cung cấp biểu mẫu (Form) nhập liệu cho quá trình xác thực tài khoản Filen. Do TUI không có native text box, ứng dụng phải tự xây dựng bộ đệm phím (Key buffer) và quản lý con trỏ.

## 2. Các trạng thái
- **`PopupState::LoginInput`**:
  - `email_buffer`: `String` - Lưu trữ chuỗi ký tự Email.
  - `pass_buffer`: `String` - Lưu trữ chuỗi ký tự Mật khẩu.
  - `active_field`: `usize` - Chỉ báo trường nào đang được focus (0: Email, 1: Password).
  - `keep_logged`: `bool` - Đánh dấu duy trì đăng nhập.
  - `error_msg`: `Option<String>` - Hiển thị lỗi nếu thông tin sai.

- **`PopupState::TwoFactorInput`**:
  - Kích hoạt khi CLI báo lỗi yêu cầu mã 2FA.
  - `code_buffer`: `String` - Lưu trữ chuỗi mã 6 chữ số.
  - `email` / `password`: `String` - Mang theo dữ liệu từ bước 1 để gửi lại cho tiến trình.

## 3. Luồng sự kiện bàn phím (`app/key_handlers/account.rs`)
Khi ứng dụng đang hiển thị Form đăng nhập, luồng sự kiện được chuyển hướng sang một khối match riêng:
- Phím `Tab` / `Shift+Tab`: Xoay vòng giá trị `active_field`.
- Ký tự thông thường: Đẩy (push) vào buffer hiện tại.
- `Backspace`: Xóa ký tự cuối (pop) khỏi buffer.
- `Enter`: Gửi dữ liệu. Gọi `Operations::login_new()`. Nếu thành công, xóa form. Nếu thất bại, gán chuỗi lỗi vào `error_msg`.

## 4. Định hướng Refactor
- Đây là một trong những khối logic phức tạp nhất vì phải quản lý trạng thái form thủ công.
- Nên thiết lập một kiến trúc `tui-textarea` hoặc một Custom Form Component có thể tái sử dụng để không phải tự code luồng đẩy chuỗi bằng phím (`push`/`pop`) cho mọi màn hình nhập liệu.
