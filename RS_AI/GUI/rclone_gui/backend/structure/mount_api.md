[Pattern Docs]
# mount_api.md

- **Tên hàm**: `mount_vfs`
- **Mô tả**: Mount một remote thành ổ đĩa ảo bằng tính năng rclone mount. Hỗ trợ đầy đủ các tham số VFS (vfs-cache-mode, vfs-read-chunk-size, vfs-cache-max-age).
- **Tham số đầu vào**:
  - `remote_path` (Bắt buộc): Đường dẫn remote.
  - `mount_point` (Bắt buộc): Đường dẫn cục bộ (VD: `X:\` trên Windows hoặc `/mnt/cloud` trên Linux).
  - `vfs_cache_mode` (Tùy chọn): off, minimal, writes, full. (Mặc định: full).
  - `auto_start` (Tùy chọn): true/false để tạo service chạy ngầm khi khởi động.
- **Đầu ra**: MountID hoặc Object mô tả tiến trình đang chạy.

- **Tên hàm**: `unmount_vfs`
- **Mô tả**: Ngắt kết nối ổ đĩa ảo. Gọi system unmount hoặc kill process rclone đang giữ mount point.
- **Tham số đầu vào**:
  - `mount_id_or_path` (Bắt buộc): Mount ID hoặc thư mục cục bộ đang mount.
- **Đầu ra**: Kết quả thành công hoặc thất bại.

- **Tên hàm**: `create_systemd_service`
- **Mô tả**: (Chỉ Linux) Kế thừa từ `services.rs` của TUI, dùng để tạo một Systemd service chạy ngầm cho tác vụ Mount tự động.
- **Tham số đầu vào**:
  - `service_name` (Bắt buộc): Tên của service (VD: `rclone-gdrive`).
  - `rclone_args` (Bắt buộc): Chuỗi tham số rclone.
- **Đầu ra**: Trạng thái cài đặt service (Enable/Start).
