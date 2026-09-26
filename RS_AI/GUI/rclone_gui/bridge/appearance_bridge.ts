/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho tuỳ biến giao diện (Theme, Font, Ngôn ngữ i18n).
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng ThemeInfo, FontInfo, AppearanceSettings từ `types.ts`.
*/

import {
  invokeCommand,
  type AppearanceSettings,
  type FontInfo,
  type JsonValue,
  type ThemeInfo,
} from './types';

/** Quét danh sách mã ngôn ngữ hiện có (en, vi...). */
export async function getAvailableLangs(): Promise<string[]> {
  try {
    return await invokeCommand<string[]>('get_available_langs');
  } catch (error) {
    console.error('Lỗi get_available_langs:', error);
    return ['en'];
  }
}

/** Tải nội dung từ điển i18n của một ngôn ngữ cụ thể. */
export async function getLangContent(langCode: string): Promise<Record<string, JsonValue>> {
  try {
    return await invokeCommand<Record<string, JsonValue>, { lang_code: string }>('get_lang_content', {
      lang_code: langCode,
    });
  } catch (error) {
    console.error(`Lỗi get_lang_content ${langCode}:`, error);
    return {};
  }
}

/** Quét danh sách theme giao diện hỗ trợ. */
export async function getAvailableThemes(): Promise<ThemeInfo[]> {
  try {
    return await invokeCommand<ThemeInfo[]>('get_available_themes');
  } catch (error) {
    console.error('Lỗi get_available_themes:', error);
    return [];
  }
}

/** Quét danh sách phông chữ tuỳ biến hỗ trợ. */
export async function getAvailableFonts(): Promise<FontInfo[]> {
  try {
    return await invokeCommand<FontInfo[]>('get_available_fonts');
  } catch (error) {
    console.error('Lỗi get_available_fonts:', error);
    return [];
  }
}

/** Tải cấu hình giao diện hiện tại của ứng dụng. */
export async function getAppearance(): Promise<AppearanceSettings> {
  return await invokeCommand<AppearanceSettings>('get_appearance');
}

/** Lưu và áp dụng cấu hình giao diện mới. */
export async function setAppearance(settings: AppearanceSettings): Promise<AppearanceSettings> {
  return await invokeCommand<AppearanceSettings, { settings: AppearanceSettings }>('set_appearance', {
    settings,
  });
}
