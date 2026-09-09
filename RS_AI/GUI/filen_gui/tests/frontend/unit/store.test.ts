import { describe, expect, it } from 'vitest';
import { DEFAULT_SETTINGS, normalizeSettings } from '../../../frontend/src/settings';

describe('normalizeSettings', () => {
  it('initializes every setting when storage is absent', () => {
    expect(normalizeSettings(null)).toEqual(DEFAULT_SETTINGS);
    expect(DEFAULT_SETTINGS.language).toBe('en');
  });

  it('preserves valid persisted fields and fills missing fields', () => {
    expect(normalizeSettings({ language: 'en', showHiddenFiles: false })).toEqual({
      language: 'en',
      showHiddenFiles: false,
      theme: 'default',
      font: 'default',
    });
  });
});
