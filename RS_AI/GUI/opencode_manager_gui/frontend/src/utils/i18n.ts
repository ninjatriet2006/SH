/*
[INTEGRITY NOTES]
- Mục đích: Cơ chế i18n theo ID Linking + các hàm format dùng chung.
- Trách nhiệm: `t('nhóm.khoá')` tra từ điển đã nạp; thiếu khoá thì trả về CHÍNH
  KHOÁ để lỗi dịch hiện rõ trên UI thay vì render chuỗi rỗng (ô trống rất khó
  phát hiện — bài học từ subscription_manager).
- Tương tác: `store/useSettingsStore.ts`.
*/

import { useSettingsStore } from '../store/useSettingsStore';

export function useTranslation() {
    const dictionary = useSettingsStore(state => state.dictionary);

    const t = (key: string): string => {
        if (!dictionary || Object.keys(dictionary).length === 0) return key;

        let current: unknown = dictionary;
        for (const seg of key.split('.')) {
            if (typeof current !== 'object' || current === null) return key;
            current = (current as Record<string, unknown>)[seg];
            if (current === undefined) return key;
        }
        return typeof current === 'string' ? current : key;
    };

    return { t };
}

/**
 * Nhãn ngôn ngữ trong dropdown. Dùng `Intl.DisplayNames` để mọi mã thả vào
 * `langs/` đều có tên đọc được, thay vì bảng if/else chỉ biết vi/en.
 */
export function langLabel(code: string): string {
    try {
        const name = new Intl.DisplayNames([code], { type: 'language' }).of(code);
        if (name && name.toLowerCase() !== code.toLowerCase()) {
            return `${name} (${code})`;
        }
    } catch { /* mã không chuẩn: hiện nguyên mã */ }
    return code;
}
