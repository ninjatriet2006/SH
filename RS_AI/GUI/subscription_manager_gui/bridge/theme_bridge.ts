import { invoke } from '@tauri-apps/api/core';
import type { Theme } from './types';

// Gọi API lấy danh sách Theme
export async function getAvailableThemes(): Promise<Theme[]> {
    try {
        return await invoke<Theme[]>('get_available_themes');
    } catch (error) {
        console.error("Lỗi lấy danh sách themes:", error);
        return [];
    }
}
