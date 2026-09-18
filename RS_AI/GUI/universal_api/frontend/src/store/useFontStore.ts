import { create } from 'zustand';

interface FontState {
    fonts: Array<{ id: string; name: string; family: string; src_path: string | null }>;
    currentFont: string;
    isLoading: boolean;
    initFonts: () => Promise<void>;
}

export const useFontStore = create<FontState>((set) => ({
    fonts: [],
    currentFont: 'default',
    isLoading: true,
    initFonts: async () => {
        try {
            const { invokeIpc } = await import('../../../bridge/ipc');
            const fonts = await invokeIpc<Array<{ id: string; name: string; family: string; src_path: string | null }>>('get_available_fonts');
            set({ fonts, isLoading: false });
        } catch {
            set({ isLoading: false });
        }
    },
}));
