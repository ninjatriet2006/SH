import { describe, expect, it, vi } from 'vitest';
import { ThemeManager } from '../../../frontend/src/themes/ThemeManager';

vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => `asset://${path}` }));

import { resolvedFont } from '../../../frontend/src/appearance';

describe('ThemeManager', () => {
  it('parses a discovered theme and rejects unknown tokens', () => {
    const theme = new ThemeManager({ 'colors.surface.canvas': '#000000' }).loadOne(
      'themes/night.json',
      JSON.stringify({ name: 'Night', version: '1', tokens: {
        'colors.surface.canvas': '#112233',
        'colors.unknown': '#ffffff',
      } }),
    );
    expect(theme.slug).toBe('night');
    expect(theme.valid).toBe(true);
    expect(theme.tokens).toEqual({ 'colors.surface.canvas': '#112233' });
    expect(theme.errors).toHaveLength(1);
  });
});

describe('appearance fallback', () => {
  it('detects when a persisted font is unavailable', () => {
    expect(resolvedFont('missing')).toBeNull();
  });
});
