import { test, expect } from '@playwright/test';

test('mood form requires score', async ({ page }) => {
    await page.goto('/mood');
    await page.click('button[type=submit]');
    await expect(page.locator('[role=alert]').first()).toContainText('required');
});

test('mood form submits and redirects', async ({ page }) => {
    await page.goto('/mood');
    await page.fill('input[type=number]', '7');
    await page.fill('textarea, input[type=text]', 'feeling decent');
    await page.click('button[type=submit]');
    await expect(page).toHaveURL(/\/timeline/);
});
