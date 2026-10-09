import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Models surface — remote mode', () => {
  test('renders cards on a remote server', async ({ page }) => {
    await installRemoteMode(
      page,
      'models',
      {
        get_models_snapshot: {
          cards: [{ id: 'free/model-a', provider: 'openrouter', tier: 'Free', cost_per_1k: 0, max_tokens: 8000, is_free: true, latency_p50_ms: 400 }],
          routing: { decision_preview: null, routing_priority: { efficiency: 20, precision: 20, latency: 20, availability: 20, balance: 10, mobile: 10 } },
          active_model: null,
          providers: [],
        },
        set_active_model: null,
      },
      ['models'],
    );
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.getByText('free/model-a')).toBeVisible();
    await page.screenshot({ path: `${SHOT_DIR}/models-remote.png`, fullPage: true });
  });
});
