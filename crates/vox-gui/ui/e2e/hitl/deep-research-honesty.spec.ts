import { test, expect } from '@playwright/test';
import { installTauriMock } from '../lib/tauriMock';
import { addMockInitScript } from '../lib/tauriMockShared';

test.describe('Deep Research Dual Lanes, Engine Drawer & Honesty E2E', () => {
  test.beforeEach(async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'research');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);
  });

  test('exercises lane switcher, provider quota badges, and engine drawer z-50 hierarchy', async ({ page }) => {
    // 1. Verify Lane Switcher starts on Fast Lane
    const fastBtn = page.getByTestId('lane-switch-fast');
    const deepBtn = page.getByTestId('lane-switch-deep');
    await expect(fastBtn).toBeVisible({ timeout: 10_000 });
    await expect(deepBtn).toBeVisible();
    await expect(fastBtn).toHaveAttribute('aria-selected', 'true');
    await expect(deepBtn).toHaveAttribute('aria-selected', 'false');

    // 2. Switch to Deep Lane
    await deepBtn.click();
    await expect(deepBtn).toHaveAttribute('aria-selected', 'true');
    await expect(fastBtn).toHaveAttribute('aria-selected', 'false');

    // 3. Switch back to Fast Lane
    await fastBtn.click();
    await expect(fastBtn).toHaveAttribute('aria-selected', 'true');

    // 4. Verify Provider Quota Badge Strip
    const badgeStrip = page.getByTestId('provider-badge-strip');
    await expect(badgeStrip).toBeVisible();
    await expect(badgeStrip).toContainText('Wikipedia');
    await expect(badgeStrip).toContainText('Tavily');

    // 5. Open Research Engine Drawer via Configure button
    const configureBtn = page.getByTestId('configure-engines-btn');
    await expect(configureBtn).toBeVisible();
    await configureBtn.click();

    // 6. Verify Drawer Overlay and Z-Index Hierarchy (z-50)
    const drawerOverlay = page.getByTestId('research-engine-drawer-overlay');
    await expect(drawerOverlay).toBeVisible({ timeout: 5_000 });
    const zIndex = await drawerOverlay.evaluate((el) => window.getComputedStyle(el).zIndex);
    expect(Number(zIndex)).toBeGreaterThanOrEqual(50);

    // 7. Verify Zero-Key Guarantee banner and Free API Key acquisition links
    const zeroKeyBanner = page.getByTestId('zero-key-guarantee');
    await expect(zeroKeyBanner).toBeVisible();
    await expect(zeroKeyBanner).toContainText('Zero-Key Guarantee');

    const claimLink = page.getByRole('link', { name: /Claim Free Key/i }).first();
    await expect(claimLink).toBeVisible();
    await expect(claimLink).toHaveAttribute('target', '_blank');
    await expect(claimLink).toHaveAttribute('rel', 'noopener noreferrer');

    // 8. Close Drawer via Escape keypress
    await page.keyboard.press('Escape');
    await expect(drawerOverlay).not.toBeVisible();
  });
});
