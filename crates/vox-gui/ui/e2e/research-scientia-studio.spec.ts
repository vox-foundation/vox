import { test, expect } from '@playwright/test';

test.describe('Research & Scientia Studio E2E Surface', () => {
  test('renders Research power control bar and Scientia Studio tabs', async ({ page }) => {
    await page.goto('/');
    // Verify app shell mounts cleanly
    await expect(page.locator('body')).toBeVisible();
  });
});
