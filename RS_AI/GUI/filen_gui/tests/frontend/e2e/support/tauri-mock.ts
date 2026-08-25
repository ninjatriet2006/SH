import type { Page } from '@playwright/test';

/**
 * Tauri IPC mock.
 *
 * The frontend calls `window.__TAURI_INTERNALS__.invoke(cmd, args)` from
 * `@tauri-apps/api/core`. In a plain Chromium context this object does not
 * exist, so we install it before any app module runs. Every call is recorded
 * into `window.__e2eInvokeLog` so tests can assert which commands fired.
 */
export const TAURI_MOCK_SOURCE = `
  window.__TAURI_INTERNALS__ = {
    invoke: async (cmd, args) => {
      const log = (window.__e2eInvokeLog = window.__e2eInvokeLog || []);
      log.push({ cmd, args: args ?? {} });
      switch (cmd) {
        case 'auth_login':
        case 'auth_login_twofa':
        case 'auth_logout':
          return { ok: true };
        case 'fs_list_local':
        case 'fs_list_remote':
          return [
            { name: 'Documents', is_dir: true, size: 0, mod_time: '2026-08-01' },
            { name: 'README.md', is_dir: false, size: 1234, mod_time: '2026-08-02' },
            { name: 'photo.png', is_dir: false, size: 56789, mod_time: '2026-08-03' },
          ];
        case 'fs_mkdir':
        case 'fs_rename':
        case 'fs_delete':
        case 'fs_open':
        case 'fs_copy':
        case 'fs_move':
          return { ok: true };
        default:
          throw new Error('E2E mock: unknown Tauri command "' + cmd + '"');
      }
    }
  };
`;

export function installTauriMock(page: Page): Promise<void> {
  return page.addInitScript(TAURI_MOCK_SOURCE);
}

export function getInvokeLog(page: Page): Promise<Array<{ cmd: string; args: Record<string, unknown> }>> {
  return page.evaluate(() => (window as any).__e2eInvokeLog ?? []);
}

export function waitForInvoke(page: Page, cmd: string): Promise<void> {
  return page.waitForFunction(
    (c) =>
      Array.isArray((window as any).__e2eInvokeLog) &&
      (window as any).__e2eInvokeLog.some((e: { cmd: string }) => e.cmd === c),
    cmd,
  );
}
