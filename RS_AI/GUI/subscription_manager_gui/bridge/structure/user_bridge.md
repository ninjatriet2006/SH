[Pattern Docs]
# user_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri (invoke) liên quan đến quản lý người dùng (User) tại tầng Bridge.

- **Tên hàm**: `addUser`
- **Mô tả**: Gọi xuống Rust backend để thêm mới một người dùng.
- **Tham số đầu vào**:
  - `username: string` (Bắt buộc)
  - `email?: string` (Tùy chọn)
- **Đầu ra**: `Promise<User>`

- **Tên hàm**: `updateUser`
- **Mô tả**: Gọi xuống Rust backend để cập nhật thông tin người dùng.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
  - `username?: string` (Tùy chọn)
  - `email?: string` (Tùy chọn)
- **Đầu ra**: `Promise<User>`

- **Tên hàm**: `deleteUser`
- **Mô tả**: Gọi xuống Rust backend để xóa một người dùng.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `listUsers`
- **Mô tả**: Gọi xuống Rust backend để lấy danh sách người dùng.
- **Tham số đầu vào**:
  - `page?: number` (Tùy chọn)
  - `limit?: number` (Tùy chọn)
- **Đầu ra**: `Promise<User[]>`
