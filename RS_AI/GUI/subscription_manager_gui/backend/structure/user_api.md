[Pattern Docs]
# user_api.rs

Tài liệu cấu trúc các hàm xử lý dữ liệu và API liên quan đến quản lý người dùng (User).

- **Tên hàm**: `add_user`
- **Mô tả**: Thêm mới một người dùng vào hệ thống.
- **Tham số đầu vào**:
  - `username: String` (Bắt buộc)
  - `email: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<User, String>`

- **Tên hàm**: `update_user`
- **Mô tả**: Cập nhật thông tin của người dùng.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
  - `username: Option<String>` (Tùy chọn)
  - `email: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<User, String>`

- **Tên hàm**: `delete_user`
- **Mô tả**: Xóa một người dùng khỏi hệ thống.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `list_users`
- **Mô tả**: Lấy danh sách toàn bộ người dùng.
- **Tham số đầu vào**:
  - `page: Option<u32>` (Tùy chọn)
  - `limit: Option<u32>` (Tùy chọn)
- **Đầu ra**: `Result<Vec<User>, String>`
