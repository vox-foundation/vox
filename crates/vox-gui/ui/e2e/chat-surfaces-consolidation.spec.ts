import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';
import { addInvokeOverrides, type InvokeOverrides } from './lib/invokeOverrides';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

const SESSION = [
  { session_id: 'surfaces-session', title: 'Surfaces', updated_at: 'now', message_count: 0, conversation_id: 1 },
];

async function openChat(page: Page, overrides: InvokeOverrides): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await addMockInitScript(page, installTauriMock, 'chat');
  await addInvokeOverrides(page, {
    ...overrides,
    responses: { chat_list_sessions: SESSION, ...(overrides.responses ?? {}) },
  });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
  const sessionTab = page.getByRole('tab', { name: /Surfaces/i });
  if (await sessionTab.isVisible()) await sessionTab.click();
}

test('the rail shows this session: next-turn Routing from the contract-family mock, lock chip, no roster, no global resources', async ({ page }) => {
  await openChat(page, {
    responses: {
      list_orchestrator_tasks: [
        {
          id: 77,
          description: 'Migrate orders table',
          priority: 'normal',
          lifecycle: 'in_progress',
          agent_id: 3,
          session_id: 'surfaces-session',
          estimated_complexity: 1,
          depends_on: [],
          write_files: [],
          remote_node: null,
          origin: 'orchestrator',
        },
      ],
      activity_query: [
        {
          id: 10,
          ts_ms: 1,
          agent_id: '3',
          session_id: 'surfaces-session',
          kind: 'LockAcquired',
          summary: 'Lock acquired',
          detail_json: JSON.stringify({
            type: 'lock_acquired',
            agent_id: 3,
            path: 'db://orders/42',
            exclusive: true,
            session_id: 'surfaces-session',
            task_id: 77,
          }),
        },
      ],
    },
  });

  const routing = page.getByRole('region', { name: 'Routing' });
  // Seam: App's one routing query answers from tauriMock (Task 8): family-keyed, resolved_from 'bootstrap'.
  await expect(routing.getByTestId('execution-rail-routing-scope')).toHaveText('next turn');
  await expect(routing.getByTestId('execution-rail-routing')).toHaveText(
    'Routes to anthropic/claude-sonnet (offline) — lowest cost that fits the mode',
  );
  await routing.getByText('Why this model').click();
  await expect(routing.getByTestId('execution-rail-routing-state')).toHaveAttribute('title', /eligible for routing/);
  await expect(routing).toContainText('Alternatives: anthropic/claude-haiku, deepseek/deepseek-flash');
  await expect(page.getByRole('region', { name: /agent shards/i })).toHaveCount(0);
  await expect(page.getByLabel('Resource strip')).toHaveCount(0);
  await expect(
    page.getByRole('region', { name: /active tasks/i }).getByTestId('execution-rail-lock-chip'),
  ).toContainText('holding db://orders/42');
  await expect(page.getByTestId('bottom-status-bar-routing')).toContainText('Auto → anthropic/claude-sonnet (offline)');

  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'chat-rail-routing.png') });
});

test('the composer names modes in full, keeps Check replies under Risk, and repeats no global spend', async ({ page }) => {
  await openChat(page, {});

  for (const name of ['Free', 'Efficient', 'Balanced', 'Genius']) {
    await expect(page.getByRole('radio', { name, exact: true })).toBeVisible();
  }
  await page.getByRole('radio', { name: 'Efficient', exact: true }).hover();
  await expect(page.getByTestId('drive-mode-hint')).toContainText('Most out of the tokens you spend');
  await expect(page.getByText(/session \$/)).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Run (Enter)' })).toContainText('↵');
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'composer-modes.png') });

  await page.getByRole('button', { name: /^Risk: / }).click();
  const risk = page.getByRole('dialog', { name: /acceptable risk/i });
  await expect(risk.getByRole('button', { name: /check replies (on|off)/i })).toBeVisible();
  await expect(risk).not.toContainText(/grounding/i);
  await expect(page.getByRole('button', { name: /grounding check/i })).toHaveCount(0);
  await page.screenshot({ path: join(OUT_DIR, 'composer-risk-check-replies.png') });
});

test('the research popover shows only reported provider state', async ({ page }) => {
  await openChat(page, {});

  await page.getByTestId('status-bar-cluster-trigger').click();
  const popover = page.getByTestId('status-bar-cluster-popover');
  await expect(popover.getByTestId('status-bar-cluster-provider-wikipedia')).toContainText('on');
  await expect(popover).not.toContainText('✓');
  await expect(popover).not.toContainText('Online');
  await expect(popover).not.toContainText('Plaintext');
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'research-popover-honest.png') });
});

test('a failed research status reads "unknown" with no green checks', async ({ page }) => {
  await openChat(page, { reject: ['get_research_engine_status'] });

  await page.getByTestId('status-bar-cluster-trigger').click();
  await expect(page.getByTestId('status-bar-cluster-unknown')).toBeVisible();
  await expect(page.getByTestId('status-bar-cluster-trigger')).toContainText('unknown');
  await expect(page.getByTestId('status-bar-cluster-popover')).not.toContainText('✓');
});
