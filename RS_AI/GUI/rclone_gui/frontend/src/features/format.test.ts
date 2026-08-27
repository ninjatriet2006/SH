/*
[INTEGRITY NOTES]
- Mục đích: Bộ kiểm thử (Unit Tests) cho module `format.ts`.
- Trách nhiệm: Đảm bảo độ chính xác của các hàm quy đổi định dạng (formatSize, formatDate) theo các ngoại lệ (edge cases).
*/
import { describe, it, expect } from 'vitest';
import { formatSize, formatDate } from './format';

describe('formatSize', () => {
  it('Nên định dạng số byte chính xác', () => {
    expect(formatSize(500)).toBe('500 B');
    expect(formatSize(1024)).toBe('1.0 KB');
    expect(formatSize(1024 * 1024)).toBe('1.0 MB');
    expect(formatSize(1.5 * 1024 * 1024 * 1024)).toBe('1.5 GB');
  });

  it('Nên xử lý các trường hợp ngoại lệ an toàn', () => {
    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(-10)).toBe('-'); // Negative bytes
    expect(formatSize(NaN)).toBe('-');
    expect(formatSize(Infinity)).toBe('-');
  });
});

describe('formatDate', () => {
  it('Nên định dạng chuỗi ISO nguyên chuẩn thành công', () => {
    const isoStr = '2023-05-10T14:30:00.000Z';
    const result = formatDate(isoStr);
    expect(result).toMatch(/^\d{2}\/\d{2}\/2023 \d{2}:\d{2}$/);
  });

  it('Nên xử lý các chuỗi ngày tháng không hợp lệ an toàn', () => {
    expect(formatDate('')).toBe('-');
    expect(formatDate('not a date')).toBe('not a date');
  });
});
