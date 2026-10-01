[Pattern Docs]
# stores.md

Tài liệu cấu trúc các Store quản lý trạng thái toàn cục (Zustand Stores) của Universe Manager GUI.

- **Tên Store**: `useAppStore`
- **Mô tả**:
  - Quản lý cấu hình hệ thống (`ManagerConfig`), danh sách ứng dụng quét được (`apps: AppEntry[]`), kết quả tìm kiếm (`searchReport: SearchReport | null`), kết quả phát hiện ứng dụng nguồn (`detectionReport: DetectionReport | null`).
  - Quản lý trạng thái công việc nền (`busy`, `activeJob`, `jobProgress`, `jobError`) thông qua kênh phát sự kiện JobClient của Tauri.
  - Cung cấp các action: `loadConfig`, `saveConfig`, `scanApps`, `detectApp`, `startApp`, `stopApp`, `searchApps`.

- **Tên Store**: `useSettingsStore`
- **Mô tả**:
  - Quản lý tùy chọn hệ thống (`Preferences`: language, theme, font_id).
  - Tải từ điển đa ngôn ngữ (`Messages`) từ file JSON đóng gói hoặc fallback an toàn.
  - Tải và tiêm tokens chủ đề CSS (CSS Custom Properties) vào `:root` và nạp động phông chữ DejaVuSans.
  - Cung cấp các action: `initSettings`, `updateSettings`.
