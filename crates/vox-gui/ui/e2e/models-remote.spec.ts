import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Models surface — remote mode', () => {
  test('stays gated while the models module is still db-class', async ({ page }) => {
    await installRemoteMode(page, 'models', { get_active_model: 'openai/gpt-x' }, ['models'], {
      models: 'not_remote_capable',
    });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toBeVisible();
    await page.screenshot({ path: `${SHOT_DIR}/models-remote-gated.png`, fullPage: true });
  });
});
