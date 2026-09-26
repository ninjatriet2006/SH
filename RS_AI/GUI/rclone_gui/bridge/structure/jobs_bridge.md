[Pattern Docs]
# jobs_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri điều khiển hàng đợi tiến trình (Job Queue).

- **Tên hàm**: `jobEnqueue`
- **Mô tả**: Đưa một công việc mới vào hàng đợi (copy, move, delete, rename, mkdir, touch).
- **Tham số đầu vào**: `kind: JobKind`, `src?: string | null`, `dst?: string | null`, `skipPaths?: string[]`
- **Đầu ra**: `Promise<Job>`

- **Tên hàm**: `jobList`
- **Mô tả**: Lấy snapshot danh sách toàn bộ các job trong hệ thống (chờ, đang chạy, đã xong, lỗi).
- **Đầu ra**: `Promise<Job[]>`

- **Tên hàm**: `jobCancel`
- **Mô tả**: Huỷ bỏ một công việc theo `jobId`.
- **Đầu ra**: `Promise<Job>`

- **Tên hàm**: `jobGetQueue`
- **Mô tả**: Lấy danh sách ID các công việc đang nằm trong hàng chờ thực thi.
- **Đầu ra**: `Promise<string[]>`

- **Tên hàm**: `jobReorder` / `jobMoveUp` / `jobMoveDown` / `jobMoveToTop`
- **Mô tả**: Điều chỉnh thứ tự ưu tiên các công việc trong hàng đợi.
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `subscribeJobUpdates`
- **Mô tả**: Đăng ký lắng nghe sự kiện `job_update` từ worker backend.
- **Đầu ra**: `Promise<UnlistenFn>`
