/**
 * Workbench tab navigation — dynamic sweep of every registered leaf surface.
 *
 * Run: pnpm exec playwright test e2e/workbench-tabs.spec.ts --project=chromium
 */
import { test, expect } from '@playwright/test';
import { SURFACE_REGISTRY } from '../src/generated/surfaceRegistry.generated';
import { DEFAULT_CHILD_BY_PARENT, TOP_LEVEL_VIEWS } from '../src/lib/navigation';
import { sidebarParentLabel } from '../src/lib/lexicon';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const LEAF_VIEWS: string[] = Array.from(
  new Set(
    SURFACE_REGISTRY.filter((e) => e.viewKey && e.tier !== 'none').map((e) => e.viewKey as string),
  ),
).sort();

const SIDEBAR_PARENTS = TOP_LEVEL_VIEWS.filter((k) => k !== 'settings');

/**
 * The workbench tab bar was removed in #460 (sidebar + breadcrumb nav shell);
 * AppShell's <main data-testid="active-surface" data-view=…> is now the
 * observable record of which surface is mounted.
 */
async function expectActiveSurface(page: import('@playwright/test').Page, viewKey: string) {
  await expect(page.getByTestId('active-surface')).toHaveAttribute('data-view', viewKey);
}

test.describe('Workbench tabs — hash navigation', () => {
  for (const viewKey of LEAF_VIEWS) {
    test(`#view=${viewKey} selects workbench tab`, async ({ page }) => {
      await addMockInitScript(page, installTauriMock, viewKey);
      await page.goto(`/#view=${encodeURIComponent(viewKey)}`);
      await page.waitForSelector('nav', { timeout: 15_000 });
      await expectActiveSurface(page, viewKey);
      await expect(page.locator('[data-surface-error]')).toHaveCount(0);
      await expect
        .poll(async () => page.evaluate(() => window.location.hash))
        .toContain(`view=${encodeURIComponent(viewKey)}`);
    });
  }
});

test.describe('Workbench tabs — sidebar parents', () => {
  test.describe.configure({ mode: 'serial' });

  for (const parentKey of SIDEBAR_PARENTS) {
    const defaultChild = DEFAULT_CHILD_BY_PARENT[parentKey] ?? parentKey;
    const sidebarLabel = sidebarParentLabel(parentKey);

    test(`sidebar "${sidebarLabel}" opens default tab ${defaultChild}`, async ({ page }) => {
      await addMockInitScript(page, installTauriMock, 'dashboard');
      await page.goto('/');
      await page.waitForSelector('aside nav', { timeout: 15_000 });

      const sidebar = page.locator('aside').first();
      await sidebar.getByRole('button', { name: new RegExp(`^${sidebarLabel}`) }).click();

      await expect
        .poll(async () => page.evaluate(() => window.location.hash))
        .toContain(`view=${encodeURIComponent(defaultChild)}`);
      await expectActiveSurface(page, defaultChild);
      await expect(page.locator('[data-surface-error]')).toHaveCount(0);
    });
  }

  test('footer Settings opens settings tab', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.goto('/');
    await page.waitForSelector('aside nav', { timeout: 15_000 });
    await page.locator('aside').first().getByRole('button', { name: /^Settings/ }).scrollIntoViewIfNeeded();
    await page.locator('aside').first().getByRole('button', { name: /^Settings/ }).click();
    await expectActiveSurface(page, 'settings');
  });

  test('footer Coverage opens coverage tab', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.goto('/');
    await page.waitForSelector('aside nav', { timeout: 15_000 });
    await page.locator('aside').first().getByRole('button', { name: /^Coverage/ }).scrollIntoViewIfNeeded();
    await page.locator('aside').first().getByRole('button', { name: /^Coverage/ }).click();
    await expectActiveSurface(page, 'coverage');
  });
});

// The former "tab bar interactions" tests (switch/close/pinned-chat tabs) were
// removed with the tab bar itself in #460; sidebar navigation is covered above.
test.describe('Workbench tabs — omnibar', () => {
  test('help omnibar search opens the doc viewer', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/Search surfaces/i);
    await expect(input).toBeVisible();
    await input.fill('help cli');
    await expect(page.getByRole('button', { name: /CLI Reference/i })).toBeVisible({ timeout: 15_000 });
    await page.getByRole('button', { name: /CLI Reference/i }).click();

    // Docs open in DocViewerDrawer (a dialog) rather than a workbench tab since #460.
    await expect(page.getByRole('dialog').getByTestId('doc-reader')).toBeVisible();
  });
});

/** Surfaces with stable smoke testids for canary depth beyond tab selection. */
const SURFACE_SMOKE: Record<string, string> = {
  chat: 'chat-surface-layout',
  console: 'console-root',
};

test.describe('Workbench tabs — surface smoke', () => {
  for (const [viewKey, testId] of Object.entries(SURFACE_SMOKE)) {
    test(`#view=${viewKey} mounts ${testId}`, async ({ page }) => {
      await addMockInitScript(page, installTauriMock, viewKey);
      await page.goto(`/#view=${encodeURIComponent(viewKey)}`);
      await page.waitForSelector('nav', { timeout: 15_000 });
      await expectActiveSurface(page, viewKey);
      await expect(page.getByTestId(testId)).toBeVisible();
    });
  }
});

test.describe('Workbench tabs — scroll host', () => {
  test('settings surface scrolls inside surface-scroll-viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 420 });
    await addMockInitScript(page, installTauriMock, 'settings');
    await page.goto('/#view=settings');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByLabel('Search settings')).toBeVisible();

    const viewport = page.getByTestId('surface-scroll-viewport');
    await expect(viewport).toBeVisible();
    const scrollable = await viewport.evaluate((el) => el.scrollHeight > el.clientHeight);
    expect(scrollable).toBe(true);
    const before = await viewport.evaluate((el) => el.scrollTop);
    await viewport.evaluate((el) => {
      el.scrollTop += 400;
    });
    const after = await viewport.evaluate((el) => el.scrollTop);
    expect(after).toBeGreaterThan(before);
  });
});
