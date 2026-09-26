[Pattern Docs]
# pages.md

Tài liệu cấu trúc các trang giao diện chính (Views/Pages) của hệ thống Rclone GUI theo chuẩn React 19 + Zustand.

- **Tên hàm**: `ExplorerPage` (Component)
- **Mô tả**: Trang duyệt tệp tin hai khung (Dual-Pane File Explorer), quản lý điều hướng, lịch sử, các tác vụ sao chép/di chuyển/xoá và giải quyết xung đột tệp.
- **Đầu ra**: `UI View`

- **Tên hàm**: `RemotesPage` (Component)
- **Mô tả**: Trang quản trị các Remote đám mây, hiển thị dung lượng (rclone about), số lượng tệp (rclone size), kiểm tra 52 cờ năng lực backend, thêm/sửa/xoá và xuất cấu hình INI.
- **Đầu ra**: `UI View`

- **Tên hàm**: `MountsPage` (Component)
- **Mô tả**: Trang quản lý các dịch vụ Systemd Mount, theo dõi trạng thái FUSE, bật/tắt/khởi động lại mount point và tạo mount service mới.
- **Đầu ra**: `UI View`

- **Tên hàm**: `TransfersPage` (Component)
- **Mô tả**: Trang giám sát hàng đợi tiến trình (Job Queue), cập nhật % tiến độ và số vé con real-time qua event `job_update`, điều chỉnh thứ tự ưu tiên hoặc huỷ bỏ tác vụ.
- **Đầu ra**: `UI View`

- **Tên hàm**: `TrashPage` (Component)
- **Mô tả**: Trang quản lý Thùng rác (Trash) cho cả ổ cục bộ (Local) và ổ đám mây (Cloud Remote), hỗ trợ khôi phục hoặc xoá vĩnh viễn.
- **Đầu ra**: `UI View`

- **Tên hàm**: `SettingsPage` (Component)
- **Mô tả**: Trang thiết lập hiệu năng engine rclone (transfers, checkers, fast_list, backup_dir), tuỳ biến theme/font/ngôn ngữ, quản lý snapshot rclone.conf và xem log backend theo yêu cầu.
- **Đầu ra**: `UI View`
