[Pattern Docs]
# remote_api.md

- **Tên hàm**: `list_remotes`
- **Mô tả**: Liệt kê danh sách các cấu hình remote đã thiết lập trong file rclone.conf.
- **Tham số đầu vào**: 
  - (Không có)
- **Đầu ra**: Mảng chuỗi tên Remote.

- **Tên hàm**: `dump_config`
- **Mô tả**: Lấy toàn bộ dữ liệu cấu hình rclone (lọc bỏ mật khẩu nếu cần) để hiển thị trên giao diện quản trị Remote.
- **Tham số đầu vào**:
  - `obfuscate_passwords` (Tùy chọn): true/false để ẩn mật khẩu.
- **Đầu ra**: Chuỗi JSON chứa cấu hình.

- **Tên hàm**: `create_remote`
- **Mô tả**: Tạo một cấu hình rclone mới (kể cả Crypt, Union, Alias) qua RPC `config/create`.
- **Tham số đầu vào**:
  - `name` (Bắt buộc): Tên remote.
  - `type` (Bắt buộc): Loại dịch vụ (gdrive, s3, crypt, v.v.).
  - `parameters` (Bắt buộc): Đối tượng JSON Key-Value chứa tham số cấu hình (VD: client_id, secret).
- **Đầu ra**: Trạng thái thành công.

- **Tên hàm**: `delete_remote`
- **Mô tả**: Xóa cấu hình của một remote hiện có.
- **Tham số đầu vào**:
  - `name` (Bắt buộc): Tên remote cần xóa.
- **Đầu ra**: Trạng thái thành công.
