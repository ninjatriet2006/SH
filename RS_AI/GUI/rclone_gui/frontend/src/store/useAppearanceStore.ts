/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái giao diện (Theme, Font, Ngôn ngữ i18n).
- Trách nhiệm: Nạp dữ liệu từ Backend, áp dụng CSS variables vào `:root` và nạp từ điển ngôn ngữ.
- Tương tác: Dùng `appearance_bridge.ts` và được gọi bởi `useTranslation()`.
*/

import { create } from 'zustand';
import {
  getAppearance,
  getAvailableFonts,
  getAvailableLangs,
  getAvailableThemes,
  getLangContent,
  setAppearance,
} from '../../../bridge/appearance_bridge';
import type { AppearanceSettings, FontInfo, ThemeInfo } from '../../../bridge/types';

interface AppearanceState {
  appearance: AppearanceSettings;
  themes: ThemeInfo[];
  fonts: FontInfo[];
  availableLangs: string[];
  translations: Record<string, string>;
  isLoading: boolean;

  initAppearance: () => Promise<void>;
  changeLanguage: (lang: string) => Promise<void>;
  changeTheme: (themeId: string) => Promise<void>;
  changeFont: (fontId: string) => Promise<void>;
}

const defaultAppearance: AppearanceSettings = {
  lang: 'vi',
  theme: 'deep-space',
  font: 'system',
  langs_dir: '',
  themes_dir: '',
  fonts_dir: '',
};

export const useAppearanceStore = create<AppearanceState>((set, get) => ({
  appearance: defaultAppearance,
  themes: [],
  fonts: [],
  availableLangs: ['vi', 'en'],
  translations: {},
  isLoading: true,

  initAppearance: async () => {
    set({ isLoading: true });
    try {
      const [appConfig, themesList, fontsList, langsList] = await Promise.all([
        getAppearance().catch(() => defaultAppearance),
        getAvailableThemes().catch(() => []),
        getAvailableFonts().catch(() => []),
        getAvailableLangs().catch(() => ['vi', 'en']),
      ]);

      set({
        appearance: appConfig,
        themes: themesList,
        fonts: fontsList,
        availableLangs: langsList.length ? langsList : ['vi', 'en'],
      });

      // Áp dụng theme và font
      if (themesList.length > 0) {
        get().changeTheme(appConfig.theme);
      }
      if (fontsList.length > 0) {
        get().changeFont(appConfig.font);
      }

      // Tải từ điển ngôn ngữ
      await get().changeLanguage(appConfig.lang || 'vi');
    } catch (err) {
      console.error('Lỗi khởi tạo Appearance:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  changeLanguage: async (lang: string) => {
    try {
      const dict = await getLangContent(lang);
      const flatDict: Record<string, string> = {};
      const flatten = (obj: Record<string, unknown>, prefix = '') => {
        for (const [k, v] of Object.entries(obj)) {
          const key = prefix ? `${prefix}.${k}` : k;
          if (v && typeof v === 'object' && !Array.isArray(v)) {
            flatten(v as Record<string, unknown>, key);
          } else {
            flatDict[key] = String(v ?? '');
          }
        }
      };
      flatten(dict as Record<string, unknown>);

      set((state) => ({
        appearance: { ...state.appearance, lang },
        translations: flatDict,
      }));

      await setAppearance({ ...get().appearance, lang });
    } catch (err) {
      console.error(`Lỗi tải ngôn ngữ ${lang}:`, err);
    }
  },

  changeTheme: async (themeId: string) => {
    const theme = get().themes.find((t) => t.id === themeId);
    if (!theme) return;

    const root = document.documentElement;
    // Áp dụng variables vào :root
    for (const [key, val] of Object.entries(theme.variables)) {
      const cssVar = key.startsWith('--') ? key : `--${key.replace(/_/g, '-')}`;
      root.style.setProperty(cssVar, val);
    }

    set((state) => ({
      appearance: { ...state.appearance, theme: themeId },
    }));

    await setAppearance({ ...get().appearance, theme: themeId }).catch(console.error);
  },

  changeFont: async (fontId: string) => {
    const font = get().fonts.find((f) => f.id === fontId);
    const root = document.documentElement;
    const fontFamily = font?.family || 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    root.style.setProperty('--font-family-base', fontFamily);

    set((state) => ({
      appearance: { ...state.appearance, font: fontId },
    }));

    await setAppearance({ ...get().appearance, font: fontId }).catch(console.error);
  },
}));
