import { convertFileSrc } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { getAvailableFonts } from '../../../bridge/font_bridge';
import type { FontInfo } from '../../../bridge/types';
import { useSettingsStore } from './useSettingsStore';

interface FontState {
    fonts: FontInfo[];
    activeFont: FontInfo | null;
    isLoading: boolean;
    initFonts: () => Promise<void>;
    setActiveFont: (fontId: string) => void;
}

function cssString(value: string): string {
    return [...value]
        .filter(char => char.charCodeAt(0) >= 32 && !`'"\\<>{};`.includes(char))
        .join('');
}

export const useFontStore = create<FontState>((set, get) => ({
    fonts: [],
    activeFont: null,
    isLoading: true,

    initFonts: async () => {
        set({ isLoading: true });
        try {
            const fonts = await getAvailableFonts();
            set({ fonts });
            get().setActiveFont(useSettingsStore.getState().font_id);
        } finally {
            set({ isLoading: false });
        }
    },

    setActiveFont: (fontId) => {
        const font = get().fonts.find(item => item.id === fontId) ?? get().fonts[0] ?? null;
        if (!font) return;

        document.getElementById('dynamic-font-style')?.remove();
        const style = document.createElement('style');
        style.id = 'dynamic-font-style';
        const family = cssString(font.family);
        style.textContent = font.src_path
            ? `@font-face { font-family: '${family}'; src: url('${convertFileSrc(font.src_path)}'); font-display: swap; } :root { --font-family-base: '${family}', sans-serif; }`
            : ':root { --font-family-base: system-ui, -apple-system, sans-serif; }';
        document.head.appendChild(style);
        set({ activeFont: font });
    },
}));
