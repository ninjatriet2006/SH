[Pattern Docs]
# subscription_api.rs

Tài liệu cấu trúc các hàm thao tác liên quan đến gán gói dịch vụ và thay đổi thời gian hết hạn của gói (Subscription).

- **Tên hàm**: `add_subscription_to_user`
- **Mô tả**: Gán một gói dịch vụ cho một người dùng với ngày hết hạn được tính tự động từ package, hoặc do admin quyết định.
- **Tham số đầu vào**:
  - `user_id: String` (Bắt buộc)
  - `package_id: String` (Bắt buộc)
  - `custom_expiration_date: Option<i64>` (Tùy chọn)
- **Đầu ra**: `Result<Subscription, String>`

- **Tên hàm**: `update_subscription_expiry`
- **Mô tả**: Thay đổi, quyết định ngày tháng hết hạn mới của một subscription hiện có.
- **Tham số đầu vào**:
  - `subscription_id: String` (Bắt buộc)
  - `new_expiration_date: i64` (Bắt buộc)
- **Đầu ra**: `Result<Subscription, String>`

- **Tên hàm**: `remove_subscription_from_user`
- **Mô tả**: Hủy gói đăng ký dịch vụ của người dùng (Xóa subscription hoặc vô hiệu hóa).
- **Tham số đầu vào**:
  - `subscription_id: String` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `list_user_subscriptions`
- **Mô tả**: Lấy danh sách toàn bộ các gói dịch vụ mà một người dùng đang sử dụng.
- **Tham số đầu vào**:
  - `user_id: String` (Bắt buộc)
- **Đầu ra**: `Result<Vec<Subscription>, String>`

- **Tên hàm**: `check_subscription_status`
- **Mô tả**: Kiểm tra trạng thái hết hạn của subscription và vô hiệu hóa nếu thời hạn đã qua.
- **Tham số đầu vào**:
  - `subscription_id: String` (Bắt buộc)
- **Đầu ra**: `Result<bool, String>`
