/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp hook `useTranslation()` để dịch UI linh hoạt.
- Trách nhiệm: Tra cứu key từ dictionary của `useAppearanceStore`, fallback về chính key nếu không tìm thấy.
*/

import { useAppearanceStore } from '../store/useAppearanceStore';

export function useTranslation() {
  const translations = useAppearanceStore((state) => state.translations);
  const currentLang = useAppearanceStore((state) => state.appearance.lang);

  const t = (key: string, fallback?: string): string => {
    if (translations[key]) {
      return translations[key];
    }
    // Nếu có fallback thì dùng, nếu không lấy đoạn cuối key hoặc chính nó
    return fallback || key.split('.').pop() || key;
  };

  return { t, currentLang };
}
