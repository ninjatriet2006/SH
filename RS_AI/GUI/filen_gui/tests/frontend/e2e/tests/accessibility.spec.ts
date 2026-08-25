import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';
import { installTauriMock, waitForInvoke } from '../support/tauri-mock';

/**
 * axe accessibility checks (@axe-core/playwright).
 * Policy: FAIL only on violations with impact === 'critical'. All other
 * violations (serious/moderate/minor) are reported below the PASS/FAIL line.
 */
async function reportViolations(results: Awaited<ReturnType<AxeBuilder['analyze']>>, label: string) {
  if (results.violations.length === 0) {
    console.log(`[axe] ${label}: 0 violations`);
    return;
  }
  const summary = results.violations.map(
    (v) => `  - ${v.id} (${v.impact}): ${v.nodes.length} node(s) — ${v.help}`,
  );
  console.log(`[axe] ${label}: ${results.violations.length} violation(s)\n${summary.join('\n')}`);
}

test('axe — logged-out shell', async ({ page }) => {
  await installTauriMock(page);
  await page.goto('/');
  await page.waitForLoadState('load');

  const results = await new AxeBuilder({ page }).withTags([
    'wcag2a',
    'wcag2aa',
    'wcag21a',
    'wcag21aa',
  ]).analyze();

  await reportViolations(results, 'logged-out shell');
  const critical = results.violations.filter((v) => v.impact === 'critical');
  expect(critical, `critical axe violations:\n${JSON.stringify(critical.map((c) => c.id), null, 2)}`).toEqual([]);
});

test('axe — auth modal', async ({ page }) => {
  await installTauriMock(page);
  await page.goto('/');
  await page.waitForLoadState('load');

  await page.locator('#btn-login').click();
  await expect(page.locator('.auth-modal')).toBeVisible();

  const results = await new AxeBuilder({ page }).withTags([
    'wcag2a',
    'wcag2aa',
    'wcag21a',
    'wcag21aa',
  ]).analyze();

  await reportViolations(results, 'auth modal');
  const critical = results.violations.filter((v) => v.impact === 'critical');
  expect(critical, `critical axe violations:\n${JSON.stringify(critical.map((c) => c.id), null, 2)}`).toEqual([]);
});

test('axe — explorer after login', async ({ page }) => {
  await installTauriMock(page);
  await page.goto('/');
  await page.waitForLoadState('load');

  await page.locator('#btn-login').click();
  const modal = page.locator('.auth-modal');
  await modal.locator('input[placeholder="Email"]').fill('qa@filen.io');
  await modal.locator('input[placeholder="Password"]').fill('s3cret-pw');
  await modal.locator('button:has-text("Login")').click();
  await waitForInvoke(page, 'fs_list_local');
  await waitForInvoke(page, 'fs_list_remote');
  await expect(page.locator('.dual-pane-explorer .file-table tbody tr')).toHaveCount(6); // 2 panes x 3 files

  const results = await new AxeBuilder({ page }).withTags([
    'wcag2a',
    'wcag2aa',
    'wcag21a',
    'wcag21aa',
  ]).analyze();

  await reportViolations(results, 'explorer after login');
  const critical = results.violations.filter((v) => v.impact === 'critical');
  expect(critical, `critical axe violations:\n${JSON.stringify(critical.map((c) => c.id), null, 2)}`).toEqual([]);
});
