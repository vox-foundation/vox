import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Tasks surface — remote mode', () => {
  test('lists the server hopper queue', async ({ page }) => {
    await installRemoteMode(
      page,
      'tasks',
      {
        hopper_list: [
          {
            item_id: 'h1',
            intent: 'remote intent',
            priority: 2,
            state: 'inbox',
            task_id: 1,
            session_id: null,
            agent_id: null,
            remote_node: null,
          },
        ],
      },
      ['tasks'],
    );
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.getByText('remote intent').first()).toBeVisible();
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toContain('hopper_list');
    await page.screenshot({ path: `${SHOT_DIR}/tasks-remote.png`, fullPage: true });
  });
});
