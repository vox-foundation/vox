import { test, expect } from '@playwright/test';
import { installOperatorShellMock } from './lib/operatorShellMock';

/**
 * Chat session list collapse/expand. Session switching moved out of the Chat
 * surface's ChatSessionRail into the global sidebar's Chat section (9fb541f50),
 * which is expandable only in the 'wide' sidebar mode.
 *
 * Run: pnpm exec playwright test e2e/chat-session-rail.spec.ts --project=chromium
 */
test.describe('Chat session rail', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(installOperatorShellMock, { initialView: 'chat' });
    // useLocalStorage JSON-parses its value, so the mode must be stored as JSON.
    await page.addInitScript(() => localStorage.setItem('vox_sidebar_mode', JSON.stringify('wide')));
    await page.setViewportSize({ width: 1400, height: 900 });
  });

  test('session rail collapses and expands', async ({ page }) => {
    await page.goto('/#view=chat');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const sidebar = page.locator('aside').first();
    const session = sidebar.getByRole('tab', { name: /Mock chat/i });
    await expect(session).toBeVisible();

    await sidebar.getByRole('button', { name: 'Collapse Chat' }).click();
    await expect(session).toHaveCount(0);
    await expect(sidebar.getByRole('button', { name: 'Expand Chat' })).toBeVisible();

    await sidebar.getByRole('button', { name: 'Expand Chat' }).click();
    await expect(session).toBeVisible();
  });
});
