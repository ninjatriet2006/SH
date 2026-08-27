/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc quản lý cấu hình rclone.
Trách nhiệm: Gọi lệnh get_config_content, set_config_content.
Các module tương tác: frontend/src/features/remotesManager.ts, backend/src/config.rs
*/

// @ts-ignore
import { invoke } from '@tauri-apps/api/core';

export async function getConfigContent(): Promise<string> {
    try {
        return await invoke<string>('get_config_content');
    } catch (error) {
        console.error("Lỗi khi đọc config:", error);
        throw error;
    }
}

export async function setConfigContent(content: string): Promise<void> {
    try {
        await invoke('set_config_content', { content });
    } catch (error) {
        console.error("Lỗi khi ghi config:", error);
        throw error;
    }
}

export async function reorderConfig(names: string[]): Promise<void> {
    try {
        await invoke('reorder_config', { names });
    } catch (error) {
        console.error("Lỗi khi sắp xếp config:", error);
        throw error;
    }
}
