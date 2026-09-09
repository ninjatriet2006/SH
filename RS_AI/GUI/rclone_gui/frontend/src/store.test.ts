/*
[INTEGRITY NOTES]
- Mục đích: Kiểm thử `readStored` — cơ chế di trú khoá localStorage từ tiền tố
  `filen_` (codebase gốc) sang `rclonegui_`.
- Trách nhiệm: Đảm bảo không mất dữ liệu người dùng đã lưu và không ghi đè dữ liệu mới.
*/
import { describe, it, expect, beforeEach } from 'vitest';
import { normalizeSettings, readStored, type AppSettings } from './store';

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

  it('di trú đúng ba khoá legacy theo contract Rclone', () => {
    for (const suffix of ['settings', 'activity_log', 'bookmarks']) {
      localStorage.setItem(`filen_${suffix}`, suffix);
      expect(readStored(`rclonegui_${suffix}`)).toBe(suffix);
      expect(localStorage.getItem(`filen_${suffix}`)).toBeNull();
    }
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

describe('normalizeSettings', () => {
  const settings = (overrides: Partial<AppSettings>): AppSettings => ({
    showHiddenFiles: false,
    language: 'en',
    theme: 'default',
    font: 'default',
    ...overrides,
  });

  it('thay language không tồn tại bằng ngôn ngữ khả dụng đầu tiên', () => {
    const value = settings({ language: 'missing' });
    expect(normalizeSettings(value, ['en', 'vi'], ['default'], ['default'])).toBe(true);
    expect(value.language).toBe('en');
  });

  it('thay theme không tồn tại bằng theme khả dụng đầu tiên', () => {
    const value = settings({ theme: 'missing' });
    expect(normalizeSettings(value, ['en'], ['default', 'amber'], ['default'])).toBe(true);
    expect(value.theme).toBe('default');
  });

  it('thay font không tồn tại bằng font khả dụng đầu tiên', () => {
    const value = settings({ font: 'missing' });
    expect(normalizeSettings(value, ['en'], ['default'], ['default', 'dejavusans'])).toBe(true);
    expect(value.font).toBe('default');
  });

  it('giữ nguyên ID hợp lệ và không báo thay đổi', () => {
    const value = settings({ language: 'vi', theme: 'amber', font: 'dejavusans' });
    expect(normalizeSettings(value, ['en', 'vi'], ['default', 'amber'], ['default', 'dejavusans'])).toBe(false);
    expect(value).toMatchObject({ language: 'vi', theme: 'amber', font: 'dejavusans' });
  });
});
