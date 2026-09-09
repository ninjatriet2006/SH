/*
[INTEGRITY NOTES]
- Mục đích: Trạng thái Theme và inject CSS variables vào `:root`.
- Trách nhiệm: Nạp theme từ backend, áp màu của theme đang chọn. Không có theme
  nào thì giữ nguyên màu mặc định trong `index.css` (app vẫn dùng được).
- Tương tác: `pages/SettingsPage.tsx`, bridge `settings_bridge.ts`.
*/

import { create } from 'zustand';
import type { Theme } from '../../../bridge/types';
import { getAvailableThemes } from '../../../bridge/settings_bridge';
import { useSettingsStore } from './useSettingsStore';

interface ThemeState {
    themes: Theme[];
    activeTheme: Theme | null;
    isLoading: boolean;

    initThemes: () => Promise<void>;
    setActiveTheme: (themeId: string) => void;
}

export const useThemeStore = create<ThemeState>((set, get) => ({
    themes: [],
    activeTheme: null,
    isLoading: true,

    initThemes: async () => {
        set({ isLoading: true });
        try {
            const themes = await getAvailableThemes();
            set({ themes });
            get().setActiveTheme(useSettingsStore.getState().theme_id);
        } catch (error) {
            console.error('Lỗi khởi tạo theme:', error);
        } finally {
            set({ isLoading: false });
        }
    },

    setActiveTheme: (themeId) => {
        // Không tìm thấy id đã lưu → dùng theme đầu tiên; không có theme nào thì
        // thôi, CSS gốc đã có đủ biến màu.
        const theme = get().themes.find(t => t.id === themeId) ?? get().themes[0] ?? null;
        if (!theme) return;

        const previous = get().activeTheme;
        set({ activeTheme: theme });
        const root = document.documentElement;
        if (previous) {
            for (const key of Object.keys(previous.colors)) {
                root.style.removeProperty(`--${key.replace(/_/g, '-')}`);
            }
        }
        for (const [key, value] of Object.entries(theme.colors)) {
            // bg_panel → --bg-panel
            root.style.setProperty(`--${key.replace(/_/g, '-')}`, value);
        }
    },
}));
