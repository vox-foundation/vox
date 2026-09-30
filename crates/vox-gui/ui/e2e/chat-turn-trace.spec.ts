import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'review-bundle', 'latest');
const CONTRACT = JSON.parse(
  readFileSync(join(HERE, '../../../../contracts/gui/turn-event-kinds.v1.json'), 'utf8'),
) as { kinds: Array<{ kind: string; example: Record<string, unknown> }> };

function example(kind: string): Record<string, unknown> {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return entry.example;
}

const ROUTING = example('routing_decision');

/**
 * Same shape as `installTrustOverrides` in chat-trust-chips.spec.ts (Playwright does not let one
 * spec import another): answer `chat_turn` with `reply`, fall through to the base mock otherwise.
 */
async function installChatTurn(page: Page, reply: unknown): Promise<void> {
  await addMockInitScript(page, installTauriMock, 'chat');
  await page.addInitScript((chatTurn: unknown) => {
    const internals = (window as any).__TAURI_INTERNALS__;
    const base: ((cmd: string, args?: any) => Promise<unknown>) | undefined = internals?.invoke;
    if (typeof base !== 'function') {
      throw new Error('installChatTurn must run after installTauriMock');
    }
    internals.invoke = async (cmd: string, args?: any) => {
      if (cmd === 'chat_turn') {
        (window as any).__VOX_IPC_ACTIVE_COUNT__ = ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 0) + 1;
        (window as any).__TAURI_CALLS__.push({ cmd, args: args ?? null });
        try {
          return chatTurn;
        } finally {
          (window as any).__VOX_IPC_ACTIVE_COUNT__ = Math.max(
            0,
            ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 1) - 1,
          );
        }
      }
      return base(cmd, args);
    };
  }, reply);
}

function reply(id: number, events: unknown[]) {
  return {
    id,
    role: 'assistant',
    content: 'Checked the repository and delegated the migration.',
    created_at: '2026-09-28T12:00:00.000Z',
    task_id: null,
    model_id: ROUTING.resolved_id,
    latency_ms: 4100,
    events,
  };
}

async function sendTurn(page: Page, prompt: string): Promise<void> {
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
  const chooseModeBtn = page.getByLabel('Choose send mode');
  if (await chooseModeBtn.isVisible()) {
    if ((await chooseModeBtn.getAttribute('aria-expanded')) !== 'true') {
      await chooseModeBtn.click();
    }
    const quickChatBtn = page.getByLabel('Set send mode: Quick chat');
    if (await quickChatBtn.isVisible()) {
      await quickChatBtn.click();
    }
  }
  const composer = page.getByLabel('Task composer');
  await composer.fill(prompt);
  await composer.press('Enter');
  await expect(page.getByTestId('chat-trace-summary')).toBeVisible();
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
}

test('a clean turn shows one collapsed trace row that expands into its steps', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await installChatTurn(
    page,
    reply(9101, [
      example('routing_decision'),
      example('tool_receipt'),
      example('delegation_spawned'),
      example('research_milestone'),
    ]),
  );
  await sendTurn(page, 'Check the repo and delegate the migration');

  const summary = page.getByTestId('chat-trace-summary');
  await expect(summary).toContainText(String(ROUTING.resolved_id));
  await expect(summary).toContainText('1 tool');
  await expect(summary).toContainText('4.1s');
  await expect(summary).toHaveAttribute('aria-expanded', 'false');
  await expect(page.getByTestId('chat-trace-steps')).toBeHidden();
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'chat-trace-collapsed.png') });

  await summary.click();
  await expect(summary).toHaveAttribute('aria-expanded', 'true');
  await expect(page.getByTestId('chat-trace-step')).toHaveCount(4);
  await expect(page.getByTestId('chat-turn-delegation-row')).toBeVisible();
  await expect(page.getByTestId('chat-turn-research-row')).toBeVisible();
  await expect(page.getByTestId('chat-turn-routing-row')).toHaveAttribute('data-resolved-from', 'catalog');
  await page.screenshot({ path: join(OUT_DIR, 'chat-trace-expanded.png') });
});

test('a fabricated claim is an inline interrupt and opens the trace', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await installChatTurn(
    page,
    reply(9102, [example('routing_decision'), example('tool_receipt'), example('receipt_claims')]),
  );
  await sendTurn(page, 'Verify the task claims');

  const claims = page.getByTestId('chat-turn-claims-row');
  await expect(claims).toHaveCount(1);
  await expect(claims).toHaveAttribute('data-flagged', 'true');
  await expect(page.getByTestId('chat-trace-summary')).toHaveAttribute('aria-expanded', 'true');
  await expect(page.getByTestId('chat-trace-steps')).toBeVisible();
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, 'chat-trace-interrupt.png') });
});

test('Verbose opens a clean trace and Quiet closes it, persisting the choice', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await installChatTurn(page, reply(9103, [example('routing_decision'), example('tool_receipt')]));
  await sendTurn(page, 'Check the repo status');

  await expect(page.getByTestId('chat-trace-steps')).toBeHidden();
  await page.getByRole('radio', { name: 'Verbose' }).click();
  await expect(page.getByTestId('chat-trace-steps')).toBeVisible();
  await page.getByRole('radio', { name: 'Quiet' }).click();
  await expect(page.getByTestId('chat-trace-steps')).toBeHidden();
  await expect
    .poll(() => page.evaluate(() => window.localStorage.getItem('gui.chat.verbosity.v1')))
    .toBe('"quiet"');
});
