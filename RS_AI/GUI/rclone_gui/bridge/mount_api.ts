/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc quản lý Mount & Systemd Services.
Trách nhiệm: Gọi lệnh kiểm tra fuse, tạo/xoá/quản lý service, lấy danh sách service.
Các module tương tác: frontend/src/features/mountManager.ts, backend/src/mount.rs
*/

// @ts-ignore
import { invoke } from '@tauri-apps/api/core';

export interface MountConfig {
    service_name: string;
    is_user_level: boolean;
    remote: string;
    mount_path: string;
    description: string;
    vfs_cache_mode: string;
    vfs_cache_max_size: string;
    vfs_cache_max_age: string;
    dir_cache_time: string;
    buffer_size: string;
    allow_other: boolean;
    read_only: boolean;
}

export interface SystemdServiceInfo {
    name: string;
    is_user: boolean;
    status: string;
    enabled: boolean;
}

export async function checkFuseInstalled(): Promise<boolean> {
    try {
        return await invoke<boolean>('check_fuse_installed');
    } catch (error) {
        console.error("Lỗi khi kiểm tra FUSE:", error);
        return false;
    }
}

export async function createMountService(config: MountConfig): Promise<boolean> {
    try {
        await invoke('create_mount_service', { config });
        return true;
    } catch (error) {
        console.error("Lỗi khi tạo mount service:", error);
        alert("Lỗi tạo service: " + error);
        return false;
    }
}

export async function deleteMountService(serviceName: string, isUser: boolean): Promise<boolean> {
    try {
        await invoke('delete_mount_service', { serviceName, isUser });
        return true;
    } catch (error) {
        console.error("Lỗi khi xoá mount service:", error);
        alert("Lỗi xoá service: " + error);
        return false;
    }
}

export async function manageMountService(serviceName: string, isUser: boolean, action: string): Promise<boolean> {
    try {
        await invoke('manage_mount_service', { serviceName, isUser, action });
        return true;
    } catch (error) {
        console.error(`Lỗi khi ${action} mount service:`, error);
        alert(`Lỗi thao tác ${action}: ` + error);
        return false;
    }
}

export async function listMountServices(): Promise<SystemdServiceInfo[]> {
    try {
        return await invoke<SystemdServiceInfo[]>('list_mount_services');
    } catch (error) {
        console.error("Lỗi khi lấy danh sách mount services:", error);
        return [];
    }
}

export async function getMountServiceConfig(serviceName: string, isUser: boolean): Promise<MountConfig | null> {
    try {
        return await invoke<MountConfig>('get_mount_service_config', { serviceName, isUser });
    } catch (error) {
        console.error("Lỗi khi lấy cấu hình mount service:", error);
        alert("Lỗi khi đọc cấu hình: " + error);
        return null;
    }
}
