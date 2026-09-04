/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái cài đặt toàn cục (Ngôn ngữ, Múi giờ, Từ điển UI).
- Trách nhiệm: Nạp cài đặt từ Backend, giữ state language, load từ điển vào bộ nhớ.
- Tương tác: Dùng bởi `utils/i18n.ts` và `SettingsPage.tsx`.
*/

import { create } from 'zustand';
import { getSettings, getLangContent, saveSettings, getAvailableLangs } from '../../../bridge/settings_bridge';

interface SettingsState {
    language: string;
    timezone: string;
    theme_id: string;
    font_id: string;
    availableLangs: string[];
    dictionary: Record<string, any>;
    isLoading: boolean;
    
    // Nạp cài đặt và từ điển ban đầu
    initSettings: () => Promise<void>;
    // Cập nhật cài đặt mới
    updateSettings: (lang: string, tz: string, themeId: string, fontId: string) => Promise<void>;
}

// `default` là quy ước "dùng giá trị gốc trong CSS", KHÔNG phải tên file nên
// không cần `themes/default.json` tồn tại.
const DEFAULT_STYLE_ID = 'default';

export const useSettingsStore = create<SettingsState>((set) => ({
    // Khởi tạo rỗng: ngôn ngữ do backend quyết theo file thực có trong `langs/`.
    // Trước đây hardcode 'vi' nên bộ ngôn ngữ không chứa vi sẽ hiện sai/ID.
    language: '',
    timezone: 'Asia/Ho_Chi_Minh',
    theme_id: DEFAULT_STYLE_ID,
    font_id: DEFAULT_STYLE_ID,
    availableLangs: [],
    dictionary: {},
    isLoading: true,

    initSettings: async () => {
        set({ isLoading: true });
        try {
            // Danh sách ngôn ngữ thực có (backend quét thư mục `langs/`).
            const langs = await getAvailableLangs();

            // Lấy cài đặt hiện tại
            const settings = await getSettings();

            // Thứ tự thử: ngôn ngữ đã lưu → file đầu tiên trong langs/.
            // Không hardcode 'vi': app phải chạy với bộ ngôn ngữ bất kỳ.
            const candidates = [settings.language, ...langs].filter(Boolean);
            let lang = '';
            let dict: Record<string, any> = {};
            for (const code of candidates) {
                const loaded = await getLangContent(code);
                if (Object.keys(loaded).length > 0) {
                    lang = code;
                    dict = loaded;
                    break;
                }
            }

            // Chữa lại settings nếu ngôn ngữ đang lưu không dùng được.
            if (lang && lang !== settings.language) {
                try {
                    await saveSettings(
                        lang,
                        settings.timezone,
                        settings.theme_id || DEFAULT_STYLE_ID,
                        settings.font_id || DEFAULT_STYLE_ID
                    );
                } catch { /* giữ state trong bộ nhớ, bỏ qua lỗi ghi file */ }
            }

            // Không có file ngôn ngữ nào dùng được → để `dictionary` rỗng, `t()`
            // trả về ID (vd "sidebar.dashboard") để lỗi hiện rõ thay vì im lặng.
            if (!lang) {
                console.error(
                    `Không tải được ngôn ngữ nào (thư mục langs/ rỗng hoặc file hỏng). ` +
                    `Đã thử: ${candidates.join(', ') || '(không có)'}`
                );
            }

            set({
                language: lang,
                timezone: settings.timezone,
                theme_id: settings.theme_id || DEFAULT_STYLE_ID,
                font_id: settings.font_id || DEFAULT_STYLE_ID,
                availableLangs: langs,
                dictionary: dict
            });
        } catch (error) {
            console.error("Lỗi khởi tạo settings:", error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    },

    updateSettings: async (lang: string, tz: string, themeId: string, fontId: string) => {
        set({ isLoading: true });
        try {
            // Nạp từ điển TRƯỚC khi lưu: file ngôn ngữ mất thì báo lỗi ngay
            // thay vì lưu settings rồi để UI hiện raw key khắp nơi.
            const dict = await getLangContent(lang);
            if (Object.keys(dict).length === 0) {
                throw new Error(`Không tải được ngôn ngữ "${lang}" (thiếu file langs/${lang}.json)`);
            }

            // Lưu xuống backend
            await saveSettings(lang, tz, themeId, fontId);

            set({ language: lang, timezone: tz, theme_id: themeId, font_id: fontId, dictionary: dict });
        } catch (error) {
            console.error("Lỗi cập nhật settings:", error);
            throw error;
        } finally {
            set({ isLoading: false });
        }
    }
}));
