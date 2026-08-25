[Pattern Docs]
# job_monitor_api.md

- **Tên hàm**: `get_active_jobs`
- **Mô tả**: Lấy danh sách toàn bộ các công việc (Jobs) bất đồng bộ đang chạy ngầm trên rclone daemon. Tương đương tính năng theo dõi hàng đợi của TUI.
- **Tham số đầu vào**: 
  - (Không có)
- **Đầu ra**: Mảng Object Job {job_id, name, type, status, start_time}.

- **Tên hàm**: `get_job_stats`
- **Mô tả**: Lấy chi tiết thông số truyền tải (tốc độ mạng, số byte đã truyền, thời gian ước tính) của một tác vụ, dùng cho Transfer Drawer (filen_gui logic).
- **Tham số đầu vào**:
  - `job_id` (Bắt buộc): ID tác vụ.
- **Đầu ra**: Object Stats {speed, transferred_bytes, total_bytes, eta}.

- **Tên hàm**: `stop_job`
- **Mô tả**: Hủy một tác vụ đang chạy thông qua rclone RPC (operations/cancel).
- **Tham số đầu vào**:
  - `job_id` (Bắt buộc): ID tác vụ.
- **Đầu ra**: Boolean xác nhận lệnh hủy thành công.
