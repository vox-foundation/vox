import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from '../lib/tauriMock';
import { addMockInitScript } from '../lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'review-bundle', 'latest');

test.describe('Research visual debugger stepper', () => {
  test('walks through diagnostic prober and inspector drawer', async ({ page }) => {
    // Sets up page with addMockInitScript(page, installTauriMock, 'research')
    await addMockInitScript(page, installTauriMock, 'research');
    await page.setViewportSize({ width: 1440, height: 900 });

    // Navigates to /
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);

    // Toggles Diagnostic Prober (getByTestId('toggle-diagnostics-btn'))
    const toggleDiagnosticsBtn = page.getByTestId('toggle-diagnostics-btn');
    await expect(toggleDiagnosticsBtn).toBeVisible({ timeout: 10_000 });
    await toggleDiagnosticsBtn.click();

    // Asserts getByTestId('live-source-prober') is visible
    const prober = page.getByTestId('live-source-prober');
    await expect(prober).toBeVisible();

    // Populate probe query so the submit button is enabled
    const probeInput = page.getByTestId('probe-query-input');
    const existingQuery = await probeInput.inputValue();
    if (!existingQuery.trim()) {
      await probeInput.fill('hybrid search architecture');
    }

    // Clicks probe button (getByTestId('probe-submit-btn'))
    const probeSubmitBtn = page.getByTestId('probe-submit-btn');
    await expect(probeSubmitBtn).toBeVisible();
    await probeSubmitBtn.click();

    // Asserts 200 OK badge and hit counts render
    const statusBadge = page.getByTestId('status-badge-200').first();
    await expect(statusBadge).toBeVisible({ timeout: 10_000 });
    await expect(statusBadge).toContainText('200');

    const hitCountBadge = page.getByTestId('hit-count-badge').first();
    await expect(hitCountBadge).toBeVisible();
    await expect(hitCountBadge).toContainText('hits');

    // Toggles Inspector Drawer via shortcut
    await page.keyboard.press('Control+Shift+D');
    const inspectorDrawer = page.getByTestId('inspector-drawer');
    await expect(inspectorDrawer).toBeVisible({ timeout: 5_000 });

    // Wait for IPC idle before capturing screenshot
    await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);

    // Captures viewport screenshot and saves to review-bundle/latest/research-debugger-stepper.png
    mkdirSync(OUT_DIR, { recursive: true });
    await page.screenshot({
      path: join(OUT_DIR, 'research-debugger-stepper.png'),
    });
  });
});
