/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái giao diện (Theme, Font, Ngôn ngữ i18n).
- Trách nhiệm: Nạp dữ liệu từ Backend, áp dụng CSS variables vào `:root` và nạp từ điển ngôn ngữ.
- Tương tác: Dùng `appearance_bridge.ts` và được gọi bởi `useTranslation()`.
*/

import { convertFileSrc } from '@tauri-apps/api/core';
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
  availableLangs: ['vi', 'en', 'test5555'],
  translations: {},
  isLoading: true,

  initAppearance: async () => {
    set({ isLoading: true });
    try {
      const [appConfig, themesList, fontsList, langsList] = await Promise.all([
        getAppearance().catch(() => defaultAppearance),
        getAvailableThemes().catch(() => []),
        getAvailableFonts().catch(() => []),
        getAvailableLangs().catch(() => ['vi', 'en', 'test5555']),
      ]);

      set({
        appearance: appConfig,
        themes: themesList,
        fonts: fontsList,
        availableLangs: langsList.length ? langsList : ['vi', 'en', 'test5555'],
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

    // Đảm bảo tương thích tức thì cho các biến gốc của UI
    if (theme.variables['colors-neon-cyan']) {
      root.style.setProperty('--primary', theme.variables['colors-neon-cyan']);
    }
    if (theme.variables['colors-surface-canvas']) {
      root.style.setProperty('--bg-dark', theme.variables['colors-surface-canvas']);
    }
    if (theme.variables['colors-surface-card']) {
      root.style.setProperty('--bg-card', theme.variables['colors-surface-card']);
    }
    if (theme.variables['colors-surface-header']) {
      root.style.setProperty('--bg-panel', theme.variables['colors-surface-header']);
    }
    if (theme.variables['colors-surface-input']) {
      root.style.setProperty('--bg-input', theme.variables['colors-surface-input']);
    }
    if (theme.variables['colors-text-primary']) {
      root.style.setProperty('--text-primary', theme.variables['colors-text-primary']);
    }
    if (theme.variables['colors-text-secondary']) {
      root.style.setProperty('--text-secondary', theme.variables['colors-text-secondary']);
    }
    if (theme.variables['colors-border-muted']) {
      root.style.setProperty('--border', theme.variables['colors-border-muted']);
    }

    set((state) => ({
      appearance: { ...state.appearance, theme: themeId },
    }));

    await setAppearance({ ...get().appearance, theme: themeId }).catch(console.error);
  },

  changeFont: async (fontId: string) => {
    const font = get().fonts.find((f) => f.id === fontId);
    const root = document.documentElement;

    let styleEl = document.getElementById('rclone-custom-font-style') as HTMLStyleElement | null;
    if (!styleEl) {
      styleEl = document.createElement('style');
      styleEl.id = 'rclone-custom-font-style';
      document.head.appendChild(styleEl);
    }

    const sanitizeCss = (str: string) => str.replace(/['"\\<>{}]/g, '');

    if (font && font.src_path && fontId !== 'system' && fontId !== 'default') {
      const safeFamily = sanitizeCss(font.family || font.name);
      let assetUrl = font.src_path;
      try {
        assetUrl = convertFileSrc(font.src_path);
      } catch {
        assetUrl = `asset://${font.src_path}`;
      }

      styleEl.textContent = `
        @font-face {
          font-family: '${safeFamily}';
          src: url('${assetUrl}');
          font-display: swap;
        }
        :root {
          --font-family-base: '${safeFamily}', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        }
      `;
      root.style.setProperty(
        '--font-family-base',
        `'${safeFamily}', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`,
      );
    } else {
      styleEl.textContent = `
        :root {
          --font-family-base: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        }
      `;
      root.style.setProperty(
        '--font-family-base',
        "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
      );
    }

    set((state) => ({
      appearance: { ...state.appearance, font: fontId },
    }));

    await setAppearance({ ...get().appearance, font: fontId }).catch(console.error);
  },
}));
