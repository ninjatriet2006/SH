/*
[INTEGRITY NOTES]
Mục đích: Cơ chế đa ngôn ngữ (i18n) theo quy tắc ID Linking (`data-lang-id`).
Trách nhiệm: Chọn ngôn ngữ khả dụng lúc CHẠY và áp từ điển vào DOM.
Các module tương tác: /bridge/lang_api.ts, frontend/src/main.ts

Tách riêng khỏi `main.ts` để test được mà không phải nạp toàn bộ ứng dụng
(main.ts kéo theo listener sự kiện Tauri, không chạy trong môi trường test).
*/

import { getAvailableLangs, getLangContent } from '../../../bridge/lang_api';

/** Nguồn dữ liệu ngôn ngữ — tách ra để test thay bằng bản giả lập. */
export interface LangSource {
  listLangs: () => Promise<string[]>;
  readLang: (code: string) => Promise<Record<string, string>>;
}

const defaultSource: LangSource = {
  listLangs: getAvailableLangs,
  readLang: getLangContent,
};

/**
 * Chọn và nạp từ điển: thử `preferred` (nếu có) rồi lần lượt các file thực có
 * trong `langs/`. Không có file nào dùng được thì trả từ điển RỖNG — UI sẽ hiện
 * raw ID để lỗi lộ ra, thay vì im lặng như thể mọi thứ bình thường.
 *
 * KHÔNG hardcode mã ngôn ngữ nào: app phải chạy với bộ ngôn ngữ bất kỳ.
 */
export async function resolveLanguage(
  preferred: string | null,
  source: LangSource = defaultSource
): Promise<{ code: string | null; data: Record<string, string> }> {
  const available = await source.listLangs();
  const candidates = [preferred, ...available].filter(
    (c): c is string => typeof c === 'string' && c.length > 0
  );
  const tried = new Set<string>();

  for (const code of candidates) {
    if (tried.has(code)) continue;
    tried.add(code);
    const data = await source.readLang(code);
    if (data && Object.keys(data).length > 0) {
      return { code, data };
    }
  }

  console.error(
    `Không nạp được ngôn ngữ nào (thư mục langs/ rỗng hoặc file hỏng). ` +
      `Đã thử: ${[...tried].join(', ') || '(không có)'}`
  );
  return { code: null, data: {} };
}

/**
 * Áp từ điển vào DOM theo `data-lang-id`. ID thiếu trong từ điển thì giữ nguyên
 * nội dung sẵn có (thường là raw ID) để chỗ thiếu bản dịch nhìn thấy được.
 */
export function applyLanguage(
  data: Record<string, string>,
  root: HTMLElement | Document = document.body
) {
  const elements = [
    ...(root instanceof HTMLElement && root.matches('[data-lang-id]') ? [root] : []),
    ...root.querySelectorAll('[data-lang-id]'),
  ];
  elements.forEach((el) => {
    const id = el.getAttribute('data-lang-id');
    if (!id) return;
    const text = data[id];
    if (text === undefined) return;
    if (el.tagName === 'INPUT' && (el as HTMLInputElement).placeholder !== undefined) {
      (el as HTMLInputElement).placeholder = text;
    } else {
      el.textContent = text;
    }
  });
}

/** Tự dịch các component/modal được gắn vào DOM sau lần tải ngôn ngữ đầu tiên. */
export function observeLanguage(data: () => Record<string, string>, root: HTMLElement = document.body) {
  const observer = new MutationObserver((records) => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (!(node instanceof HTMLElement)) continue;
        applyLanguage(data(), node);
      }
    }
  });
  observer.observe(root, { childList: true, subtree: true });
  return observer;
}
