[Pattern Docs]
# models.rs

Tài liệu cấu trúc các đối tượng dữ liệu cốt lõi cho hệ thống quản lý đăng ký dịch vụ (Subscription Manager).

- **Tên hàm**: `User` (Struct)
- **Mô tả**: Đại diện cho thông tin của một người dùng trong hệ thống.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
  - `username: String` (Bắt buộc)
  - `email: String` (Tùy chọn)
  - `created_at: i64` (Bắt buộc)
- **Đầu ra**: `N/A`

- **Tên hàm**: `Package` (Struct)
- **Mô tả**: Đại diện cho một gói dịch vụ mà nhà cung cấp phân phối.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
  - `name: String` (Bắt buộc)
  - `description: String` (Tùy chọn)
  - `duration_days: u32` (Bắt buộc)
- **Đầu ra**: `N/A`

- **Tên hàm**: `Subscription` (Struct)
- **Mô tả**: Đại diện cho gói đăng ký dịch vụ của người dùng với thời gian hết hạn được quyết định.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
  - `user_id: String` (Bắt buộc)
  - `package_id: String` (Bắt buộc)
  - `expiration_date: i64` (Bắt buộc)
  - `is_active: bool` (Bắt buộc)
- **Đầu ra**: `N/A`
