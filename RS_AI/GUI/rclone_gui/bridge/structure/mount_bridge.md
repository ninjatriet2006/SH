[Pattern Docs]
# mount_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri quản lý dịch vụ Mount Systemd.

- **Tên hàm**: `checkFuseInstalled`
- **Mô tả**: Kiểm tra FUSE có mặt trên hệ thống hay không.
- **Đầu ra**: `Promise<boolean>`

- **Tên hàm**: `createMountService`
- **Mô tả**: Tạo service file systemd cho mount point rclone.
- **Tham số đầu vào**: `config: MountConfig`, `confirmed?: boolean`
- **Đầu ra**: `Promise<string>`

- **Tên hàm**: `deleteMountService`
- **Mô tả**: Dừng, gỡ cài đặt và xoá service file mount.
- **Tham số đầu vào**: `serviceName: string`, `isUser: boolean`, `confirmed?: boolean`
- **Đầu ra**: `Promise<string>`

- **Tên hàm**: `manageMountService`
- **Mô tả**: Điều khiển trạng thái service (start, stop, enable, disable, restart).
- **Tham số đầu vào**: `serviceName: string`, `isUser: boolean`, `action: 'start' | 'stop' | 'enable' | 'disable' | 'restart'`, `confirmed?: boolean`
- **Đầu ra**: `Promise<string>`

- **Tên hàm**: `listMountServices`
- **Mô tả**: Liệt kê các dịch vụ mount hiện có trên máy.
- **Đầu ra**: `Promise<SystemdServiceInfo[]>`

- **Tên hàm**: `getMountServiceConfig`
- **Mô tả**: Đọc lại cấu hình tham số từ một unit file mount.
- **Đầu ra**: `Promise<MountConfig | null>`
