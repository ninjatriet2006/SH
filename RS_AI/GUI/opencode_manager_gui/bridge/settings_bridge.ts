/*
[INTEGRITY NOTES]
- Mục đích: Bridge cho cài đặt GUI, ngôn ngữ và theme.
- Trách nhiệm: Trả về giá trị "rỗng/an toàn" khi lỗi ĐỌC (để store tự chọn
  phương án dự phòng), nhưng NÉM lỗi khi GHI (người dùng phải biết lưu thất bại).
  Không đoán mã ngôn ngữ cụ thể ở đây — đoán sai sẽ che mất lỗi thật.
- Tương tác: `store/useSettingsStore.ts`, `store/useThemeStore.ts`.
*/

import { invoke } from '@tauri-apps/api/core';
import type { ConfigPaths, GuiSettings, Theme } from './types';

export async function getGuiSettings(): Promise<GuiSettings> {
    try {
        return await invoke<GuiSettings>('get_gui_settings');
    } catch (error) {
        console.error('Lỗi lấy cài đặt:', error);
        // `language` rỗng = "để store tự chọn theo file có thật trong langs/".
        return { language: '', theme_id: 'default' };
    }
}

/** Đường dẫn file cấu hình opencode.json / auth.json (hiện ở trang Cài đặt). */
export async function getConfigPaths(): Promise<ConfigPaths> {
    try {
        return await invoke<ConfigPaths>('get_config_paths');
    } catch (error) {
        console.error('Lỗi lấy đường dẫn cấu hình:', error);
        throw new Error(String(error));
    }
}

export async function saveGuiSettings(language: string, themeId: string): Promise<void> {
    try {
        await invoke('save_gui_settings', { language, theme_id: themeId });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function getAvailableLangs(): Promise<string[]> {
    try {
        return await invoke<string[]>('get_available_langs');
    } catch (error) {
        console.error('Lỗi lấy danh sách ngôn ngữ:', error);
        return [];
    }
}

export async function getLangContent(langCode: string): Promise<Record<string, unknown>> {
    try {
        return await invoke<Record<string, unknown>>('get_lang_content', { lang_code: langCode });
    } catch (error) {
        console.error(`Lỗi lấy dữ liệu ngôn ngữ ${langCode}:`, error);
        return {};
    }
}

export async function getAvailableThemes(): Promise<Theme[]> {
    try {
        return await invoke<Theme[]>('get_available_themes');
    } catch (error) {
        console.error('Lỗi lấy danh sách theme:', error);
        return [];
    }
}
