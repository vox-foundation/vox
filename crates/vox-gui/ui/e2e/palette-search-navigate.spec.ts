import { test, expect } from '@playwright/test';
import { installOperatorShellMock } from './lib/operatorShellMock';

/**
 * Command palette — Best match contract and keyboard navigation (phase 06.3, D-20).
 * The index replaces the surface/command rows (D-05) and the single best hit is MOVED to a
 * "Best match" row (D-08); a federated policy hit stays reachable and keyboard-navigable.
 *
 * Run: pnpm exec playwright test e2e/palette-search-navigate.spec.ts --project=chromium
 */
test.describe('Command palette search navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(installOperatorShellMock, { initialView: 'dashboard' });
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.keyboard.press('Control+k');
    await expect(page.getByPlaceholder(/search surfaces, commands/i)).toBeVisible();
  });

  test('fmt makes the CLI verb the Best match and keeps the policy reachable', async ({ page }) => {
    await page.getByPlaceholder(/search surfaces, commands/i).fill('fmt');

    const heading = page.getByText('Best match', { exact: true });
    await expect(heading).toBeVisible({ timeout: 10_000 });
    const firstRow = heading.locator('xpath=ancestor::div[1]/following-sibling::button[1]');
    await expect(firstRow).toContainText('vox fmt');
    await expect(page.getByText('fmt.rust')).toBeVisible();
  });

  test('keyboard navigate on policy row opens policies surface', async ({ page }) => {
    await page.getByPlaceholder(/search surfaces, commands/i).fill('fmt.rust');
    await expect(page.getByText('fmt.rust')).toBeVisible({ timeout: 10_000 });

    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');

    await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=policies');
    await expect(page.getByLabel('Policy tree')).toBeVisible();
  });
});
