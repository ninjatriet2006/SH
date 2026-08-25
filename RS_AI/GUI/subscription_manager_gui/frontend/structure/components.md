[Pattern Docs]
# components.md

Tài liệu cấu trúc các Component UI tái sử dụng cho hệ thống.

- **Tên hàm**: `UserModal` (Component)
- **Mô tả**: Form popup dùng để thêm mới hoặc chỉnh sửa thông tin một người dùng.
- **Tham số đầu vào**:
  - `isOpen: boolean` (Bắt buộc)
  - `userData: User | null` (Tùy chọn)
- **Đầu ra**: `Emit Event (onSave, onClose)`

- **Tên hàm**: `PackageModal` (Component)
- **Mô tả**: Form popup dùng để tạo hoặc cấu hình gói dịch vụ.
- **Tham số đầu vào**:
  - `isOpen: boolean` (Bắt buộc)
  - `packageData: Package | null` (Tùy chọn)
- **Đầu ra**: `Emit Event (onSave, onClose)`

- **Tên hàm**: `SubscriptionModal` (Component)
- **Mô tả**: Form popup gán gói dịch vụ cho một User và cho phép thiết lập tùy chỉnh ngày hết hạn.
- **Tham số đầu vào**:
  - `isOpen: boolean` (Bắt buộc)
  - `selectedUserId: string` (Bắt buộc)
  - `subscriptionData: Subscription | null` (Tùy chọn)
- **Đầu ra**: `Emit Event (onSave, onClose)`
