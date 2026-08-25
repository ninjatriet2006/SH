[Pattern Docs]
# package_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri (invoke) liên quan đến quản lý gói dịch vụ (Package) tại tầng Bridge.

- **Tên hàm**: `addPackage`
- **Mô tả**: Gọi xuống Rust backend để tạo mới gói dịch vụ.
- **Tham số đầu vào**:
  - `name: string` (Bắt buộc)
  - `durationDays: number` (Bắt buộc)
  - `description?: string` (Tùy chọn)
- **Đầu ra**: `Promise<Package>`

- **Tên hàm**: `updatePackage`
- **Mô tả**: Gọi xuống Rust backend để sửa thông tin gói dịch vụ.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
  - `name?: string` (Tùy chọn)
  - `durationDays?: number` (Tùy chọn)
  - `description?: string` (Tùy chọn)
- **Đầu ra**: `Promise<Package>`

- **Tên hàm**: `deletePackage`
- **Mô tả**: Gọi xuống Rust backend để xóa gói dịch vụ.
- **Tham số đầu vào**:
  - `id: string` (Bắt buộc)
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `listPackages`
- **Mô tả**: Gọi xuống Rust backend để lấy danh sách các gói.
- **Tham số đầu vào**:
  - `page?: number` (Tùy chọn)
  - `limit?: number` (Tùy chọn)
- **Đầu ra**: `Promise<Package[]>`
