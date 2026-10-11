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

    const row = page.getByRole('button', { name: /vox check/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toContainText('CLI');
    await expect(row).toContainText('MCP');
    await shot(page, 'universal-palette-01-cli-row');
  });
});
