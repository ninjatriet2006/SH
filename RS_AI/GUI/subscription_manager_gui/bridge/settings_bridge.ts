import { invokeCommand, type Empty } from './types';

export interface Settings {
    language: string;
    timezone: string;
    theme_id: string;
    font_id: string;
}

// Gọi API lấy cài đặt
export async function getSettings(): Promise<Settings> {
    try {
        return await invokeCommand<Settings, Empty>('get_settings', {});
    } catch (error) {
        console.error("Lỗi lấy cài đặt:", error);
        // `language` rỗng = "để store tự chọn theo file có thật trong langs/".
        // Không đoán 'vi' ở đây vì bộ ngôn ngữ có thể không chứa vi.
        return { language: '', timezone: 'Asia/Ho_Chi_Minh', theme_id: 'default', font_id: 'default' };
    }
}

// Gọi API lưu cài đặt
export async function saveSettings(language: string, timezone: string, theme_id: string, font_id: string): Promise<void> {
    await invokeCommand<void, { language: string; timezone: string; theme_id: string; font_id: string }>('save_settings', { language, timezone, theme_id, font_id });
}

export async function exportBackup(destination: string): Promise<void> {
    await invokeCommand<void, { destination: string }>('export_backup', { destination });
}

// Gọi API lấy danh sách ngôn ngữ
export async function getAvailableLangs(): Promise<string[]> {
    try {
        return await invokeCommand<string[], Empty>('get_available_langs', {});
    } catch (error) {
        console.error("Lỗi lấy danh sách ngôn ngữ:", error);
        // Trả rỗng thay vì bịa ['vi']: danh sách này quyết định dropdown và
        // ngôn ngữ fallback, đoán sai sẽ che mất lỗi thật.
        return [];
    }
}

// Gọi API lấy nội dung ngôn ngữ
export async function getLangContent(langCode: string): Promise<Record<string, any>> {
    try {
        return await invokeCommand<Record<string, any>, { lang_code: string }>('get_lang_content', { lang_code: langCode });
    } catch (error) {
        console.error(`Lỗi lấy dữ liệu ngôn ngữ ${langCode}:`, error);
        return {};
    }
}
