[Pattern Docs]
# types.ts

Tài liệu cấu trúc các Interface TypeScript định nghĩa dữ liệu để giao tiếp với Rust backend thông qua Tauri.

- **Tên hàm**: `User` (Interface)
- **Mô tả**: Giao diện dữ liệu mô tả thông tin người dùng từ Backend.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
  - `username: string` (Bắt buộc)
  - `email: string | null` (Tùy chọn)
  - `created_at: number` (Bắt buộc)
- **Đầu ra**: `N/A`

- **Tên hàm**: `Package` (Interface)
- **Mô tả**: Giao diện dữ liệu mô tả gói dịch vụ.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
  - `name: string` (Bắt buộc)
  - `description: string | null` (Tùy chọn)
  - `duration_days: number` (Bắt buộc)
- **Đầu ra**: `N/A`

- **Tên hàm**: `Subscription` (Interface)
- **Mô tả**: Giao diện dữ liệu mô tả gói đăng ký của người dùng.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
  - `user_id: string` (Bắt buộc)
  - `package_id: string` (Bắt buộc)
  - `expiration_date: number` (Bắt buộc)
  - `is_active: boolean` (Bắt buộc)
- **Đầu ra**: `N/A`
