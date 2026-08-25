import { invoke } from '@tauri-apps/api/core';
import type { FontInfo } from './types';

// Gọi API lấy danh sách Font
export async function getAvailableFonts(): Promise<FontInfo[]> {
    try {
        return await invoke<FontInfo[]>('get_available_fonts');
    } catch (error) {
        console.error("Lỗi lấy danh sách fonts:", error);
        return [];
    }
}
