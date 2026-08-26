import { describe, it, expect } from 'vitest';
import { formatSize, formatDate } from './format';

describe('formatSize', () => {
  it('should format bytes correctly', () => {
    expect(formatSize(500)).toBe('500 B');
    expect(formatSize(1024)).toBe('1.0 KB');
    expect(formatSize(1024 * 1024)).toBe('1.0 MB');
    expect(formatSize(1.5 * 1024 * 1024 * 1024)).toBe('1.5 GB');
  });

  it('should handle edge cases', () => {
    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(-10)).toBe('-'); // Negative bytes
    expect(formatSize(NaN)).toBe('-');
    expect(formatSize(Infinity)).toBe('-');
  });
});

describe('formatDate', () => {
  it('should format standard ISO strings correctly', () => {
    const isoStr = '2023-05-10T14:30:00.000Z';
    const result = formatDate(isoStr);
    expect(result).toMatch(/^\d{2}\/\d{2}\/2023 \d{2}:\d{2}$/);
  });

  it('should handle invalid dates', () => {
    expect(formatDate('')).toBe('-');
    expect(formatDate('not a date')).toBe('not a date');
  });
});
