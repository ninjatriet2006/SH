import { create } from 'zustand';
import { invokeIpc } from '../../../bridge/ipc';
import type { ThemeInfo } from '../../../bridge/types';

interface ThemeState {
    themes: ThemeInfo[];
    currentTheme: string;
    isLoading: boolean;
    initThemes: () => Promise<void>;
    applyTheme: (id: string) => void;
}

export const useThemeStore = create<ThemeState>((set, get) => ({
    themes: [],
    currentTheme: 'default',
    isLoading: true,
    initThemes: async () => {
        try {
            const themes = await invokeIpc<ThemeInfo[]>('get_available_themes');
            set({ themes, isLoading: false });
            if (themes.length > 0) {
                get().applyTheme(themes[0].id);
            }
        } catch {
            set({ isLoading: false });
        }
    },
    applyTheme: (id) => {
        const theme = useThemeStore.getState().themes.find((item) => item.id === id);
        if (!theme) return;
        for (const [key, value] of Object.entries(theme.colors)) {
            document.documentElement.style.setProperty(`--${key.replace(/_/g, '-')}`, value);
        }
        set({ currentTheme: id });
    },
}));
