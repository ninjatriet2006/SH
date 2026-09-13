import { invokeIpc } from './ipc';
import type { FontInfo } from './types';

export async function getAvailableFonts(): Promise<FontInfo[]> {
    try {
        return await invokeIpc<FontInfo[]>('get_available_fonts');
    } catch (error) {
        console.error('Lỗi lấy danh sách font:', error);
        return [];
    }
}
