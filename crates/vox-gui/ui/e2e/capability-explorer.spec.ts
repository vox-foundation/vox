import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

async function shot(page: Page, name: string) {
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, `${name}.png`) });
}

test.describe('Capability Explorer', () => {
  test.beforeEach(async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);
  });

  test('explorer renders generated groups and chip counts sum to leaf count (01)', async ({ page }) => {
    const explorerBtn = page.locator('aside').first().getByRole('button', { name: /^Explorer/ });
    await explorerBtn.scrollIntoViewIfNeeded();
    await explorerBtn.click();

    const heading = page.getByRole('heading', { name: 'Capability Explorer' });
    await expect(heading).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Everything Vox can do, by domain, and how you can reach it.')).toBeVisible();

    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible();

    const rows = treegrid.getByRole('row');
    const count = await rows.count();
    expect(count).toBeGreaterThan(20);

    await expect(treegrid).toContainText('Generic form');
    await expect(treegrid).toContainText('MCP-only / orphan');

    const chipGroup = page.getByRole('group', { name: 'Filter by status' });
    const chipNames = ['Dedicated surface', 'Generic form', 'CLI-only', 'MCP-only / orphan'];
    let chipSum = 0;
    for (const label of chipNames) {
      const btn = chipGroup.getByRole('button', { name: new RegExp(label) });
      await expect(btn).toBeVisible();
      const text = (await btn.innerText()).trim();
      const m = text.match(/(\d+)\s*$/);
      expect(m).not.toBeNull();
      chipSum += Number(m![1]);
    }
    const statusLine = page.locator('[role="status"]').filter({ hasText: /capabilities in \d+ groups/ });
    await expect(statusLine).toBeVisible();
    const statusText = await statusLine.innerText();
    const leafMatch = statusText.match(/^(\d+)\s+capabilities/);
    expect(leafMatch).not.toBeNull();
    expect(chipSum).toBe(Number(leafMatch![1]));

    await shot(page, 'capability-explorer-01-default');
  });

  test('coverage alias and sidebar', async ({ page }) => {
    await page.goto('/#view=coverage');
    const heading = page.getByRole('heading', { name: 'Capability Explorer' });
    await expect(heading).toBeVisible({ timeout: 15_000 });

    const aside = page.locator('aside').first();
    const explorerBtn = aside.getByRole('button', { name: /^Explorer/ });
    await expect(explorerBtn).toBeVisible();

    const coverageBtn = aside.getByRole('button', { name: /^Coverage/ });
    await expect(coverageBtn).toHaveCount(0);
  });

  test('domain selection, group expansion, keyboard focus, exempt strip, mcp-only, search, and no-results (02-05, 07-09)', async ({ page }) => {
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    const rail = page.getByRole('navigation', { name: 'Domains' });

    // 02: domain-selected (Mesh / Populi)
    const meshBtn = rail.getByRole('button', { name: /Mesh \/ Populi/ });
    await meshBtn.click();
    await expect(meshBtn).toHaveAttribute('aria-current', 'true');
    await expect(page.locator('[role="status"]').filter({ hasText: /capabilities in Mesh \/ Populi/ })).toBeVisible();
    await expect(treegrid).toContainText('Dedicated surface');
    await shot(page, 'capability-explorer-02-domain-selected');

    // 04: keyboard-focus on Dedicated surface group row (vox populi)
    const exemptCheckbox = page.getByRole('checkbox', { name: /Show exempt groups/ });
    await exemptCheckbox.focus();
    await page.keyboard.press('Tab');
    const populiRow = treegrid.getByRole('row').filter({ hasText: 'vox populi' }).first();
    await expect(populiRow).toBeFocused();
    await expect(populiRow).toContainText('Dedicated surface');
    await expect(populiRow.getByRole('button', { name: 'Open surface' })).toBeVisible();
    await expect(populiRow.getByRole('button', { name: /Explain capability/ })).toBeVisible();
    await expect(populiRow.getByRole('button', { name: /Open docs/ })).toBeVisible();
    await expect(populiRow).toContainText('vox-populi · L3');
    await shot(page, 'capability-explorer-04-keyboard-focus');

    // 07: mcp-only pseudo-group expanded
    const mcpGroupRow = treegrid.getByRole('row').filter({ hasText: 'MCP-only tools' }).first();
    await mcpGroupRow.click();
    await expect(mcpGroupRow).toHaveAttribute('aria-expanded', 'true');
    await expect(treegrid).toContainText('MCP-only / orphan');
    await shot(page, 'capability-explorer-07-mcp-only');

    // 03: generic-form group expanded (CI & governance -> vox audit)
    await rail.getByRole('button', { name: /CI & governance/ }).click();
    const auditRow = treegrid.getByRole('row').filter({ hasText: 'vox audit' }).first();
    await auditRow.click();
    await expect(auditRow).toHaveAttribute('aria-expanded', 'true');
    await expect(treegrid).toContainText('Generic form');
    await expect(treegrid).toContainText('vox audit arch');
    await shot(page, 'capability-explorer-03-group-expanded');

    // 05: exempt-expanded (Show exempt on -> expand vox architect)
    await exemptCheckbox.check();
    const architectRow = treegrid.getByRole('row').filter({ hasText: 'vox architect' }).first();
    await expect(architectRow).toContainText('CLI-only');
    await expect(architectRow).toContainText('Exempt');
    await architectRow.click();
    await expect(architectRow).toHaveAttribute('aria-expanded', 'true');
    await expect(treegrid).toContainText('Owner');
    await expect(treegrid).toContainText('Reason');
    await expect(treegrid).toContainText('Review by');
    await expect(treegrid).toContainText('2027-01-31');
    await shot(page, 'capability-explorer-05-exempt-expanded');

    // Reset exempt and select All domains for 08: search-filtered
    await exemptCheckbox.uncheck();
    await rail.getByRole('button', { name: /^All domains/ }).click();
    const searchInput = page.getByRole('searchbox', { name: 'Search capabilities' });
    await searchInput.fill('mesh');
    await expect(treegrid).toContainText('vox ci mesh-gate');
    await expect(treegrid).toContainText('Generic form');
    await expect(treegrid).toContainText('MCP-only / orphan');
    await shot(page, 'capability-explorer-08-search-filtered');

    // 09: no-results
    await searchInput.fill('zzzz-no-such-capability-xyz');
    await expect(page.getByRole('heading', { name: 'No capabilities match' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Clear filters' })).toBeVisible();
    await shot(page, 'capability-explorer-09-no-results');
  });

  test('exempt overdue in dark and travertine themes (06, 16)', async ({ page }) => {
    await page.clock.setFixedTime(new Date('2027-02-01T12:00:00Z'));
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    const rail = page.getByRole('navigation', { name: 'Domains' });
    await rail.getByRole('button', { name: /CI & governance/ }).click();
    await page.getByRole('checkbox', { name: /Show exempt groups/ }).check();

    const architectRow = treegrid.getByRole('row').filter({ hasText: 'vox architect' }).first();
    await expect(architectRow).toContainText('CLI-only');
    await expect(architectRow).toContainText('Exempt');
    await expect(architectRow).toContainText('Review overdue');
    await architectRow.click();
    await expect(treegrid).toContainText('2027-01-31, overdue');
    await shot(page, 'capability-explorer-06-exempt-overdue');

    await page.evaluate(() => {
      document.documentElement.dataset.theme = 'travertine';
    });
    await expect(architectRow).toContainText('Review overdue');
    await expect(treegrid).toContainText('2027-01-31, overdue');
    await shot(page, 'capability-explorer-16-exempt-overdue');
  });

  test('loading state with delayed capabilityIndex chunk (10)', async ({ page }) => {
    let releaseChunk!: () => void;
    const gate = new Promise<void>((resolve) => {
      releaseChunk = resolve;
    });

    await page.route('**/capabilityIndex.generated*', async (route) => {
      await gate;
      await route.continue();
    });

    await page.goto('/#view=explorer');
    await expect(page.getByText('Loading the capability index…')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('searchbox', { name: 'Search capabilities' })).toBeDisabled();
    await shot(page, 'capability-explorer-10-loading');

    releaseChunk();
    await expect(page.getByRole('treegrid', { name: 'Capabilities' })).toBeVisible({ timeout: 15_000 });
  });

  test('error degraded state and retry recovery (11, 12)', async ({ page }) => {
    // Blink caches failed module URLs in its per-Document ModuleMap, so fulfill the base specifier
    // with a retry-delegating thenable that imports `?attempt=N` and abort the attempt fetch until unrouted.
    await page.route('**/capabilityIndex.generated*', async (route) => {
      const url = route.request().url();
      if (url.includes('attempt=')) {
        await route.abort('failed');
        return;
      }
      await route.fulfill({
        status: 200,
        contentType: 'application/javascript',
        body: `
          let attempt = 0;
          export function then(resolve, reject) {
            attempt += 1;
            import('/src/generated/capabilityIndex.generated.ts?attempt=' + attempt).then(resolve, reject);
          }
        `,
      });
    });

    await page.goto('/#view=explorer');
    const alert = page.getByRole('alert');
    await expect(alert).toBeVisible({ timeout: 10_000 });
    await expect(alert).toContainText('Capability index unavailable');
    await expect(alert).toContainText('Showing registered surfaces only.');
    const retryBtn = alert.getByRole('button', { name: 'Retry loading' });
    await expect(retryBtn).toBeVisible();
    await expect(page.locator('section')).toContainText('Dedicated surface');
    await shot(page, 'capability-explorer-11-error-degraded');

    await page.unroute('**/capabilityIndex.generated*');
    await retryBtn.click();

    await expect(alert).toHaveCount(0);
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });
    await expect(treegrid).toContainText('Generic form');
    await expect(page.getByRole('searchbox', { name: 'Search capabilities' })).toBeFocused();
    await shot(page, 'capability-explorer-12-error-recovered');
  });

  test('explain drawer opens from row and restores focus on close (13)', async ({ page }) => {
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    const explainBtn = treegrid.getByRole('button', { name: 'Explain capability vox build' });
    await explainBtn.focus();
    await explainBtn.click();

    const drawer = page.getByRole('dialog').filter({ hasText: 'Equivalents' });
    await expect(drawer).toBeVisible({ timeout: 10_000 });
    await expect(treegrid).toContainText('Generic form');
    await shot(page, 'capability-explorer-13-explain-drawer');

    await page.keyboard.press('Escape');
    await expect(drawer).toHaveCount(0);
    await expect(explainBtn).toBeFocused();
  });

  test('travertine default, all-badges, and no-results states (14, 15, 17)', async ({ page }) => {
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    await page.evaluate(() => {
      document.documentElement.dataset.theme = 'travertine';
    });

    // 14: default in light theme
    await expect(treegrid).toContainText('Generic form');
    await expect(treegrid).toContainText('MCP-only / orphan');
    await shot(page, 'capability-explorer-14-default');

    // 15: all-badges in light theme (Orchestrator & agents + Show exempt groups shows all 4 badges + Exempt tag)
    const rail = page.getByRole('navigation', { name: 'Domains' });
    await rail.getByRole('button', { name: /Orchestrator & agents/ }).click();
    await page.getByRole('checkbox', { name: /Show exempt groups/ }).check();
    await expect(treegrid).toContainText('Dedicated surface');
    await expect(treegrid).toContainText('Generic form');
    await expect(treegrid).toContainText('CLI-only');
    await expect(treegrid).toContainText('MCP-only / orphan');
    await expect(treegrid).toContainText('Exempt');
    await shot(page, 'capability-explorer-15-all-badges');

    // 17: no-results in light theme
    const searchInput = page.getByRole('searchbox', { name: 'Search capabilities' });
    await searchInput.fill('zzzz-no-such-capability-xyz');
    await expect(page.getByRole('heading', { name: 'No capabilities match' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Clear filters' })).toBeVisible();
    await shot(page, 'capability-explorer-17-no-results');
  });

  test('narrow viewport domain select and icon-only docs buttons (18)', async ({ page }) => {
    await page.setViewportSize({ width: 960, height: 700 });
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    const domainSelect = page.getByRole('combobox', { name: 'Domain' });
    await expect(domainSelect).toBeVisible();
    await expect(treegrid).toContainText('Generic form');

    const docsBtn = treegrid.getByRole('button', { name: /^Open docs for / }).first();
    await expect(docsBtn).toBeVisible();
    expect((await docsBtn.innerText()).trim()).toBe('');

    await shot(page, 'capability-explorer-18-narrow-domain-select');
  });

  test('palette Browse all N in the Explorer link opens Explorer with seeded query (19)', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const palette = page.getByRole('dialog', { name: 'Command palette' });
    const input = palette.getByPlaceholder(/search surfaces, commands/i);
    await expect(input).toBeVisible();
    await input.fill('>vox ci');

    const browseBtn = palette.getByRole('button', { name: /^Browse all \d+ in the Explorer$/ });
    await expect(browseBtn).toBeVisible({ timeout: 10_000 });
    await browseBtn.scrollIntoViewIfNeeded();
    await shot(page, 'capability-explorer-19-palette-link');

    await browseBtn.click();
    await expect(palette).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'Capability Explorer' })).toBeVisible({ timeout: 10_000 });
    const explorerSearch = page.getByRole('searchbox', { name: 'Search capabilities' });
    await expect(explorerSearch).toHaveValue('vox ci');
    await expect(page.getByRole('treegrid', { name: 'Capabilities' })).toContainText('Generic form');
  });

  test('axe accessibility scan passes in dark and travertine themes', async ({ page }) => {
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    const darkResults = await new AxeBuilder({ page })
      .include('section[aria-labelledby="explorer-title"]')
      .analyze();
    const darkSerious = darkResults.violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? ''),
    );
    expect(darkSerious).toEqual([]);

    await page.evaluate(() => {
      document.documentElement.dataset.theme = 'travertine';
    });
    const lightResults = await new AxeBuilder({ page })
      .include('section[aria-labelledby="explorer-title"]')
      .analyze();
    const lightSerious = lightResults.violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? ''),
    );
    expect(lightSerious).toEqual([]);
  });

  test('APG nine-step keyboard sequence', async ({ page }) => {
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    // Select CI & governance so row 0 (vox audit) and row 3 (vox policy) are both collapsed expandable group rows with docs
    const rail = page.getByRole('navigation', { name: 'Domains' });
    await rail.getByRole('button', { name: /CI & governance/ }).click();

    const exemptCheckbox = page.getByRole('checkbox', { name: /Show exempt groups/ });
    await exemptCheckbox.focus();
    await expect(exemptCheckbox).toBeFocused();

    const activeInfo = () =>
      page.evaluate(() => {
        const el = document.activeElement as HTMLElement | null;
        return {
          role: el?.getAttribute('role'),
          tag: el?.tagName.toLowerCase(),
          tabIndex: el?.getAttribute('tabindex'),
          expanded: el?.getAttribute('aria-expanded'),
          rowIndex: el?.getAttribute('aria-rowindex'),
          action: el?.getAttribute('data-action'),
          text: el?.innerText?.split('\n')[0]?.trim() ?? '',
          ariaLabel: el?.getAttribute('aria-label'),
          inGrid: Boolean(el?.closest('[role="treegrid"]')),
        };
      });

    // Step 1: Tab -> first group row (role=row, tabindex=0, aria-expanded=false)
    await page.keyboard.press('Tab');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      tabIndex: '0',
      expanded: 'false',
      rowIndex: '2',
      text: 'vox audit',
    });

    // Step 2: Down x3 -> 4th visible row (a group row, still collapsed: vox policy)
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      tabIndex: '0',
      expanded: 'false',
      rowIndex: '5',
      text: 'vox policy',
    });

    // Step 3: Right -> same row, now aria-expanded=true; first verb row appears below
    await page.keyboard.press('ArrowRight');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      tabIndex: '0',
      expanded: 'true',
      rowIndex: '5',
      text: 'vox policy',
    });
    await expect(treegrid.locator('[role="row"][aria-level="2"]').first()).toBeVisible();

    // Step 4: Right -> same row's first action button ("Open form" or "Open surface")
    await page.keyboard.press('ArrowRight');
    expect(await activeInfo()).toMatchObject({
      tag: 'button',
      action: 'primary',
      text: 'Open form',
    });

    // Step 5: Right -> "Explain capability"; Right again -> "Open docs"; further Right stays on "Open docs"
    await page.keyboard.press('ArrowRight');
    expect(await activeInfo()).toMatchObject({
      tag: 'button',
      action: 'explain',
      text: 'Explain capability',
    });
    await page.keyboard.press('ArrowRight');
    expect(await activeInfo()).toMatchObject({
      tag: 'button',
      action: 'docs',
      text: 'Open docs',
    });
    await page.keyboard.press('ArrowRight');
    expect(await activeInfo()).toMatchObject({
      tag: 'button',
      action: 'docs',
      text: 'Open docs',
    });

    // Step 6: Left x2 -> "Explain capability", then "Open form"; Left again -> the row itself (still expanded)
    await page.keyboard.press('ArrowLeft');
    expect(await activeInfo()).toMatchObject({
      tag: 'button',
      action: 'explain',
    });
    await page.keyboard.press('ArrowLeft');
    expect(await activeInfo()).toMatchObject({
      tag: 'button',
      action: 'primary',
    });
    await page.keyboard.press('ArrowLeft');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      expanded: 'true',
      rowIndex: '5',
      text: 'vox policy',
    });

    // Step 7: Left -> same row, now aria-expanded=false
    await page.keyboard.press('ArrowLeft');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      expanded: 'false',
      rowIndex: '5',
      text: 'vox policy',
    });

    // Step 8: End -> last visible row; Home -> first group row
    await page.keyboard.press('End');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      rowIndex: '5',
    });
    await page.keyboard.press('Home');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      rowIndex: '2',
      text: 'vox audit',
    });

    // Step 9: Tab -> focus leaves the grid; Shift+Tab -> back on the Home row
    await page.keyboard.press('Tab');
    expect((await activeInfo()).inGrid).toBe(false);
    await page.keyboard.press('Shift+Tab');
    expect(await activeInfo()).toMatchObject({
      role: 'row',
      rowIndex: '2',
      text: 'vox audit',
      inGrid: true,
    });
  });

  test('virtualization caps rendered DOM rows and retains focused row across scroll (M8)', async ({ page }) => {
    await page.goto('/#view=explorer');
    const treegrid = page.getByRole('treegrid', { name: 'Capabilities' });
    await expect(treegrid).toBeVisible({ timeout: 15_000 });

    const searchInput = page.getByRole('searchbox', { name: 'Search capabilities' });
    await searchInput.fill('-');

    // The count grows with the catalog; the row-count bound below is what this test pins.
    await expect(page.locator('[role="status"]').filter({ hasText: /\d+ capabilities/ })).toBeVisible();

    // Poll: the status line can render before the full catalog has loaded.
    await expect
      .poll(async () => Number(await treegrid.getAttribute('aria-rowcount')), { timeout: 15_000 })
      .toBeGreaterThan(500);
    const ariaRowCount = Number(await treegrid.getAttribute('aria-rowcount'));

    const domRowCount = await treegrid.getByRole('row').count();
    expect(domRowCount).toBeLessThan(150);

    // Focus the grid, jump to End, step Up x3, then scroll the container back to top so the active row is outside the viewport window
    const firstDataRow = treegrid.locator('[role="row"][tabindex="0"]');
    await firstDataRow.focus();
    await page.keyboard.press('End');
    await expect(treegrid.locator(`[role="row"][aria-rowindex="${ariaRowCount}"]`)).toBeFocused();
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('ArrowUp');
    await expect(
      treegrid.locator(`[role="row"][aria-rowindex="${ariaRowCount - 3}"][tabindex="0"]`),
    ).toBeFocused();

    const focusedRowIndexBeforeScroll = await page.evaluate(() =>
      document.activeElement?.getAttribute('aria-rowindex'),
    );
    expect(Number(focusedRowIndexBeforeScroll)).toBeGreaterThan(500);

    // Scroll the virtualizer container to top (0) and wait for virtualizer re-render
    await treegrid.evaluate((el) => {
      el.scrollTop = 0;
      el.dispatchEvent(new Event('scroll'));
    });
    await page.waitForTimeout(100);

    const focusAfterScroll = await page.evaluate(() => {
      const el = document.activeElement;
      return {
        role: el?.getAttribute('role'),
        isConnected: Boolean(el?.isConnected),
        rowIndex: el?.getAttribute('aria-rowindex'),
      };
    });
    expect(focusAfterScroll).toEqual({
      role: 'row',
      isConnected: true,
      rowIndex: focusedRowIndexBeforeScroll,
    });
  });
});
