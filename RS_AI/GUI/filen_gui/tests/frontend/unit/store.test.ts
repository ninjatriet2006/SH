import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_SETTINGS, normalizeSettings } from '../../../frontend/src/settings';

class IsolatedStorage implements Storage {
  private readonly values = new Map<string, string>();

  get length(): number {
    return this.values.size;
  }

  clear(): void {
    this.values.clear();
  }

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  key(index: number): string | null {
    return [...this.values.keys()][index] ?? null;
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

beforeEach(() => {
  vi.resetModules();
  vi.stubGlobal('localStorage', new IsolatedStorage());
});

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

describe('settings persistence', () => {
  it('reads the same values after a fresh module load', async () => {
    const firstRun = await import('../../../frontend/src/store');
    firstRun.appState.settings = {
      showHiddenFiles: false,
      language: 'vi',
      theme: 'midnight',
      font: 'dejavusans',
    };
    firstRun.saveSettings();

    vi.resetModules();
    const restarted = await import('../../../frontend/src/store');

    expect(restarted.appState.settings).toEqual(firstRun.appState.settings);
  });
});
