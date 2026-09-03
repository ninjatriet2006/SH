/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc quản lý Remote.
Trách nhiệm: Gọi lệnh get_remotes, add_remote, remove_remote.
Các module tương tác: frontend/src/main.ts, backend/src/remote.rs
*/

import { invoke } from '@tauri-apps/api/core';

export interface RemoteConfig {
  name: string;
  type: string;
  [key: string]: any;
}

export interface ProviderOption {
  Name: string;
  Help: string;
  Type: string;
  Required: boolean;
  Advanced: boolean;
  IsPassword?: boolean;
  DefaultStr?: string;
  Examples?: Array<{ Value: string; Help: string }>;
}

export interface ProviderInfo {
  Name: string;
  Description: string;
  Prefix: string;
  Options: ProviderOption[];
}

// Gọi API Rust (gọi lệnh rclone config dump)
export async function listRemotes(): Promise<RemoteConfig[]> {
  try {
    const remotes = await invoke<RemoteConfig[]>('list_remotes');
    return remotes;
  } catch (error) {
    console.error("Lỗi khi lấy danh sách remote từ backend:", error);
    return [];
  }
}

// Gọi rclone config providers
export async function getProviders(): Promise<ProviderInfo[]> {
  try {
    const jsonStr = await invoke<string>('get_providers');
    return JSON.parse(jsonStr) as ProviderInfo[];
  } catch (error) {
    console.error("Lỗi khi lấy danh sách providers:", error);
    return [];
  }
}

// Gọi rclone config create
export async function createRemote(name: string, provider: string, options: Record<string, string>): Promise<boolean> {
  try {
    await invoke('create_remote', { name, provider, options });
    return true;
  } catch (error) {
    console.error("Lỗi khi tạo remote:", error);
    alert("Lỗi tạo remote: " + error);
    return false;
  }
}

// Gọi rclone config update
export async function updateRemote(name: string, options: Record<string, string>): Promise<boolean> {
  try {
    await invoke('update_remote', { name, options });
    return true;
  } catch (error) {
    console.error("Lỗi khi cập nhật remote:", error);
    alert("Lỗi cập nhật remote: " + error);
    return false;
  }
}

// Gọi rclone config delete
export async function deleteRemote(name: string): Promise<boolean> {
  try {
    await invoke('delete_remote', { name });
    return true;
  } catch (error) {
    console.error("Lỗi khi xóa remote:", error);
    alert("Lỗi xóa remote: " + error);
    return false;
  }
}

// Gọi rclone backend features <remote>:
export async function getBackendFeatures(remote: string): Promise<any> {
  try {
    const features = await invoke<any>('get_backend_features', { remote });
    return features;
  } catch (error) {
    console.error("Lỗi khi lấy backend features:", error);
    return null;
  }
}

// Gọi rclone about <remote>:
export async function getAbout(remote: string): Promise<{ total?: number, used?: number, free?: number, trashed?: number, other?: number }> {
    try {
        const result = await invoke<{ total?: number, used?: number, free?: number, trashed?: number, other?: number }>('rclone_about', { remote });
        return result;
    } catch (e) {
        console.warn("Failed to get about for remote:", remote, e);
        return {};
    }
}

// Gọi rclone size <remote>:
export async function getSize(remote: string): Promise<{ count?: number, bytes?: number, sizeless?: number }> {
    try {
        const result = await invoke<{ count?: number, bytes?: number, sizeless?: number }>('rclone_size', { remote });
        return result;
    } catch (e) {
        console.warn("Failed to get size for remote:", remote, e);
        return {};
    }
}

// Kiểm tra khả năng copy/move giữa 2 đường dẫn:
export async function checkTransferCapability(src: string, dst: string): Promise<{ canMove: boolean, canCopy: boolean, canCopyDelete: boolean }> {
    try {
        const result = await invoke<{ canMove: boolean, canCopy: boolean, canCopyDelete: boolean }>('check_transfer_capability', { src, dst });
        return result;
    } catch (e) {
        console.warn("Failed to check transfer capability:", e);
        return { canMove: false, canCopy: false, canCopyDelete: false };
    }
}
