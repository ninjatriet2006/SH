/*
[INTEGRITY NOTES]
- Mục đích: Kiểm thử `escapeHtml` — lớp phòng vệ XSS khi ghép dữ liệu ngoài
  (tên file, tên remote do người dùng đặt) vào innerHTML.
*/
import { describe, it, expect } from 'vitest';
import { escapeHtml } from './format';

describe('escapeHtml', () => {
  it('thoát các ký tự đặc biệt của HTML', () => {
    expect(escapeHtml('<script>alert(1)</script>')).toBe(
      '&lt;script&gt;alert(1)&lt;/script&gt;',
    );
    expect(escapeHtml('a & b')).toBe('a &amp; b');
    expect(escapeHtml(`"quoted"`)).toBe('&quot;quoted&quot;');
    expect(escapeHtml("it's")).toBe('it&#39;s');
  });

  it('chặn payload phá vỡ thuộc tính (attribute breakout)', () => {
    // Tên remote dạng này từng phá được <option value="...">
    const out = escapeHtml('"><img src=x onerror=alert(1)>');
    expect(out).not.toContain('<img');
    expect(out).not.toContain('">');
  });

  it('thoát & trước tiên, không tạo entity lồng nhau', () => {
    expect(escapeHtml('&lt;')).toBe('&amp;lt;');
  });

  it('trả về chuỗi rỗng cho null/undefined', () => {
    expect(escapeHtml(null)).toBe('');
    expect(escapeHtml(undefined)).toBe('');
  });

  it('chuyển số và boolean thành chuỗi', () => {
    expect(escapeHtml(0)).toBe('0');
    expect(escapeHtml(false)).toBe('false');
  });
});
