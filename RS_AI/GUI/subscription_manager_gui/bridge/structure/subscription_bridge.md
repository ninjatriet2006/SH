[Pattern Docs]
# subscription_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri (invoke) liên quan đến gán gói dịch vụ và cập nhật thời hạn (Subscription) tại tầng Bridge.

- **Tên hàm**: `addSubscriptionToUser`
- **Mô tả**: Gán gói dịch vụ cho người dùng (có thể chỉ định ngày hết hạn).
- **Tham số đầu vào**:
  - `userId: string` (Bắt buộc)
  - `packageId: string` (Bắt buộc)
  - `customExpirationDate?: number` (Tùy chọn)
- **Đầu ra**: `Promise<Subscription>`

- **Tên hàm**: `updateSubscriptionExpiry`
- **Mô tả**: Thay đổi, cập nhật ngày hết hạn của gói đang sử dụng.
- **Tham số đầu vào**:
  - `subscriptionId: string` (Bắt buộc)
  - `newExpirationDate: number` (Bắt buộc)
- **Đầu ra**: `Promise<Subscription>`

- **Tên hàm**: `removeSubscriptionFromUser`
- **Mô tả**: Xóa hoặc vô hiệu hóa gói đăng ký của người dùng.
- **Tham số đầu vào**:
  - `subscriptionId: string` (Bắt buộc)
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `listUserSubscriptions`
- **Mô tả**: Lấy danh sách các gói dịch vụ của một người dùng cụ thể.
- **Tham số đầu vào**:
  - `userId: string` (Bắt buộc)
- **Đầu ra**: `Promise<Subscription[]>`

- **Tên hàm**: `checkSubscriptionStatus`
- **Mô tả**: Yêu cầu backend kiểm tra và cập nhật trạng thái hạn sử dụng của gói.
- **Tham số đầu vào**:
  - `subscriptionId: string` (Bắt buộc)
- **Đầu ra**: `Promise<boolean>`
