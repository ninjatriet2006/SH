[Pattern Docs]
# remote_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri liên quan đến quản lý và kiểm tra Remote Cloud.

- **Tên hàm**: `listRemotes`
- **Mô tả**: Liệt kê toàn bộ các remote đã cấu hình từ rclone.
- **Đầu ra**: `Promise<RemoteConfig[]>`

- **Tên hàm**: `getProviders`
- **Mô tả**: Lấy danh sách các nhà cung cấp cloud hỗ trợ cùng các tuỳ chọn cấu hình.
- **Đầu ra**: `Promise<ProviderInfo[]>`

- **Tên hàm**: `createRemote` / `updateRemote` / `deleteRemote`
- **Mô tả**: Thêm, sửa hoặc xoá cấu hình remote.
- **Đầu ra**: `Promise<string>`

- **Tên hàm**: `getFeatureFlags`
- **Mô tả**: Lấy toàn bộ 52 cờ năng lực của remote (about, move_native, list_r, duplicate_files...).
- **Đầu ra**: `Promise<FeatureFlags | null>`

- **Tên hàm**: `checkTransferCapability`
- **Mô tả**: Kiểm tra khả năng copy/move giữa 2 vị trí nguồn và đích (não checkcap).
- **Đầu ra**: `Promise<TransferCapability>`

- **Tên hàm**: `getAbout` / `getSize`
- **Mô tả**: Lấy thông tin dung lượng và thống kê file của remote.
- **Đầu ra**: `Promise<AboutInfo>` / `Promise<SizeInfo>`

- **Tên hàm**: `checkFilesIntegrity`
- **Mô tả**: Kiểm tra toàn vẹn dữ liệu giữa hai nguồn (rclone check --combined).
- **Đầu ra**: `Promise<IntegrityCheckResult>`
