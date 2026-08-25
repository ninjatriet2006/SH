[Pattern Docs]
# pages.md

Tài liệu cấu trúc các trang giao diện chính (Views/Pages) của hệ thống Subscription Manager.

- **Tên hàm**: `DashboardPage` (Component)
- **Mô tả**: Trang tổng quan, hiển thị thống kê số lượng User, Package và Subscription đang hoạt động.
- **Tham số đầu vào**:
  - `metricsData: object` (Tùy chọn)
- **Đầu ra**: `UI View`

- **Tên hàm**: `UserManagementPage` (Component)
- **Mô tả**: Trang danh sách người dùng, tích hợp bảng dữ liệu và thanh công cụ thêm/sửa/xóa.
- **Tham số đầu vào**:
  - `initialPage: number` (Tùy chọn)
- **Đầu ra**: `UI View`

- **Tên hàm**: `PackageManagementPage` (Component)
- **Mô tả**: Trang quản lý các gói dịch vụ (Tạo, cập nhật thông tin gói).
- **Tham số đầu vào**:
  - `initialPage: number` (Tùy chọn)
- **Đầu ra**: `UI View`
