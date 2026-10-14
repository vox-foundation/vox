import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Chat surface — remote mode', () => {
  test('lists sessions served by the remote daemon', async ({ page }) => {
    await installRemoteMode(
      page,
      'chat',
      {
        chat_list_sessions: [
          {
            session_id: 's1',
            title: 'Remote session',
            updated_at: '2026-10-08T00:00:00Z',
            message_count: 0,
            conversation_id: 1,
          },
        ],
        chat_get_messages: [],
      },
      ['chat'],
    );
    // Session switching lives in the sidebar's Chat section, expandable only in 'wide' mode.
    await page.addInitScript(() => localStorage.setItem('vox_sidebar_mode', JSON.stringify('wide')));
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.locator('aside').first().getByRole('tab', { name: /Remote session/i })).toBeVisible();
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toContain('chat_list_sessions');
    await page.screenshot({ path: `${SHOT_DIR}/chat-remote.png`, fullPage: true });
  });
});
