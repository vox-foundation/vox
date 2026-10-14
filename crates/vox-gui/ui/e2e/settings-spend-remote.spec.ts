import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Settings spend meter — remote mode', () => {
  test('shows the server-recorded spend while Runtime config stays on this machine', async ({ page }) => {
    await installRemoteMode(
      page,
      'settings',
      {
        get_llm_spend: {
          sessionUsd: 0.5,
          dayUsd: 1.25,
          totalUsd: 9,
          dailyBudgetUsd: 5,
          perSessionBudgetUsd: 1,
        },
      },
      ['settings'],
    );
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await page.getByRole('button', { name: /runtime/i }).click();
    await expect(page.getByTestId('surface-unavailable')).toHaveAttribute('data-reason', 'needs_local_machine');
    const meter = page.getByTestId('llm-spend');
    await expect(meter).toContainText('$1.2500');
    await expect(meter).toContainText('$5.00');
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toContain('get_llm_spend');
    await page.screenshot({ path: `${SHOT_DIR}/settings-spend-remote.png`, fullPage: true });
  });
});
