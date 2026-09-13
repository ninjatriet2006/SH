/*
[INTEGRITY NOTES]
 - Mục đích: Bridge cho cài đặt GUI, ngôn ngữ, theme và font.
- Trách nhiệm: Trả về giá trị "rỗng/an toàn" khi lỗi ĐỌC (để store tự chọn
  phương án dự phòng), nhưng NÉM lỗi khi GHI (người dùng phải biết lưu thất bại).
  Không đoán mã ngôn ngữ cụ thể ở đây — đoán sai sẽ che mất lỗi thật.
- Tương tác: `store/useSettingsStore.ts`, `store/useThemeStore.ts`.
*/

import { invokeIpc, ipcErrorMessage } from './ipc';
import type { ConfigPaths, GuiSettings, Theme } from './types';

export async function getGuiSettings(): Promise<GuiSettings> {
    try {
        return await invokeIpc<GuiSettings>('get_gui_settings');
    } catch (error) {
        console.error('Lỗi lấy cài đặt:', error);
        // `language` rỗng = "để store tự chọn theo file có thật trong langs/".
        return { language: '', theme_id: 'default', font_id: 'default' };
    }
}

/** Đường dẫn file cấu hình opencode.json / auth.json (hiện ở trang Cài đặt). */
export async function getConfigPaths(): Promise<ConfigPaths> {
    try {
        return await invokeIpc<ConfigPaths>('get_config_paths');
    } catch (error) {
        console.error('Lỗi lấy đường dẫn cấu hình:', error);
        throw new Error(ipcErrorMessage(error));
    }
}

/** Mở URL https trong trình duyệt ngoài (QR nạp tiền CKey...). */
export async function openExternalUrl(url: string): Promise<void> {
    try {
        await invokeIpc<void, { url: string }>('open_external_url', { url });
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

export async function saveGuiSettings(language: string, themeId: string, fontId: string): Promise<void> {
    try {
        await invokeIpc<void, { language: string; theme_id: string; font_id: string }>(
            'save_gui_settings', { language, theme_id: themeId, font_id: fontId },
        );
    } catch (error) {
        throw new Error(ipcErrorMessage(error));
    }
}

export async function getAvailableLangs(): Promise<string[]> {
    try {
        return await invokeIpc<string[]>('get_available_langs');
    } catch (error) {
        console.error('Lỗi lấy danh sách ngôn ngữ:', error);
        return [];
    }
}

export async function getLangContent(langCode: string): Promise<Record<string, unknown>> {
    try {
        return await invokeIpc<Record<string, unknown>, { lang_code: string }>('get_lang_content', { lang_code: langCode });
    } catch (error) {
        console.error(`Lỗi lấy dữ liệu ngôn ngữ ${langCode}:`, error);
        return {};
    }
}

export async function getAvailableThemes(): Promise<Theme[]> {
    try {
        return await invokeIpc<Theme[]>('get_available_themes');
    } catch (error) {
        console.error('Lỗi lấy danh sách theme:', error);
        return [];
    }
}
