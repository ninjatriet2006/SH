import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: (path: string) => `asset://${path}` }));

import { applyFont, applyTheme } from './appearance';

describe('runtime appearance', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('style');
    document.head.innerHTML = '';
  });

  it('replaces previous theme variables', () => {
    applyTheme({ id: 'first', name: 'First', variables: { 'colors-neon-cyan': '#123456' } });
    applyTheme({ id: 'second', name: 'Second', variables: { 'colors-text-primary': '#ffffff' } });
    expect(document.documentElement.style.getPropertyValue('--colors-neon-cyan')).toBe('');
    expect(document.documentElement.style.getPropertyValue('--colors-text-primary')).toBe('#ffffff');
  });

  it('loads a local font and updates the typography token', () => {
    applyFont({ id: 'local_demo', name: 'Demo', family: 'Demo', src_path: '/fonts/demo.woff2' });
    expect(document.getElementById('rclone-dynamic-font')?.textContent).toContain('asset:///fonts/demo.woff2');
    expect(document.documentElement.dataset.font).toBe('local_demo');
  });

  it('quotes font asset URLs containing apostrophes', () => {
    applyFont({ id: 'quoted', name: 'Quoted', family: 'Quoted', src_path: "/fonts/user's font.woff2" });
    expect(document.getElementById('rclone-dynamic-font')?.textContent)
      .toContain('url("asset:///fonts/user\'s font.woff2")');
  });
});
