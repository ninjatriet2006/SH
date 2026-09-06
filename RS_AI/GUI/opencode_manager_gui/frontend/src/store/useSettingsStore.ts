/*
[INTEGRITY NOTES]
- Mục đích: Trạng thái cài đặt toàn cục (ngôn ngữ, theme, từ điển UI).
- Trách nhiệm: Nạp cài đặt từ backend, chọn ngôn ngữ theo file THỰC CÓ trong
  `langs/`. Không hardcode 'vi': app phải chạy với bộ ngôn ngữ bất kỳ, và khi
  không nạp được file nào thì để từ điển rỗng để `t()` trả raw key (lộ lỗi).
- Tương tác: `utils/i18n.ts`, `pages/SettingsPage.tsx`.
*/

import { create } from 'zustand';
import {
    getGuiSettings, saveGuiSettings, getAvailableLangs, getLangContent, getConfigPaths,
} from '../../../bridge/settings_bridge';
import type { ConfigPaths } from '../../../bridge/types';

/** `default` là quy ước "dùng màu gốc trong CSS", KHÔNG phải tên file. */
const DEFAULT_THEME_ID = 'default';

interface SettingsState {
    language: string;
    theme_id: string;
    availableLangs: string[];
    dictionary: Record<string, unknown>;
    /** Đường dẫn file cấu hình (hiện ở trang Cài đặt). */
    paths: ConfigPaths | null;
    isLoading: boolean;

    initSettings: () => Promise<void>;
    updateSettings: (lang: string, themeId: string) => Promise<void>;
    fetchPaths: () => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
    language: '',
    theme_id: DEFAULT_THEME_ID,
    availableLangs: [],
    dictionary: {},
    paths: null,
    isLoading: true,

    initSettings: async () => {
        set({ isLoading: true });
        try {
            const langs = await getAvailableLangs();
            const settings = await getGuiSettings();

            // Thứ tự thử: ngôn ngữ đã lưu → các file thực có.
            const candidates = [settings.language, ...langs].filter(Boolean);
            let lang = '';
            let dict: Record<string, unknown> = {};
            const tried = new Set<string>();
            for (const code of candidates) {
                if (tried.has(code)) continue;
                tried.add(code);
                const loaded = await getLangContent(code);
                if (Object.keys(loaded).length > 0) {
                    lang = code;
                    dict = loaded;
                    break;
                }
            }

            // Chữa lại settings nếu ngôn ngữ đang lưu không dùng được.
            if (lang && lang !== settings.language) {
                try {
                    await saveGuiSettings(lang, settings.theme_id || DEFAULT_THEME_ID);
                } catch { /* giữ state trong bộ nhớ, bỏ qua lỗi ghi file */ }
            }

            if (!lang) {
                console.error(
                    `Không nạp được ngôn ngữ nào (langs/ rỗng hoặc file hỏng). ` +
                    `Đã thử: ${[...tried].join(', ') || '(không có)'}`
                );
            }

            set({
                language: lang,
                theme_id: settings.theme_id || DEFAULT_THEME_ID,
                availableLangs: langs,
                dictionary: dict,
            });
        } catch (error) {
            console.error('Lỗi khởi tạo cài đặt:', error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    updateSettings: async (lang, themeId) => {
        set({ isLoading: true });
        try {
            // Nạp từ điển TRƯỚC khi lưu: file mất thì báo lỗi ngay thay vì lưu
            // rồi để UI hiện raw key khắp nơi.
            const dict = await getLangContent(lang);
            if (Object.keys(dict).length === 0) {
                throw new Error(`Không tải được ngôn ngữ "${lang}" (thiếu file langs/${lang}.json)`);
            }
            await saveGuiSettings(lang, themeId);
            set({ language: lang, theme_id: themeId, dictionary: dict });
        } catch (error) {
            console.error('Lỗi cập nhật cài đặt:', error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    fetchPaths: async () => {
        try {
            set({ paths: await getConfigPaths() });
        } catch (error) {
            console.error('Lỗi lấy đường dẫn cấu hình:', error);
        }
    },
}));
