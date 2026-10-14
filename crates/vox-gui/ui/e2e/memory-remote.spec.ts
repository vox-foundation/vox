import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Memory surface — remote mode', () => {
  test('browses knowledge nodes and KB health served by the remote daemon', async ({ page }) => {
    await installRemoteMode(
      page,
      'memory',
      {
        list_knowledge_nodes: [
          {
            id: 'doc:remote1',
            label: 'Remote note',
            snippet: 'Ingested on the remote Vox server.',
            node_type: 'document',
            created_at: '2026-10-09 00:00:00',
          },
        ],
        get_memory_status: {
          corpus_counts: { proj: 12, docs: 1, chats: 4, rules: 0, web: 3 },
          shards: [
            { id: 'K-01', depth: 2, entries: 1, hot: false, dirty: false, spark: [] },
            { id: 'M-01', depth: 1, entries: 4, hot: true, dirty: true, spark: [] },
          ],
          recent_recalls: [],
          embedding_dim: 1024,
        },
        get_kb_health: {
          node_count: 1,
          edge_count: 0,
          fts_available: true,
          corpus_counts: { document: 1 },
        },
      },
      ['memory'],
    );
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await page.getByTestId('tab-browse').click();
    await expect(page.getByText('Remote note')).toBeVisible();
    await page.getByTestId('tab-health').click();
    await expect(page.getByText('document').first()).toBeVisible();
    const invoked = await page.evaluate(() =>
      ((window as unknown as { __VOX_INVOKES__: Array<[string, unknown]> }).__VOX_INVOKES__).map(([c]) => c),
    );
    expect(invoked).toEqual(
      expect.arrayContaining(['get_memory_status', 'list_knowledge_nodes', 'get_kb_health']),
    );
    await page.screenshot({ path: `${SHOT_DIR}/memory-remote.png`, fullPage: true });
  });
});
