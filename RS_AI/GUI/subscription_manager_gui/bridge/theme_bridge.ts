import { invokeCommand, type Empty, type Theme } from './types';

// Gọi API lấy danh sách Theme
export async function getAvailableThemes(): Promise<Theme[]> {
    try {
        return await invokeCommand<Theme[], Empty>('get_available_themes', {});
    } catch (error) {
        console.error("Lỗi lấy danh sách themes:", error);
        return [];
    }
}
