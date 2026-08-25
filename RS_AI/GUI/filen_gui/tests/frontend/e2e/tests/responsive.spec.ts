import { expect, test } from '@playwright/test';
import { installTauriMock } from '../support/tauri-mock';

/**
 * Responsive layout tests across desktop / tablet / mobile breakpoints.
 * Breakpoints come from src/style.css:
 *   - max-width:1024px → sidebar hidden, hamburger visible
 *   - max-width:768px  → central goes single-column, pane-divider hidden
 *   - max-width:480px  → top-bar stacks vertically, smaller nav text
 */
async function css(page: import('@playwright/test').Page, selector: string, prop: string): Promise<string> {
  return page.$eval(selector, (el, p) => getComputedStyle(el).getPropertyValue(p as string), prop);
}

/** Count resolved grid columns (fr resolves to px in computed styles). */
async function gridColumns(page: import('@playwright/test').Page, selector: string): Promise<number> {
  const cols = await css(page, selector, 'grid-template-columns');
  return cols.trim().split(/\s+/).filter(Boolean).length;
}

test.describe('responsive viewports', () => {
  test('desktop 1440×900 — sidebar + dual pane side-by-side', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await installTauriMock(page);
    await page.goto('/');
    await page.waitForLoadState('load');

    await expect(page.locator('#sidebar')).toBeVisible();
    await expect(page.locator('#sidebar-toggle')).toBeHidden();
    expect(await css(page, '#sidebar', 'display')).not.toBe('none');
    expect(await gridColumns(page, '#central')).toBe(3);
    expect(await css(page, '#pane-divider', 'display')).not.toBe('none');
  });

  test('tablet 900×700 — sidebar hidden, hamburger visible, panes still split', async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 700 });
    await installTauriMock(page);
    await page.goto('/');
    await page.waitForLoadState('load');

    expect(await css(page, '#sidebar', 'display')).toBe('none');
    await expect(page.locator('#sidebar-toggle')).toBeVisible();
    expect(await gridColumns(page, '#central')).toBe(3);
  });

  test('mobile 375×667 — stacked panes, divider hidden, hamburger visible', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await installTauriMock(page);
    await page.goto('/');
    await page.waitForLoadState('load');

    expect(await css(page, '#sidebar', 'display')).toBe('none');
    await expect(page.locator('#sidebar-toggle')).toBeVisible();
    // Single-column central: both panes stack vertically.
    expect(await gridColumns(page, '#central')).toBe(1);
    expect(await css(page, '#pane-divider', 'display')).toBe('none');
    // Panes remain within viewport width (no horizontal overflow).
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
  });
});
