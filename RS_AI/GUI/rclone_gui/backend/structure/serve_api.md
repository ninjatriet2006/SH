[Pattern Docs]
# serve_api.md

- **Tên hàm**: `start_server`
- **Mô tả**: Khởi tạo dịch vụ Network Server cho một remote (HTTP, FTP, WebDAV, DLNA) để truy cập nội dung qua LAN. (Ánh xạ từ tính năng ServersDashboard của filen_gui).
- **Tham số đầu vào**:
  - `remote_path` (Bắt buộc): Đường dẫn rclone.
  - `protocol` (Bắt buộc): enum (http, ftp, webdav, dlna).
  - `bind_address` (Tùy chọn): Địa chỉ IP:Port (Mặc định: 127.0.0.1:8080).
  - `auth_user` (Tùy chọn): Tên đăng nhập để bảo mật.
  - `auth_pass` (Tùy chọn): Mật khẩu truy cập.
- **Đầu ra**: ServerID hoặc trạng thái chạy ngầm.

- **Tên hàm**: `stop_server`
- **Mô tả**: Dừng dịch vụ Network Server đang chạy.
- **Tham số đầu vào**:
  - `server_id` (Bắt buộc): ID hoặc PID của server.
- **Đầu ra**: Trạng thái thành công.
