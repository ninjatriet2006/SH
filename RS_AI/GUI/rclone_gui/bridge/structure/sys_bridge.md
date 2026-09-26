[Pattern Docs]
# sys_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri tương tác hệ điều hành, clipboard và custom actions.

- **Tên hàm**: `sysOpenWith`
- **Mô tả**: Mở file bằng ứng dụng cụ thể hoặc ứng dụng mặc định.
- **Tham số đầu vào**: `path: string`, `execCmd?: string | null`, `app?: string | null`
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `sysListApps`
- **Mô tả**: Quét danh sách các ứng dụng .desktop trên hệ điều hành Linux.
- **Đầu ra**: `Promise<DesktopApp[]>`

- **Tên hàm**: `osClipboardSet` / `osClipboardGet`
- **Mô tả**: Đặt hoặc lấy nội dung clipboard nội bộ của trình quản lý file.
- **Đầu ra**: `Promise<void>` / `Promise<OSClipboardData | null>`

- **Tên hàm**: `sysGetCustomActions` / `sysGetValidActions`
- **Mô tả**: Quét cấu hình hoặc lọc các thao tác tuỳ chỉnh hợp lệ với danh sách file được chọn.
- **Đầu ra**: `Promise<CustomAction[]>`

- **Tên hàm**: `sysExecuteCustomAction`
- **Mô tả**: Khởi chạy lệnh tuỳ chỉnh theo mẫu template với danh sách tệp tin.
- **Đầu ra**: `Promise<void>`
