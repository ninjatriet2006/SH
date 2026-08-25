/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái cài đặt toàn cục (Ngôn ngữ, Múi giờ, Từ điển UI).
- Trách nhiệm: Nạp cài đặt từ Backend, giữ state language, load từ điển vào bộ nhớ.
- Tương tác: Dùng bởi `utils/i18n.ts` và `SettingsPage.tsx`.
*/

import { create } from 'zustand';
import { getSettings, getLangContent, saveSettings, getAvailableLangs } from '../../../bridge/settings_bridge';

interface SettingsState {
    language: string;
    timezone: string;
    theme_id: string;
    font_id: string;
    availableLangs: string[];
    dictionary: Record<string, any>;
    isLoading: boolean;
    
    // Nạp cài đặt và từ điển ban đầu
    initSettings: () => Promise<void>;
    // Cập nhật cài đặt mới
    updateSettings: (lang: string, tz: string, themeId: string, fontId: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
    language: 'vi',
    timezone: 'Asia/Ho_Chi_Minh',
    theme_id: 'default',
    font_id: 'default',
    availableLangs: ['vi'],
    dictionary: {},
    isLoading: true,
    
    initSettings: async () => {
        set({ isLoading: true });
        try {
            // Lấy danh sách ngôn ngữ có sẵn
            const langs = await getAvailableLangs();
            
            // Lấy cài đặt hiện tại
            const settings = await getSettings();
            
            // Lấy nội dung từ điển của ngôn ngữ hiện tại
            const dict = await getLangContent(settings.language);
            
            set({ 
                language: settings.language, 
                timezone: settings.timezone,
                theme_id: settings.theme_id || 'default',
                font_id: settings.font_id || 'default',
                availableLangs: langs.length > 0 ? langs : ['vi'],
                dictionary: dict
            });
        } catch (error) {
            console.error("Lỗi khởi tạo settings:", error);
        } finally {
            set({ isLoading: false });
        }
    },
    
    updateSettings: async (lang: string, tz: string, themeId: string, fontId: string) => {
        set({ isLoading: true });
        try {
            // Lưu xuống backend
            await saveSettings(lang, tz, themeId, fontId);
            
            // Nạp lại từ điển nếu đổi ngôn ngữ
            let dict = get().dictionary;
            if (lang !== get().language) {
                dict = await getLangContent(lang);
            }
            
            set({ language: lang, timezone: tz, theme_id: themeId, font_id: fontId, dictionary: dict });
        } catch (error) {
            console.error("Lỗi cập nhật settings:", error);
        } finally {
            set({ isLoading: false });
        }
    }
}));
