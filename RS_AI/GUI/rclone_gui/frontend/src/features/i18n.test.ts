/*
[INTEGRITY NOTES]
Mục đích: Chặn hồi quy cho cơ chế i18n của rclone_gui.
Bối cảnh bug: `main.ts` từng `import viLang from '../../langs/vi.json'` — nhúng
  cứng một file vào bundle. Hậu quả: sửa bản dịch phải build lại, `langs/` trong
  release là file chết, và app không chạy được với bộ ngôn ngữ không có `vi`.
Trách nhiệm: Kiểm tra thứ tự ưu tiên khi chọn ngôn ngữ, và hành vi khi không có
  file nào dùng được (phải để UI hiện raw ID, không im lặng).
*/

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { resolveLanguage, applyLanguage, observeLanguage, type LangSource } from './i18n';

/** Giả lập thư mục `langs/`: khoá là mã ngôn ngữ, giá trị là nội dung file. */
function fakeLangs(files: Record<string, Record<string, string>>): LangSource {
  return {
    listLangs: async () => Object.keys(files).sort(),
    readLang: async (code: string) => files[code] ?? {},
  };
}

describe('resolveLanguage', () => {
  it('dùng ngôn ngữ được yêu cầu khi file tồn tại', async () => {
    const deps = fakeLangs({ en: { app_title: 'Rclone' }, vi: { app_title: 'Rclone VN' } });
    const { code, data } = await resolveLanguage('vi', deps);
    expect(code).toBe('vi');
    expect(data.app_title).toBe('Rclone VN');
  });

  it('không hardcode vi: bộ ngôn ngữ không có vi vẫn chạy', async () => {
    const deps = fakeLangs({ ja: { app_title: 'Rclone JP' }, ko: { app_title: 'Rclone KR' } });
    const { code, data } = await resolveLanguage(null, deps);
    // 'ja' đứng trước 'ko' sau khi sắp xếp.
    expect(code).toBe('ja');
    expect(data.app_title).toBe('Rclone JP');
  });

  it('không thử mã nào ngoài danh sách thực có khi preferred rỗng', async () => {
    // Chốt trực tiếp điều đã từng sai: không được chèn mã cứng (vd "vi") vào
    // danh sách ứng viên. Chỉ những mã do `listLangs()` trả về mới được đọc.
    const readLang = vi.fn(
      async (code: string): Promise<Record<string, string>> =>
        code === 'ko' ? { app_title: 'Rclone KR' } : {}
    );
    const { code } = await resolveLanguage(null, {
      listLangs: async () => ['ja', 'ko'],
      readLang,
    });
    expect(code).toBe('ko');
    const daDoc = readLang.mock.calls.map(([c]) => c);
    expect(daDoc).toEqual(['ja', 'ko']);
    expect(daDoc).not.toContain('vi');
  });

  it('rớt sang file có thật khi mã yêu cầu không tồn tại', async () => {
    const deps = fakeLangs({ en: { app_title: 'Rclone' } });
    const { code } = await resolveLanguage('khong_ton_tai_9999', deps);
    expect(code).toBe('en');
  });

  it('bỏ qua file rỗng/hỏng và thử file kế tiếp', async () => {
    const deps = fakeLangs({ aa_hong: {}, bb_ok: { app_title: 'OK' } });
    const { code, data } = await resolveLanguage(null, deps);
    expect(code).toBe('bb_ok');
    expect(data.app_title).toBe('OK');
  });

  it('langs/ rỗng thì trả từ điển rỗng và ghi log lỗi', async () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { code, data } = await resolveLanguage(null, fakeLangs({}));
    expect(code).toBeNull();
    expect(data).toEqual({});
    expect(spy).toHaveBeenCalled(); // không im lặng
    spy.mockRestore();
  });

  it('không đọc lặp khi preferred trùng file đầu tiên', async () => {
    const readLang = vi.fn(
      async (code: string): Promise<Record<string, string>> =>
        code === 'en' ? { app_title: 'Rclone' } : {}
    );
    const { code } = await resolveLanguage('en', {
      listLangs: async () => ['en', 'vi'],
      readLang,
    });
    expect(code).toBe('en');
    expect(readLang).toHaveBeenCalledTimes(1);
  });
});

describe('applyLanguage', () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <span data-lang-id="app_title">RAW_TITLE</span>
      <button data-lang-id="nav_explorer">RAW_NAV</button>
      <input data-lang-id="pane_filter_placeholder" placeholder="RAW_PH" />
      <span data-lang-id="khong_co_trong_tu_dien">nav_missing</span>
    `;
  });

  it('áp text và placeholder theo data-lang-id', () => {
    applyLanguage({
      app_title: 'Rclone Manager',
      nav_explorer: 'Explorer',
      pane_filter_placeholder: 'Lọc...',
    });
    expect(document.querySelector('[data-lang-id="app_title"]')?.textContent).toBe(
      'Rclone Manager'
    );
    expect(document.querySelector('[data-lang-id="nav_explorer"]')?.textContent).toBe('Explorer');
    expect(
      (document.querySelector('[data-lang-id="pane_filter_placeholder"]') as HTMLInputElement)
        .placeholder
    ).toBe('Lọc...');
  });

  it('ID thiếu bản dịch thì giữ nguyên nội dung để lỗi hiện ra', () => {
    applyLanguage({ app_title: 'Rclone Manager' });
    expect(
      document.querySelector('[data-lang-id="khong_co_trong_tu_dien"]')?.textContent
    ).toBe('nav_missing');
  });

  it('từ điển rỗng thì không xoá trắng UI', () => {
    applyLanguage({});
    expect(document.querySelector('[data-lang-id="app_title"]')?.textContent).toBe('RAW_TITLE');
  });
});

describe('observeLanguage', () => {
  it('dịch DOM được tạo sau khi khởi tạo', async () => {
    document.body.innerHTML = '<div id="host"></div>';
    const observer = observeLanguage(() => ({ late_label: 'Đã dịch' }));
    document.getElementById('host')!.innerHTML = '<span data-lang-id="late_label">RAW</span>';
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(document.querySelector('[data-lang-id="late_label"]')?.textContent).toBe('Đã dịch');
    observer.disconnect();
  });
});
