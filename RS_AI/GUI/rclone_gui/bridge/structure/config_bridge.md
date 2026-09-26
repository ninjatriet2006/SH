[Pattern Docs]
# config_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri thao tác file cấu hình rclone.conf và thiết lập hiệu năng engine.

- **Tên hàm**: `getConfigContent` / `setConfigContent`
- **Mô tả**: Đọc hoặc ghi đè nội dung file rclone.conf.
- **Đầu ra**: `Promise<string>` / `Promise<void>`

- **Tên hàm**: `reorderConfig`
- **Mô tả**: Thay đổi thứ tự các khối section remote trong rclone.conf.
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `listConfigSnapshots` / `restoreConfigSnapshot`
- **Mô tả**: Liệt kê hoặc phục hồi từ bản sao lưu cấu hình.
- **Đầu ra**: `Promise<string[]>` / `Promise<void>`

- **Tên hàm**: `exportConfigRemote` / `importConfigRemote`
- **Mô tả**: Xuất hoặc nhập cấu hình của một remote cụ thể dưới dạng chuỗi INI.
- **Đầu ra**: `Promise<string>` / `Promise<void>`

- **Tên hàm**: `getEngineFlags` / `setEngineFlags`
- **Mô tả**: Đọc và lưu các cờ rclone engine tối ưu (transfers, checkers, fast_list, backup_dir...).
- **Đầu ra**: `Promise<EngineSettings>`

- **Tên hàm**: `getDebugSettings` / `setDebugSettings`
- **Mô tả**: Quản lý cấu hình chẩn đoán và xoay vòng file log backend.
- **Đầu ra**: `Promise<DebugSettings>`

- **Tên hàm**: `getBackendLog`
- **Mô tả**: Đọc nội dung log backend theo yêu cầu (Pull model).
- **Đầu ra**: `Promise<string>`
