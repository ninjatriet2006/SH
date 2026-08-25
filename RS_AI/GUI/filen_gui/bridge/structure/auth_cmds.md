# auth_cmds.rs
Tài liệu tham chiếu các Tauri commands liên quan đến xác thực tài khoản (Auth) trong Bridge.

- **Tên hàm**: `auth_login_terminal`
- **Mô tả**: Đăng nhập vào Filen bằng email và mật khẩu (trả về lỗi "2FA_REQUIRED" nếu cần).
- **Tham số đầu vào**:
  - `email: String` (Bắt buộc)
  - `password: String` (Bắt buộc)
  - `twofa_code: Option<String>` (Tùy chọn)
  - `keep_logged: bool` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `auth_login_twofa_terminal`
- **Mô tả**: Xử lý bước 2 cho tài khoản yêu cầu mã 2FA.
- **Tham số đầu vào**:
  - `email: String` (Bắt buộc)
  - `password: String` (Bắt buộc)
  - `twofa_code: String` (Bắt buộc)
  - `keep_logged: bool` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `auth_logout_terminal`
- **Mô tả**: Đăng xuất khỏi tài khoản chỉ định.
- **Tham số đầu vào**:
  - `account: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `auth_whoami_terminal`
- **Mô tả**: Trả về email account đang kích hoạt.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Result<Option<String>, String>`

- **Tên hàm**: `auth_statfs_terminal`
- **Mô tả**: Lấy thông tin dung lượng sử dụng của tài khoản.
- **Tham số đầu vào**:
  - `account: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<(String, String), String>`

- **Tên hàm**: `accounts_load`
- **Mô tả**: Nạp danh sách tài khoản đã lưu để đăng nhập nhanh.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Vec<StoredAccount>`

- **Tên hàm**: `accounts_save`
- **Mô tả**: Lưu lại danh sách tài khoản đã đăng nhập.
- **Tham số đầu vào**:
  - `accounts: Vec<StoredAccount>` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`
