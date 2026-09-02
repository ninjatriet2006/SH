/*
[INTEGRITY NOTES]
- Mục đích: Kiểm thử `readStored` — cơ chế di trú khoá localStorage từ tiền tố
  `filen_` (codebase gốc) sang `rclonegui_`.
- Trách nhiệm: Đảm bảo không mất dữ liệu người dùng đã lưu và không ghi đè dữ liệu mới.
*/
import { describe, it, expect, beforeEach } from 'vitest';
import { readStored } from './store';

describe('readStored', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('trả về giá trị của khoá mới nếu đã tồn tại', () => {
    localStorage.setItem('rclonegui_x', 'new');
    expect(readStored('rclonegui_x')).toBe('new');
  });

  it('di trú giá trị từ khoá filen_* rồi xoá khoá cũ', () => {
    localStorage.setItem('filen_x', 'legacy');
    expect(readStored('rclonegui_x')).toBe('legacy');
    expect(localStorage.getItem('rclonegui_x')).toBe('legacy');
    expect(localStorage.getItem('filen_x')).toBeNull();
  });

  it('không ghi đè khoá mới bằng khoá cũ khi cả hai cùng tồn tại', () => {
    localStorage.setItem('rclonegui_x', 'new');
    localStorage.setItem('filen_x', 'legacy');
    expect(readStored('rclonegui_x')).toBe('new');
    // Khoá cũ được giữ nguyên vì không cần di trú.
    expect(localStorage.getItem('filen_x')).toBe('legacy');
  });

  it('trả về null khi không có khoá nào', () => {
    expect(readStored('rclonegui_missing')).toBeNull();
  });
});
