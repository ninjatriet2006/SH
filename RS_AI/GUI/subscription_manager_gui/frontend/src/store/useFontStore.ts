/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái Font (Tùy biến Typography) và inject cấu hình Font vào DOM.
- Trách nhiệm: Nạp danh sách Font từ Backend, phân biệt Local Font & Web Font, và tự động tạo tag <style> chứa @import hoặc @font-face.
- Tương tác: Dùng bởi `SettingsPage.tsx` và gọi API `font_bridge.ts`.
*/

import { create } from 'zustand';
import type { FontInfo } from '../../../bridge/types';
import { getAvailableFonts } from '../../../bridge/font_bridge';
import { useSettingsStore } from './useSettingsStore';
import { convertFileSrc } from '@tauri-apps/api/core';

interface FontState {
    fonts: FontInfo[];
    activeFont: FontInfo | null;
    isLoading: boolean;
    
    initFonts: () => Promise<void>;
    setActiveFont: (fontId: string) => void;
}

export const useFontStore = create<FontState>((set, get) => ({
    fonts: [],
    activeFont: null,
    isLoading: true,
    
    initFonts: async () => {
        set({ isLoading: true });
        try {
            const fontsList = await getAvailableFonts();
            set({ fonts: fontsList });
            
            // Lấy font_id từ settings store
            const currentFontId = useSettingsStore.getState().font_id;
            get().setActiveFont(currentFontId);
            
        } catch (error) {
            console.error("Lỗi khởi tạo fonts:", error);
        } finally {
            set({ isLoading: false });
        }
    },
    
    setActiveFont: (fontId: string) => {
        const fontToSet = get().fonts.find(f => f.id === fontId) || get().fonts[0] || null;
        
        if (fontToSet) {
            set({ activeFont: fontToSet });
            
            // Inject Font Style vào DOM
            const existingStyle = document.getElementById('dynamic-font-style');
            if (existingStyle) {
                existingStyle.remove();
            }
            
            const styleEl = document.createElement('style');
            styleEl.id = 'dynamic-font-style';
            
            // Sanitize font-family để tránh CSS injection qua tên font độc hại
            const sanitizeCss = (str: string) => str.replace(/['"\\<>{}]/g, '');
            const safeFamily = sanitizeCss(fontToSet.family);
            
            if (!fontToSet.is_local && fontToSet.src_url) {
                // Web Font: Dùng @import url - sanitize URL
                const safeUrl = fontToSet.src_url.replace(/['"\\<>]/g, '');
                styleEl.textContent = `
                    @import url('${safeUrl}');
                    :root {
                        --font-family-base: '${safeFamily}', sans-serif;
                    }
                `;
            } else if (fontToSet.is_local && fontToSet.src_url) {
                // Local Font: Dùng @font-face và convertFileSrc để Tauri cho phép đọc file
                const assetUrl = convertFileSrc(fontToSet.src_url);
                styleEl.textContent = `
                    @font-face {
                        font-family: '${safeFamily}';
                        src: url('${assetUrl}');
                    }
                    :root {
                        --font-family-base: '${safeFamily}', sans-serif;
                    }
                `;
            } else {
                // Font rỗng/mặc định hệ thống
                styleEl.textContent = `
                    :root {
                        --font-family-base: system-ui, -apple-system, sans-serif;
                    }
                `;
            }
            
            document.head.appendChild(styleEl);
        }
    }
}));
