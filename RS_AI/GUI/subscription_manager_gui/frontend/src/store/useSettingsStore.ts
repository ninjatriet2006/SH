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

export const useSettingsStore = create<SettingsState>((set) => ({
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
            let lang = settings.language;
            let dict = await getLangContent(lang);

            // Tự phục hồi: settings.json có thể lưu mã ngôn ngữ không còn tồn
            // tại (vd "5555" từ file test đã xóa) → từ điển rỗng làm cả UI
            // hiện raw key. Rớt về 'vi' và chữa luôn file settings.
            if (Object.keys(dict).length === 0) {
                lang = 'vi';
                dict = await getLangContent('vi');
                try { await saveSettings('vi', settings.timezone, settings.theme_id || 'default', settings.font_id || 'default'); } catch { /* giữ state, bỏ qua lỗi chữa file */ }
            }

            set({
                language: lang,
                timezone: settings.timezone,
                theme_id: settings.theme_id || 'default',
                font_id: settings.font_id || 'default',
                availableLangs: langs.length > 0 ? langs : ['vi'],
                dictionary: dict
            });
        } catch (error) {
            console.error("Lỗi khởi tạo settings:", error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    updateSettings: async (lang: string, tz: string, themeId: string, fontId: string) => {
        set({ isLoading: true });
        try {
            // Nạp từ điển TRƯỚC khi lưu: file ngôn ngữ mất thì báo lỗi ngay
            // thay vì lưu settings rồi để UI hiện raw key khắp nơi.
            const dict = await getLangContent(lang);
            if (Object.keys(dict).length === 0) {
                throw new Error(`Không tải được ngôn ngữ "${lang}" (thiếu file langs/${lang}.json)`);
            }

            // Lưu xuống backend
            await saveSettings(lang, tz, themeId, fontId);

            set({ language: lang, timezone: tz, theme_id: themeId, font_id: fontId, dictionary: dict });
        } catch (error) {
            console.error("Lỗi cập nhật settings:", error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    }
}));
