import { test, expect } from '@playwright/test';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

test.describe('Vox Dashboard', () => {
  test('should load the dashboard and verify event payload delivery', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.goto('/');

    // Check if the dashboard loads
    await expect(page).toHaveTitle(/Axis|frontend|vox/i);

    // Verify one of the expected UI shells rendered.
    const hasDashboard = (await page.getByText('Dashboard').count()) > 0;
    const hasBakeOff = (await page.getByText('Vox bake-off — Path A (Tauri-mobile)').count()) > 0;
    expect(hasDashboard || hasBakeOff).toBeTruthy();

    // Non-fixture execution path when the full shell is active.
    if (hasDashboard) {
      const activeSurface = page.getByTestId('active-surface');
      await expect(activeSurface).toHaveAttribute('data-view', 'dashboard');

      await page.locator('aside').first().getByRole('button', { name: /^Workspace/ }).click();
      await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=console');
      await expect(activeSurface).toHaveAttribute('data-view', 'console');

      await page.goto('/#view=repository');
      await page.waitForSelector('nav', { timeout: 15_000 });
      await expect(activeSurface).toHaveAttribute('data-view', 'repository');
      await page.getByRole('button', { name: 'Workspace status' }).click();
      // The mock's `vox status` run returns stdout 'ok'; assert it in the command-output pane, not anywhere on the page.
      await expect(page.getByLabel('Command output')).toHaveText('ok');

      // Harness deep link renders the Axis Harness River HUD (HarnessRedirect), which keeps the composer-focus action.
      await page.goto('/#view=harness');
      await page.waitForSelector('nav', { timeout: 15_000 });
      await expect(page.getByText('AXIS HARNESS RIVER')).toBeVisible();
      await expect(page.getByTestId('harness-waveform-tier-strip')).toBeVisible();
      await page.getByRole('button', { name: 'Focus composer' }).click();

      // Runs view under Review parent nav.
      await page.locator('aside').first().getByRole('button', { name: /^Review/ }).click();
      await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=approvals');
      await page.goto('/#view=runs');
      await page.waitForSelector('nav', { timeout: 15_000 });
      await expect(page.getByText('gui-run-1').first()).toBeVisible();
    }
  });
});
