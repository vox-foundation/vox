import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

/**
 * Universal palette (phase 06.3): capability-index rows in the Cmd/Ctrl+K palette.
 *
 * Run: pnpm exec playwright test e2e/universal-palette.spec.ts --project=chromium
 */
const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

async function shot(page: Page, name: string) {
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, `${name}.png`) });
}

test.describe('Universal palette', () => {
  test.beforeEach(async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);
  });

  test('index CLI row opens the prefilled Catalog form', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await expect(input).toBeVisible();
    await input.fill('vox check');

    // The MCP twin (vox_check) also mentions the verb in its detail now that every kind is
    // listed, so anchor on the row label.
    const row = page.getByRole('button', { name: /^vox check\b/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toContainText('CLI');
    await expect(row).toContainText('MCP');
    await shot(page, 'universal-palette-01-cli-row');

    // Activate the row (D-01: opens the Catalog form, never executes). Enter-on-top-row is
    // covered by the Omnibar unit test; Best match (a later plan) will put this row first.
    await row.click();
    await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=catalog');
    await expect(page.getByRole('heading', { name: 'vox check' })).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('[data-catalog-path="check"]')).toHaveAttribute('aria-current', 'true');
    await shot(page, 'universal-palette-02-prefilled-form');
  });

  test('a full CLI path ranks the exact verb first without fuzzy noise (D-19)', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await input.fill('vox fabrica fmt');

    const row = page.getByRole('button', { name: /vox fabrica fmt/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    // Jaro-Winkler is a typo fallback only: an exact path must not drag in near-miss verbs.
    await expect(page.getByRole('button', { name: /vox fmt\b/ })).toHaveCount(0);
    await shot(page, 'universal-palette-03-ranked-exact-path');
  });

  test('every kind is reachable from ⌘K (D-05, D-08, D-08a, D-09, D-10)', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await expect(input).toBeVisible();
    const row = (text: string | RegExp) => page.getByRole('button').filter({ hasText: text }).first();
    // Best match MOVES the top hit out of its facet, so the Best match row is the first button
    // under the "Best match" heading.
    // Scoped to the dialog: the page behind it has nav items called "Commands" etc.
    const heading = (name: string) => page.getByRole('dialog', { name: 'Command palette' }).getByText(name, { exact: true });
    const bestRow = () => heading('Best match').locator('xpath=ancestor::div[1]/following-sibling::button[1]');

    // CLI verb: exact full path is the Best match.
    await input.fill('vox fabrica fmt');
    await expect(bestRow()).toContainText('vox fabrica fmt', { timeout: 10_000 });
    await shot(page, 'universal-palette-03-best-match');

    // MCP tool, with its CLI equivalent chip.
    await input.fill('');
    await input.fill('vox_capability_explain');
    await expect(row('vox_capability_explain')).toBeVisible({ timeout: 10_000 });
    await expect(row('vox_capability_explain')).toContainText('MCP');
    await expect(row('vox_capability_explain')).toContainText('CLI');

    // Tools & crates facet, capped with an honest count only when it overflows.
    await input.fill('');
    await input.fill('capability');
    await expect(heading('Tools & crates')).toBeVisible({ timeout: 10_000 });
    await expect(row('vox_capability_features')).toBeVisible();
    await row('vox_capability_features').scrollIntoViewIfNeeded(); // the facet sits below the Commands rows
    await shot(page, 'universal-palette-04-tools');

    // # mode: crates only, layer query, "N of M" narrowing hint on overflow (D-08a, D-10).
    await input.fill('');
    await input.fill('#L3');
    await expect(heading('Best match')).toBeVisible({ timeout: 10_000 });
    await expect(bestRow()).toContainText('vox-');
    await expect(heading('Tools & crates')).toBeVisible();
    await expect(page.getByText(/^6 of \d+$/)).toBeVisible();
    await expect(page.getByText(/Keep typing to narrow/)).toBeVisible();
    await expect(heading('Surfaces')).toHaveCount(0);
    await expect(heading('Commands')).toHaveCount(0);
    await shot(page, 'universal-palette-05-crates');

    await input.fill('');
    await input.fill('#vox-actor-runtime');
    await expect(row('vox-actor-runtime')).toBeVisible({ timeout: 10_000 });

    // Surface.
    await input.fill('');
    await input.fill('mesh');
    await expect(bestRow()).toContainText('Mesh', { timeout: 10_000 });
    await expect(bestRow()).toContainText('GUI');

    // Doc (title-exact match, so default mode shows it).
    await input.fill('');
    await input.fill('where things live');
    await expect(row('Where Things Live')).toBeVisible({ timeout: 10_000 });

    // Setting (federated).
    await input.fill('');
    await input.fill('Max concurrent agents');
    await expect(row('Max concurrent agents')).toBeVisible({ timeout: 10_000 });

    // Skill (index gap-fill: not installed in the mock).
    await input.fill('');
    await input.fill('algorithmic-art');
    await expect(row('algorithmic-art')).toBeVisible({ timeout: 10_000 });
  });
});
