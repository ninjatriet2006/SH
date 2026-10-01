import { create } from "zustand";
import type { FontId, Language, Preferences, Theme } from "../utils/contract";
import { api, ipcError } from "../utils/ipc";
import { loadAndApplyTheme, loadMessages, type Messages } from "../utils/resources";

const defaultPreferences: Preferences = { language: "vi", theme: "system", font_id: "system-default" };

interface SettingsState {
  preferences: Preferences;
  messages: Messages;
  isLoading: boolean;

  initSettings: () => Promise<void>;
  updateSettings: (next: Preferences) => Promise<void>;
  previewTheme: (theme: Theme, fontId?: FontId) => Promise<void>;
  previewLanguage: (language: Language) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  preferences: defaultPreferences,
  messages: {},
  isLoading: true,

  initSettings: async () => {
    set({ isLoading: true });
    try {
      let prefs: Preferences;
      try {
        prefs = await api.getPreferences();
      } catch {
        prefs = defaultPreferences;
      }
      const messages = await loadMessages(prefs.language);
      await loadAndApplyTheme(prefs.theme, prefs.font_id);
      document.documentElement.lang = prefs.language;
      set({ preferences: prefs, messages });
    } catch (error) {
      console.error("Settings init failed:", ipcError(error).message);
    } finally {
      set({ isLoading: false });
    }
  },

  previewTheme: async (theme: Theme, fontId?: FontId) => {
    const current = get().preferences;
    const font = fontId ?? current.font_id;
    await loadAndApplyTheme(theme, font);
    set({ preferences: { ...current, theme, font_id: font } });
  },

  previewLanguage: async (language: Language) => {
    const current = get().preferences;
    const messages = await loadMessages(language);
    document.documentElement.lang = language;
    set({ preferences: { ...current, language }, messages });
  },

  updateSettings: async (next: Preferences) => {
    try {
      const saved = await api.setPreferences(next);
      const messages = await loadMessages(saved.language as Language);
      await loadAndApplyTheme(saved.theme, saved.font_id);
      document.documentElement.lang = saved.language;
      set({ preferences: saved, messages });
    } catch (error) {
      console.error("Settings update failed:", ipcError(error).message);
      throw error;
    }
  },
}));
