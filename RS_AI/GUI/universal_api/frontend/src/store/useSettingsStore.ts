import { create } from 'zustand';
import { getGuiSettings, saveGuiSettings } from '../../../bridge/settings_bridge';
import { invokeIpc } from '../../../bridge/ipc';
import type { GuiSettings } from '../../../bridge/types';

interface SettingsState {
    language: string;
    dictionary: Record<string, unknown>;
    settings: GuiSettings;
    isLoading: boolean;
    initSettings: () => Promise<void>;
    setLanguage: (lang: string) => Promise<void>;
    updateSettings: (settings: GuiSettings) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
    language: 'en',
    dictionary: {},
    settings: { language: 'en', theme: 'default', font: 'default' },
    isLoading: true,
    initSettings: async () => {
        try {
            const settings = await getGuiSettings();
            const dict = await invokeIpc<Record<string, unknown>>('get_lang_content', { lang_code: settings.language });
            set({ language: settings.language, settings, dictionary: dict, isLoading: false });
        } catch {
            set({ isLoading: false });
        }
    },
    setLanguage: async (lang: string) => {
        try {
            const dict = await invokeIpc<Record<string, unknown>>('get_lang_content', { lang_code: lang });
            const settings = { ...useSettingsStore.getState().settings, language: lang };
            await saveGuiSettings(settings);
            set({ language: lang, settings, dictionary: dict });
        } catch (e) { console.error('setLanguage failed:', e); }
    },
    updateSettings: async (settings) => {
        await saveGuiSettings(settings);
        const dict = await invokeIpc<Record<string, unknown>>('get_lang_content', { lang_code: settings.language });
        set({ settings, language: settings.language, dictionary: dict });
    },
}));
