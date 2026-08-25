# auth.rs
Tài liệu tham chiếu các hàm làm việc với xác thực tài khoản (Auth).

- **Tên hàm**: `login_new_terminal`
- **Mô tả**: Đăng nhập vào Filen bằng email và mật khẩu (có thể kèm mã 2FA).
- **Tham số đầu vào**:
  - `email: &str` (Bắt buộc)
  - `password: &str` (Bắt buộc)
  - `twofa: Option<&str>` (Tùy chọn)
  - `keep_logged: &str` ("y" hoặc "n") (Bắt buộc)
  - `active_account: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `whoami_terminal`
- **Mô tả**: Trả về email tài khoản đang đăng nhập hiện tại.
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<String, String>`

- **Tên hàm**: `statfs_terminal`
- **Mô tả**: Lấy thông tin dung lượng đã dùng / tổng dung lượng của tài khoản.
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<(String, String), String>`

- **Tên hàm**: `logout_terminal`
- **Mô tả**: Đăng xuất khỏi tài khoản hiện tại.
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<(), String>`
