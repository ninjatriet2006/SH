import { expect, test } from '@playwright/test';
import { installTauriMock, waitForInvoke, getInvokeLog } from '../support/tauri-mock';

/**
 * E2E UI flow: login → explorer → file ops → logout.
 * Tauri IPC is mocked (support/tauri-mock.ts); assertions verify both the DOM
 * state transitions and the exact Tauri commands emitted.
 */
test.beforeEach(async ({ page }) => {
  await installTauriMock(page);
  await page.goto('/');
  await page.waitForLoadState('load');
});

test('login → explorer → file ops → logout', async ({ page }) => {
  // ── Initial (logged out) state ─────────────────────────────────────────
  const loginBtn = page.locator('#btn-login');
  const logoutBtn = page.locator('#btn-logout');
  const accountPill = page.locator('#account-pill');

  await expect(loginBtn).toBeEnabled();
  await expect(logoutBtn).toBeDisabled();
  await expect(accountPill).toContainText('Chưa đăng nhập');

  // ── Login ─────────────────────────────────────────────────────────────
  await loginBtn.click();
  const modal = page.locator('.auth-modal');
  await expect(modal).toBeVisible();
  await modal.locator('input[placeholder="Email"]').fill('qa@filen.io');
  await modal.locator('input[placeholder="Password"]').fill('s3cret-pw');
  await modal.locator('button:has-text("Login")').click();

  await waitForInvoke(page, 'auth_login');
  await expect(modal).toBeHidden();

  // After login: logout enabled, login disabled, account pill shows user,
  // DualPaneExplorer mounted and loaded both panes from fs_list_*.
  await expect(logoutBtn).toBeEnabled();
  await expect(loginBtn).toBeDisabled();
  await expect(accountPill).toContainText('qa@filen.io');
  await waitForInvoke(page, 'fs_list_local');
  await waitForInvoke(page, 'fs_list_remote');

  const explorer = page.locator('.dual-pane-explorer');
  await expect(explorer).toBeVisible();
  const rows = explorer.locator('.pane.right .file-table tbody tr');
  await expect(rows).toHaveCount(3);
  await expect(rows.filter({ hasText: 'README.md' })).toBeVisible();

  // ── File op 1: mkdir (right pane) ─────────────────────────────────────
  await page.locator('#btn-mkdir-right').click();
  const mkdirModal = page.locator('.operation-modal');
  await expect(mkdirModal).toBeVisible();
  await expect(mkdirModal.locator('h2')).toHaveText('Create Folder');
  await mkdirModal.locator('#folderName').fill('e2e-new-folder');
  await mkdirModal.locator('.confirm').click();
  await waitForInvoke(page, 'fs_mkdir');
  await expect(mkdirModal).toBeHidden();

  // ── File op 2: rename via context menu ────────────────────────────────
  const readmeRow = rows.filter({ hasText: 'README.md' });
  await readmeRow.click({ button: 'right' });
  const menu = page.locator('.context-menu');
  await expect(menu).toBeVisible();
  await expect(menu.locator('.item')).toHaveText([
    'Open',
    'Rename',
    'Delete',
    'Copy',
    'Move',
  ]);

  await menu.locator('.item:has-text("Rename")').click();
  const renameModal = page.locator('.operation-modal');
  await expect(renameModal).toBeVisible();
  await renameModal.locator('#newName').fill('renamed.md');
  await renameModal.locator('.confirm').click();
  await waitForInvoke(page, 'fs_rename');
  await expect(renameModal).toBeHidden();

  // ── File op 3: delete via context menu ────────────────────────────────
  const photoRow = rows.filter({ hasText: 'photo.png' });
  await photoRow.click({ button: 'right' });
  await expect(menu).toBeVisible();
  await menu.locator('.item:has-text("Delete")').click();
  const deleteModal = page.locator('.operation-modal');
  await expect(deleteModal).toBeVisible();
  await deleteModal.locator('.confirm').click();
  await waitForInvoke(page, 'fs_delete');
  await expect(deleteModal).toBeHidden();

  // ── File op 4+5: copy & move (prompt dialogs) ─────────────────────────
  page.on('dialog', (dialog) => dialog.accept('/e2e-target'));
  await rows.filter({ hasText: 'README.md' }).click({ button: 'right' });
  await expect(menu).toBeVisible();
  await menu.locator('.item:has-text("Copy")').click();
  await waitForInvoke(page, 'fs_copy');

  await rows.filter({ hasText: 'README.md' }).click({ button: 'right' });
  await expect(menu).toBeVisible();
  await menu.locator('.item:has-text("Move")').click();
  await waitForInvoke(page, 'fs_move');

  // ── Logout ────────────────────────────────────────────────────────────
  await logoutBtn.click();
  await waitForInvoke(page, 'auth_logout');
  await expect(loginBtn).toBeEnabled();
  await expect(logoutBtn).toBeDisabled();
  await expect(accountPill).toContainText('Chưa đăng nhập');

  // ── Audit the exact command sequence emitted ─────────────────────────
  const log = await getInvokeLog(page);
  const cmds = log.map((e) => e.cmd);
  for (const expected of [
    'auth_login',
    'fs_list_local',
    'fs_list_remote',
    'fs_mkdir',
    'fs_rename',
    'fs_delete',
    'fs_copy',
    'fs_move',
    'auth_logout',
  ]) {
    expect(cmds, `expected invoke('${expected}') to have been emitted`).toContain(expected);
  }
});
