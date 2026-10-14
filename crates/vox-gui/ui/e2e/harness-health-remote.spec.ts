import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Harness Health surface — remote mode', () => {
  test('reads eval history and regressions from the server', async ({ page }) => {
    await installRemoteMode(
      page,
      'harness-health',
      { harness_eval_history: [], harness_eval_regressions: [] },
      ['harness-health'],
    );
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.getByRole('heading', { level: 2, name: 'No harness eval runs yet' })).toBeVisible();
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toContain('harness_eval_history');
    await page.screenshot({ path: `${SHOT_DIR}/harness-health-remote.png`, fullPage: true });
  });
});
