/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái Theme (Tùy biến giao diện) và inject CSS variables vào DOM.
- Trách nhiệm: Nạp danh sách Themes từ Backend, quản lý theme hiện tại, và tự động set CSS property vào `:root`.
- Tương tác: Dùng bởi `SettingsPage.tsx` và gọi API `theme_bridge.ts`.
*/

import { create } from 'zustand';
import type { Theme } from '../../../bridge/types';
import { getAvailableThemes } from '../../../bridge/theme_bridge';
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
            const themesList = await getAvailableThemes();
            set({ themes: themesList });
            
            // Lấy theme_id từ settings store
            const currentThemeId = useSettingsStore.getState().theme_id;
            get().setActiveTheme(currentThemeId);
            
        } catch (error) {
            console.error("Lỗi khởi tạo themes:", error);
        } finally {
            set({ isLoading: false });
        }
    },
    
    setActiveTheme: (themeId: string) => {
        const themeToSet = get().themes.find(t => t.id === themeId) || get().themes[0] || null;
        
        if (themeToSet) {
            set({ activeTheme: themeToSet });
            // Inject CSS variables vào DOM (:root)
            const root = document.documentElement;
            Object.entries(themeToSet.colors).forEach(([key, value]) => {
                // Đảm bảo key có dạng css, ví dụ bg_primary -> --bg-primary
                const cssVar = `--${key.replace(/_/g, '-')}`;
                root.style.setProperty(cssVar, value);
            });
        }
    }
}));
