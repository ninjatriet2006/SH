import { describe, it, expect } from 'vitest';
import { formatSize, formatDate } from '../../../../frontend/src/features/format';

describe('formatSize', () => {
  it('formats bytes', () => {
    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(1023)).toBe('1023 B');
  });
  it('formats KB', () => {
    expect(formatSize(1024)).toBe('1.0 KB');
    expect(formatSize(2048)).toBe('2.0 KB');
  });
  it('formats MB', () => {
    expect(formatSize(1024 * 1024)).toBe('1.0 MB');
  });
  it('formats GB', () => {
    expect(formatSize(1024 * 1024 * 1024)).toBe('1.0 GB');
  });
  it('handles invalid input', () => {
    expect(formatSize(-1)).toBe('-');
    expect(formatSize(NaN)).toBe('-');
    expect(formatSize(Infinity)).toBe('-');
  });
});

describe('formatDate', () => {
  it('formats ISO to dd/mm/yyyy hh:mm', () => {
    // Use a fixed local date to avoid TZ issues
    const d = new Date(2024, 0, 5, 9, 7); // Jan 5 2024 09:07 local
    expect(formatDate(d.toISOString())).toBe('05/01/2024 09:07');
  });
  it('returns "-" for empty', () => {
    expect(formatDate('')).toBe('-');
  });
  it('returns original for invalid date', () => {
    expect(formatDate('not-a-date')).toBe('not-a-date');
  });
});