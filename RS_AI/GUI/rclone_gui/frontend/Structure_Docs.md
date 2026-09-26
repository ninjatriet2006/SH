[Pattern Docs]
# KIẾN TRÚC FRONTEND (REACT 19 + ZUSTAND + VITE)

Dự án Rclone GUI đã được chuẩn hoá toàn diện theo mô hình kiến trúc hiện đại (tương đồng với `subscription_manager_gui`).

## 1. Cấu trúc Thư mục

- **`index.html`**: Điểm neo DOM đơn giản `<div id="root"></div>` nạp module `src/main.tsx`.
- **`src/main.tsx`**: Khởi chạy ứng dụng React trong `StrictMode` kết hợp `HashRouter`.
- **`src/App.tsx`**: Layout tổng quan gồm thanh điều hướng `Sidebar` và các Routes tới các màn hình chức năng.
- **`src/index.css`**: Hệ thống thiết kế chuẩn (Deep space radial gradient, Glassmorphism, CSS Custom Properties).
- **`src/store/`**: Quản lý trạng thái toàn cục thông qua Zustand (v5):
  - `useExplorerStore.ts`: Quản lý 2 khung duyệt file (`left`, `right`), lịch sử back/forward, lựa chọn, sắp xếp, lọc.
  - `useRemotesStore.ts`: Quản lý danh sách remote đám mây, 52 cờ tính năng backend, dung lượng (rclone about/size).
  - `useMountsStore.ts`: Quản lý dịch vụ mount systemd (User/System unit), kiểm tra FUSE.
  - `useJobsStore.ts`: Quản lý hàng đợi tác vụ (Job Queue), nhận sự kiện real-time `job_update` từ backend worker.
  - `useTrashStore.ts`: Quản lý thùng rác hệ điều hành (Local) và đám mây (Cloud Remote).
  - `useAppearanceStore.ts`: Quản lý chủ đề màu sắc, phông chữ và từ điển đa ngôn ngữ (i18n).
  - `useSettingsStore.ts`: Quản lý cờ hiệu năng engine rclone, cấu hình xoay vòng log, bản sao lưu snapshot cấu hình.
- **`src/pages/`**: Các trang chức năng chính:
  - `ExplorerPage.tsx`: Trình duyệt tệp tin hai khung (Dual-Pane Explorer) kèm các modal giải quyết xung đột, thuộc tính.
  - `RemotesPage.tsx`: Danh mục Remote Cloud, bảng chi tiết năng lực (features) và tạo mới remote.
  - `MountsPage.tsx`: Giám sát và điều khiển điểm mount rclone qua systemd (Start/Stop/Restart/Enable/Disable).
  - `TransfersPage.tsx`: Bảng điều khiển tiến trình truyền tải file với thanh tiến độ %, reorder hàng đợi.
  - `TrashPage.tsx`: Quản lý thùng rác cục bộ và thùng rác đám mây.
  - `SettingsPage.tsx`: Cài đặt cờ engine, giao diện, snapshot cấu hình và đọc `backend.log` theo yêu cầu (Pull-on-demand).
- **`src/components/`**: Các thành phần tái sử dụng (`Sidebar`, `Breadcrumbs`, `FileTable`, `DualPaneExplorer`, `ConflictModal`, `PropertiesModal`, `SearchModal`, `CreateRemoteModal`, `CreateMountModal`).
- **`src/utils/`**: Tiện ích định dạng byte/thời gian (`formatters.ts`) và dịch giao diện (`i18n.ts`).
- **`../bridge/`**: Cổng giao tiếp chuẩn Enveloped IPC Pattern (A.1 Contract), sạch sẽ và không sử dụng shim tương thích ngược.

## 2. Tiêu chuẩn Giao tiếp IPC & Bridge

- Giao thức A.1 Enveloped Contract: `Req<P> -> IpcResult<T>`.
- Mỗi module bridge mang tên `*_bridge.ts` và sử dụng `invokeCommand<T, P>`.
- Toàn bộ tài liệu chi tiết của từng bridge và trang được đặt trong thư mục `structure/` (`bridge/structure/` và `frontend/structure/`).
