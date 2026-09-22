/*
 * Mount systemd — tương ứng `backend/src/api/mount_manager.rs`.
 * Thao tác nguy hiểm (stop/disable/restart, ghi file service) cần `confirmed`.
 */

import { invoke } from './ipc';
import type { MountConfig, SystemdServiceInfo } from './types';

/** Kiểm tra FUSE đã cài chưa. */
export async function checkFuseInstalled(): Promise<boolean> {
  try {
    return await invoke<boolean>('check_fuse_installed');
  } catch (error) {
    console.error('Lỗi check_fuse_installed:', error);
    return false;
  }
}

/** Tạo service mount mới. */
export async function createMountService(config: MountConfig, confirmed: boolean): Promise<string> {
  return await invoke<string>('create_mount_service', { config, confirmed });
}

/** Dừng + gỡ service mount. */
export async function deleteMountService(serviceName: string, isUser: boolean, confirmed: boolean): Promise<string> {
  return await invoke<string>('delete_mount_service', {
    service_name: serviceName,
    is_user: isUser,
    confirmed,
  });
}

/** Điều khiển service (`start`/`stop`/`enable`/`disable`/`restart`). */
export async function manageMountService(
  serviceName: string,
  isUser: boolean,
  action: string,
  confirmed: boolean,
): Promise<string> {
  return await invoke<string>('manage_mount_service', {
    service_name: serviceName,
    is_user: isUser,
    action,
    confirmed,
  });
}

/** Quét service mount hiện có. */
export async function listMountServices(): Promise<SystemdServiceInfo[]> {
  try {
    return await invoke<SystemdServiceInfo[]>('list_mount_services');
  } catch (error) {
    console.error('Lỗi list_mount_services:', error);
    return [];
  }
}

/** Đọc cấu hình một service. */
export async function getMountServiceConfig(serviceName: string, isUser: boolean): Promise<MountConfig | null> {
  try {
    return await invoke<MountConfig>('get_mount_service_config', {
      service_name: serviceName,
      is_user: isUser,
    });
  } catch (error) {
    console.error('Lỗi get_mount_service_config:', error);
    return null;
  }
}
