/*
[INTEGRITY NOTES]
Mục đích: Khai báo API đa ngôn ngữ (i18n) giữa Frontend và Backend.
Trách nhiệm: Lấy danh sách ngôn ngữ có thật trong `langs/` và đọc từ điển theo
  mã. Trả về rỗng khi lỗi để `main.ts` tự chọn phương án dự phòng — KHÔNG đoán
  một mã cụ thể ở đây vì làm vậy sẽ che mất lỗi thật.
Các module tương tác: frontend/src/main.ts, backend/src/api/lang.rs
*/

import { invoke } from '@tauri-apps/api/core';

/** Danh sách mã ngôn ngữ (tên file `.json` trong `langs/`), đã sắp xếp. */
export async function getAvailableLangs(): Promise<string[]> {
    try {
        return await invoke<string[]>('get_available_langs');
    } catch (error) {
        console.error('Lỗi khi lấy danh sách ngôn ngữ:', error);
        return [];
    }
}

/** Từ điển của một mã ngôn ngữ. Trả `{}` nếu không đọc được. */
export async function getLangContent(langCode: string): Promise<Record<string, string>> {
    try {
        return await invoke<Record<string, string>>('get_lang_content', { lang_code: langCode });
    } catch (error) {
        console.error(`Lỗi khi đọc ngôn ngữ ${langCode}:`, error);
        return {};
    }
}
