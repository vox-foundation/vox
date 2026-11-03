import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Research and Scientia surfaces — remote mode', () => {
  test('lists research history and engine status served by the remote daemon', async ({ page }) => {
    await installRemoteMode(
      page,
      'research',
      {
        list_research_sessions: [
          { id: 1, query_text: 'remote query', status: 'completed', started_at_ms: 1, finished_at_ms: 2 },
        ],
        get_research_engine_status: {
          active_lane: 'fast',
          fast_timeout_ms: 1500,
          deep_timeout_ms: 30000,
          providers: [],
          free_key_offers: [],
        },
      },
      ['research'],
    );
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.getByText('remote query').first()).toBeVisible();
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toEqual(expect.arrayContaining(['list_research_sessions', 'get_research_engine_status']));
    await page.screenshot({ path: `${SHOT_DIR}/research-remote.png`, fullPage: true });
  });

  test('renders the Scientia queue dashboard from the remote daemon', async ({ page }) => {
    await installRemoteMode(
      page,
      'scientia',
      {
        scientia_dashboard_snapshot: {
          candidates: {
            total: 7,
            by_class: { finding: 7 },
            top_5_by_confidence: [
              {
                candidate_id: 'cand-remote-1',
                candidate_class: 'finding',
                confidence: 0.91,
                state: 'queued',
                created_at_ms: 1,
                updated_at_ms: 2,
              },
            ],
          },
          claims_pending: { verifiable: 3, abstained: 1, extraction_running: 0 },
          manifests_in_reply_window: [],
          retraction_queue: [],
          stalls: [],
        },
        list_publication_manifests: [],
        scientia_cost_rollup: {
          this_quarter: {
            extraction_usd: 0.5,
            critic_usd: 0.25,
            novelty_retrieval_usd: 0.25,
            scholarly_submission_usd: 0.25,
            total_usd: 1.25,
          },
          per_finding_average_usd: 0.625,
          by_provider: [],
        },
      },
      ['scientia'],
    );
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.getByText('Publication pipeline queue snapshot')).toBeVisible();
    await expect(page.getByText('Candidates').first()).toBeVisible();
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toEqual(expect.arrayContaining(['scientia_dashboard_snapshot', 'scientia_cost_rollup']));
    await page.screenshot({ path: `${SHOT_DIR}/scientia-remote.png`, fullPage: true });
  });
});
