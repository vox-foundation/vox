/**
 * Session rail rename/archive flows (Phase 2 wiring) against the stateful
 * tauriMock: outgoing IPC contract + product-rendered rail state after the
 * handler's loadSessions() refetch of the stateful mock. The session list
 * lives in the global sidebar's Chat section ('wide' mode) since 9fb541f50.
 */
import { test, expect } from '@playwright/test';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

test.describe('Session rail actions', () => {
  test.beforeEach(async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'chat');
    // useLocalStorage JSON-parses its value, so the mode must be stored as JSON.
    await page.addInitScript(() => localStorage.setItem('vox_sidebar_mode', JSON.stringify('wide')));
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.locator('aside').first().getByRole('tab', { name: /Mock chat/i })).toBeVisible();
  });

  test('rename flows through chat_rename_session and re-renders the new title', async ({ page }) => {
    // Rename is inline: double-click the row to swap in an input, Enter commits.
    await page.getByRole('tab', { name: /Mock chat/i }).dblclick();
    const input = page.locator('aside').first().getByRole('textbox');
    await input.fill('Renamed chat');
    await input.press('Enter');

    await expect
      .poll(
        () =>
          page.evaluate(() =>
            (window as any).__TAURI_CALLS__.filter((c: any) => c.cmd === 'chat_rename_session').length,
          ),
        { timeout: 10_000 },
      )
      .toBe(1);
    const call = await page.evaluate(() =>
      (window as any).__TAURI_CALLS__.find((c: any) => c.cmd === 'chat_rename_session'),
    );
    expect(call.args).toMatchObject({ sessionId: 'mock-session-1', title: 'Renamed chat' });
    await expect(page.getByRole('tab', { name: /Renamed chat/i })).toBeVisible();
  });

  test('archive flows through chat_archive_session and removes the session tab', async ({ page }) => {
    const row = page.getByRole('tab', { name: /Mock chat/i });
    await row.hover();
    await row.getByRole('button', { name: 'Archive' }).click();

    await expect
      .poll(
        () =>
          page.evaluate(() =>
            (window as any).__TAURI_CALLS__.filter((c: any) => c.cmd === 'chat_archive_session').length,
          ),
        { timeout: 10_000 },
      )
      .toBe(1);
    const call = await page.evaluate(() =>
      (window as any).__TAURI_CALLS__.find((c: any) => c.cmd === 'chat_archive_session'),
    );
    expect(call.args).toMatchObject({ sessionId: 'mock-session-1' });
    await expect(page.getByRole('tab', { name: /Mock chat/i })).toHaveCount(0);
  });
});
