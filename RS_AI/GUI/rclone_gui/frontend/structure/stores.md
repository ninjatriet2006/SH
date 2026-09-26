[Pattern Docs]
# stores.md

Tài liệu cấu trúc các Store quản lý trạng thái toàn cục (Zustand Stores) của ứng dụng Rclone GUI.

- **Tên Store**: `useExplorerStore`
- **Mô tả**: Quản lý trạng thái 2 khung duyệt file (`left`, `right`), thư mục hiện tại, lịch sử back/forward, danh sách file, mục được chọn, thứ tự sắp xếp và bộ lọc tìm kiếm.
- **Phương thức chính**: `loadDirectory`, `navigateUp`, `goBack`, `goForward`, `setSort`, `toggleSelect`, `selectAll`, `clearSelection`, `setSearchQuery`, `toggleBookmark`.

- **Tên Store**: `useRemotesStore`
- **Mô tả**: Quản lý danh sách remote đám mây từ `rclone.conf`, thông tin nhà cung cấp cloud (providers), bộ nhớ dung lượng và 52 cờ tính năng backend.
- **Phương thức chính**: `loadRemotes`, `loadProviders`, `fetchRemoteDetails`, `createRemote`, `updateRemote`, `deleteRemote`, `checkIntegrity`.

- **Tên Store**: `useMountsStore`
- **Mô tả**: Quản lý các dịch vụ mount systemd, kiểm tra thư viện FUSE trên máy, điều khiển start/stop/restart/enable/disable service.
- **Phương thức chính**: `loadMounts`, `createMount`, `deleteMount`, `manageMount`.

- **Tên Store**: `useJobsStore`
- **Mô tả**: Quản lý hàng đợi công việc (Job Queue), nhận cập nhật tiến độ bất đồng bộ qua sự kiện `job_update`, đảo vị trí ưu tiên trong hàng đợi.
- **Phương thức chính**: `loadJobs`, `enqueueJob`, `cancelJob`, `reorderQueue`, `moveJobUp`, `moveJobDown`, `moveJobToTop`, `initSubscription`.

- **Tên Store**: `useTrashStore`
- **Mô tả**: Quản lý các mục trong thùng rác cục bộ và đám mây, kích hoạt khôi phục hoặc xoá vĩnh viễn.
- **Phương thức chính**: `loadLocalTrash`, `restoreLocal`, `deleteLocal`, `emptyLocal`, `loadRemoteTrash`, `restoreRemote`, `deleteRemote`, `emptyRemote`.

- **Tên Store**: `useAppearanceStore`
- **Mô tả**: Quản lý chủ đề màu sắc, phông chữ và từ điển i18n đa ngôn ngữ, tự động tiêm CSS variables vào `:root`.
- **Phương thức chính**: `initAppearance`, `changeLanguage`, `changeTheme`, `changeFont`.

- **Tên Store**: `useSettingsStore`
- **Mô tả**: Quản lý các cờ hiệu năng engine rclone, cấu hình dung lượng log, bản sao lưu snapshot cấu hình và kéo dữ liệu file `backend.log`.
- **Phương thức chính**: `loadSettings`, `updateEngineFlags`, `updateDebugSettings`, `fetchBackendLog`, `restoreSnapshot`, `exportRemote`, `importRemote`.
