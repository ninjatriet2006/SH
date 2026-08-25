import { invoke } from '@tauri-apps/api/core';

export interface Settings {
    language: string;
    timezone: string;
    theme_id: string;
    font_id: string;
}

// Gọi API lấy cài đặt
export async function getSettings(): Promise<Settings> {
    try {
        return await invoke<Settings>('get_settings');
    } catch (error) {
        console.error("Lỗi lấy cài đặt:", error);
        return { language: 'vi', timezone: 'Asia/Ho_Chi_Minh', theme_id: 'default', font_id: 'default' };
    }
}

// Gọi API lưu cài đặt
export async function saveSettings(language: string, timezone: string, theme_id: string, font_id: string): Promise<void> {
    await invoke('save_settings', { language, timezone, themeId: theme_id, fontId: font_id });
}

// Gọi API lấy danh sách ngôn ngữ
export async function getAvailableLangs(): Promise<string[]> {
    try {
        return await invoke<string[]>('get_available_langs');
    } catch (error) {
        console.error("Lỗi lấy danh sách ngôn ngữ:", error);
        return ['vi']; // Fallback an toàn
    }
}

// Gọi API lấy nội dung ngôn ngữ
export async function getLangContent(langCode: string): Promise<Record<string, any>> {
    try {
        return await invoke<Record<string, any>>('get_lang_content', { langCode });
    } catch (error) {
        console.error(`Lỗi lấy dữ liệu ngôn ngữ ${langCode}:`, error);
        return {};
    }
}
