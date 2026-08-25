/**
 * filen_gui — i18n (docs/i18n-and-themes.md §2, v3.0.0)
 *
 * Dictionary phẳng `{ lang_id: string }` per-language JSON:
 *   src/i18n/en.json (default), src/i18n/vi.json
 *
 * Lookup qua `t(key)`:
 *   fallback chain: current lang → en → raw key (không bao giờ trả về rỗng).
 * Hot-switch: `setLanguage(lang)` + `applyLanguage()` — duyệt DOM `[data-lang-id]`
 * gán `textContent` (không reload WebView, không mất state).
 *
 * Thêm ngôn ngữ = thêm 1 file JSON + đăng ký trong `dicts` — không sửa code lookup.
 */
import en from "./i18n/en.json";
import vi from "./i18n/vi.json";

export type LangCode = "en" | "vi";

export interface Dict {
  [key: string]: string;
}

const DEFAULT_LANG: LangCode = "en";

/** Đăng ký dictionary — thêm ngôn ngữ mới tại đây. */
const dicts: Record<LangCode, Dict> = {
  en: en as Dict,
  vi: vi as Dict,
};

let currentLang: LangCode = DEFAULT_LANG;

export function isSupported(lang: string): lang is LangCode {
  return lang in dicts;
}

export function setLanguage(lang: string): void {
  currentLang = isSupported(lang) ? lang : DEFAULT_LANG;
}

export function getLanguage(): LangCode {
  return currentLang;
}

export function getDict(lang: LangCode = currentLang): Dict {
  return dicts[lang] ?? dicts.en;
}

/**
 * Tra cứu chuỗi UI theo key.
 * Fallback chain: current lang → en → raw key (dễ phát hiện key thiếu khi QA).
 */
export function t(key: string): string {
  const dict = getDict(currentLang);
  if (key in dict) return dict[key];
  if (currentLang !== DEFAULT_LANG && key in dicts.en) return dicts.en[key];
  return key;
}

/** Hot-switch: cập nhật `textContent` mọi node có `data-lang-id`. */
export function applyLanguage(): void {
  document.querySelectorAll<HTMLElement>("[data-lang-id]").forEach((el) => {
    const key = el.dataset.langId;
    if (key) el.textContent = t(key);
  });
}

/** Danh sách mã ngôn ngữ hỗ trợ (cho menu chọn ngôn ngữ). */
export function supportedLanguages(): LangCode[] {
  return Object.keys(dicts) as LangCode[];
}