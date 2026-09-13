import { invokeCommand, type Empty, type FontInfo } from './types';

// Gọi API lấy danh sách Font
export async function getAvailableFonts(): Promise<FontInfo[]> {
    try {
        return await invokeCommand<FontInfo[], Empty>('get_available_fonts', {});
    } catch (error) {
        console.error("Lỗi lấy danh sách fonts:", error);
        return [];
    }
}
