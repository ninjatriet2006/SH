/*
 * Giao diện (langs/themes/fonts) — `backend/src/api/appearance_manager.rs`.
 * Backend chỉ liệt kê file CÓ THẬT, không nhồi mặc định. Giá trị "none"
 * (rỗng/không file) nghĩa là không ghi đè — UI tự quyết giao diện mặc định.
 */

import { invoke } from './ipc';
import type { FontInfo, ThemeInfo } from './types';

/** Mã ngôn ngữ có file `.json` thật, đã sắp xếp. */
export async function getAvailableLangs(): Promise<string[]> {
  try {
    return await invoke<string[]>('get_available_langs');
  } catch (error) {
    console.error('Lỗi get_available_langs:', error);
    return [];
  }
}

/** Từ điển của một mã ngôn ngữ. Đọc lỗi → `{}` để UI hiện raw ID (lộ lỗi rõ). */
export async function getLangContent(langCode: string): Promise<Record<string, string>> {
  try {
    return await invoke<Record<string, string>>('get_lang_content', { lang_code: langCode });
  } catch (error) {
    console.error(`Lỗi get_lang_content ${langCode}:`, error);
    return {};
  }
}

/** Theme có file JSON hợp lệ (rỗng = none, UI dùng mặc định của mình). */
export async function getAvailableThemes(): Promise<ThemeInfo[]> {
  try {
    return await invoke<ThemeInfo[]>('get_available_themes');
  } catch (error) {
    console.error('Lỗi get_available_themes:', error);
    return [];
  }
}

/** Font có file thật (rỗng = none, UI dùng font hệ thống). */
export async function getAvailableFonts(): Promise<FontInfo[]> {
  try {
    return await invoke<FontInfo[]>('get_available_fonts');
  } catch (error) {
    console.error('Lỗi get_available_fonts:', error);
    return [];
  }
}
