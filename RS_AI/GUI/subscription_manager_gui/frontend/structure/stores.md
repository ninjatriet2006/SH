[Pattern Docs]
# stores.md

Tài liệu cấu trúc các State Stores (ví dụ sử dụng Pinia, Redux hoặc Context) để quản lý dữ liệu toàn cục.

- **Tên hàm**: `useUserStore`
- **Mô tả**: Lưu trữ và quản lý trạng thái của danh sách người dùng đã được tải về.
- **Tham số đầu vào**:
  - `initialData: User[]` (Tùy chọn)
- **Đầu ra**: `Store (state, getters, actions)`

- **Tên hàm**: `usePackageStore`
- **Mô tả**: Lưu trữ và quản lý danh sách các gói dịch vụ có sẵn.
- **Tham số đầu vào**:
  - `initialData: Package[]` (Tùy chọn)
- **Đầu ra**: `Store (state, getters, actions)`

- **Tên hàm**: `useSubscriptionStore`
- **Mô tả**: Quản lý bộ nhớ đệm cho các subscriptions của user đang được chọn, để tránh gọi API nhiều lần.
- **Tham số đầu vào**:
  - `userId: string` (Bắt buộc)
- **Đầu ra**: `Store (state, getters, actions)`
