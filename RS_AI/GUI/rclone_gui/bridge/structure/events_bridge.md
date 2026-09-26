[Pattern Docs]
# events_bridge.ts

Tài liệu cấu trúc các hàm lắng nghe sự kiện từ Tauri backend (Watcher & Job updates).

- **Tên hàm**: `subscribeLocalDirChanged`
- **Mô tả**: Lắng nghe sự kiện `local-dir-changed` khi thư mục local được theo dõi có biến động trên ổ đĩa.
- **Tham số đầu vào**: `cb: () => void`
- **Đầu ra**: `Promise<UnlistenFn>`

- **Tên hàm**: `subscribeJobUpdates`
- **Mô tả**: Lắng nghe sự kiện `job_update` khi có tiến trình công việc thay đổi tiến độ hoặc trạng thái.
- **Tham số đầu vào**: `cb: (job: Job) => void`
- **Đầu ra**: `Promise<UnlistenFn>`
