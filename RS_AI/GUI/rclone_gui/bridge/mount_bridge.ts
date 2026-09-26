/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho quản lý dịch vụ Mount qua Systemd.
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng MountConfig và SystemdServiceInfo từ `types.ts`.
*/

import { invokeCommand, type MountConfig, type SystemdServiceInfo } from './types';

/** Kiểm tra hệ thống Linux đã cài đặt thư viện FUSE hay chưa. */
export async function checkFuseInstalled(): Promise<boolean> {
  try {
    return await invokeCommand<boolean>('check_fuse_installed');
  } catch (error) {
    console.error('Lỗi check_fuse_installed:', error);
    return false;
  }
}

/** Tạo hoặc cập nhật systemd service cho mount point. */
export async function createMountService(config: MountConfig, confirmed: boolean = false): Promise<string> {
  return await invokeCommand<string, { config: MountConfig; confirmed: boolean }>('create_mount_service', {
    config,
    confirmed,
  });
}

/** Xoá systemd service mount point. */
export async function deleteMountService(
  serviceName: string,
  isUser: boolean,
  confirmed: boolean = false,
): Promise<string> {
  return await invokeCommand<string, { service_name: string; is_user: boolean; confirmed: boolean }>(
    'delete_mount_service',
    {
      service_name: serviceName,
      is_user: isUser,
      confirmed,
    },
  );
}

/** Quản lý trạng thái service mount (start, stop, enable, disable, restart). */
export async function manageMountService(
  serviceName: string,
  isUser: boolean,
  action: 'start' | 'stop' | 'enable' | 'disable' | 'restart',
  confirmed: boolean = false,
): Promise<string> {
  return await invokeCommand<
    string,
    { service_name: string; is_user: boolean; action: string; confirmed: boolean }
  >('manage_mount_service', {
    service_name: serviceName,
    is_user: isUser,
    action,
    confirmed,
  });
}

/** Liệt kê toàn bộ các dịch vụ rclone mount qua systemd (user & system). */
export async function listMountServices(): Promise<SystemdServiceInfo[]> {
  try {
    return await invokeCommand<SystemdServiceInfo[]>('list_mount_services');
  } catch (error) {
    console.error('Lỗi list_mount_services:', error);
    return [];
  }
}

/** Đọc cấu hình chi tiết của một service mount. */
export async function getMountServiceConfig(serviceName: string, isUser: boolean): Promise<MountConfig | null> {
  try {
    return await invokeCommand<MountConfig, { service_name: string; is_user: boolean }>(
      'get_mount_service_config',
      {
        service_name: serviceName,
        is_user: isUser,
      },
    );
  } catch (error) {
    console.error(`Lỗi get_mount_service_config ${serviceName}:`, error);
    return null;
  }
}
