[Pattern Docs]
# package_api.rs

Tài liệu cấu trúc các hàm xử lý dữ liệu và API liên quan đến quản lý gói dịch vụ (Package).

- **Tên hàm**: `add_package`
- **Mô tả**: Tạo mới một gói dịch vụ để cung cấp cho người dùng.
- **Tham số đầu vào**:
  - `name: String` (Bắt buộc)
  - `duration_days: u32` (Bắt buộc)
  - `description: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<Package, String>`

- **Tên hàm**: `update_package`
- **Mô tả**: Chỉnh sửa thông tin của một gói dịch vụ đã có.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
  - `name: Option<String>` (Tùy chọn)
  - `duration_days: Option<u32>` (Tùy chọn)
  - `description: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<Package, String>`

- **Tên hàm**: `delete_package`
- **Mô tả**: Xóa một gói dịch vụ khỏi hệ thống.
- **Tham số đầu vào**:
  - `id: String` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `list_packages`
- **Mô tả**: Lấy danh sách toàn bộ các gói dịch vụ.
- **Tham số đầu vào**:
  - `page: Option<u32>` (Tùy chọn)
  - `limit: Option<u32>` (Tùy chọn)
- **Đầu ra**: `Result<Vec<Package>, String>`
