/**
 * Research trace panel under a sync chat reply, driven by RECORDED live daemon
 * replies (e2e/fixtures/research-trace/*.json — each names its source run).
 * The two `.derived.json` fixtures are real runs with one labelled edit each:
 * tavily swapped to budget_exhausted (no live run exhausted the budget), and a
 * deep run given the provider table deep traces only carry since Task 9.
 * Screenshots taken from a derived fixture carry a `-derived` filename suffix.
 *
 * The base tauriMock is extended with a `chat_turn` handler returning the
 * fixture's reply, so the real App -> buildChatTurn -> chat_turn -> transcript
 * path renders the `research_trace` event (events[0]).
 * Screenshots land in review-bundle/latest/ (dark = default theme, light =
 * travertine).
 */
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'review-bundle', 'latest');

type Fixture = {
  user: string;
  reply: { content: string; model_used: string; latency_ms: string; selection_reason: string; events: unknown[] };
};
const load = (name: string): Fixture =>
  JSON.parse(readFileSync(join(HERE, 'fixtures', 'research-trace', `${name}.json`), 'utf8'));
const quickOk = load('quick-ok');
const quickErrors = load('quick-provider-errors');
const budget = load('quick-budget-exhausted.derived');
const deepOk = load('deep-ok');
const deepProviders = load('deep-providers.derived');
const deepFailed = load('deep-failed');
const noResearch = load('no-research');

/** Self-contained (serialized into the page): wraps the base invoke for `chat_turn`. */
function installChatTurnReply(reply: Fixture['reply']): void {
  const internals = (window as any).__TAURI_INTERNALS__;
  const base = internals.invoke;
  internals.invoke = async (cmd: string, args?: any) => {
    if (cmd === 'chat_turn') {
      (window as any).__TAURI_CALLS__?.push({ cmd, args });
      return {
        id: 9001,
        role: 'assistant',
        content: reply.content,
        created_at: new Date().toISOString(),
        task_id: null,
        model_id: reply.model_used,
        latency_ms: Number(reply.latency_ms),
        selection_reason: reply.selection_reason,
        events: reply.events,
      };
    }
    return base(cmd, args);
  };
}

async function sendTurn(page: Page, fx: Fixture, theme: 'dark' | 'light') {
  await addMockInitScript(page, installTauriMock, 'chat');
  await page.addInitScript(installChatTurnReply, fx.reply);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
  if (theme === 'light') {
    await page.evaluate(() => { document.documentElement.dataset.theme = 'travertine'; });
  }
  const composer = page.getByLabel('Task composer');
  await composer.fill(fx.user);
  await composer.press('Enter');
  const turn = await page.evaluate(() =>
    (window as any).__TAURI_CALLS__.find((c: any) => c.cmd === 'chat_turn')?.args?.input,
  );
  return turn;
}

async function shot(page: Page, name: string) {
  mkdirSync(OUT_DIR, { recursive: true });
  const panel = page.getByTestId('research-trace');
  if (await panel.count()) await panel.scrollIntoViewIfNeeded();
  await page.screenshot({ path: join(OUT_DIR, `${name}.png`) });
}

async function expandedPanel(page: Page) {
  const panel = page.getByTestId('research-trace');
  await expect(panel).toBeVisible();
  await page.getByTestId('research-trace-toggle').click();
  await expect(page.getByTestId('research-trace-toggle')).toHaveAttribute('aria-expanded', 'true');
  return panel;
}

for (const theme of ['dark', 'light'] as const) {
  test.describe(`research trace panel (${theme})`, () => {
    test('quick research, all providers ok', async ({ page }) => {
      const turn = await sendTurn(page, quickOk, theme);
      expect(turn.execution).toBe('sync');
      const panel = await expandedPanel(page);
      await expect(page.getByTestId('research-trace-toggle'))
        .toHaveText(/Quick research · 8 sources · google\/gemini-3\.8-flash · 11\.0s · ok/);
      await expect(panel.getByTestId('research-provider')).toHaveCount(5);
      await expect(panel.getByTestId('research-tavily-credits')).toHaveText(/1\/50 used · 49 left/);
      await expect(panel.getByTestId('research-source-5'))
        .toHaveAttribute('href', 'https://openrouter.ai/google/gemini-3.8-flash');
      await shot(page, `research-trace-quick-ok-${theme}`);
    });

    test('quick research with provider errors and an exhausted Tavily budget', async ({ page }) => {
      await sendTurn(page, budget, theme);
      const panel = await expandedPanel(page);
      await expect(panel.locator('[data-provider="tavily"]')).toHaveText(/budget exhausted/);
      await expect(panel.locator('[data-provider="openalex"]')).toHaveText(/429 Too Many Requests/);
      await expect(panel.locator('[data-provider="searxng"]')).toHaveText(/circuit open/);
      await expect(panel.getByTestId('research-tavily-credits')).toHaveText(/50\/50 used · 0 left/);
      // m6: no viewport-height decorative ring inside the scrolling transcript
      // (its bottom edge used to cut through a tall expanded panel).
      await expect(page.getByRole('log', { name: 'Chat transcript' }).locator(':scope > .ring-inset'))
        .toHaveCount(0);
      await shot(page, `research-trace-budget-exhausted-derived-${theme}`);
    });

    test('deep research, slash command stays on the sync path', async ({ page }) => {
      const turn = await sendTurn(page, deepOk, theme);
      expect(turn.execution).toBe('sync');
      expect(turn.content).toBe(deepOk.user);
      await expandedPanel(page);
      await expect(page.getByTestId('research-trace-toggle')).toHaveText(/Deep research · 40 sources/);
      await expect(page.getByTestId('research-stage-claims')).toBeVisible();
      await expect(page.getByTestId('research-stage-citation_audit')).toHaveAttribute('data-status', 'degraded');
      await shot(page, `research-trace-deep-${theme}`);
    });

    test('deep research shows the per-provider table and Tavily credits', async ({ page }) => {
      await sendTurn(page, deepProviders, theme);
      const panel = await expandedPanel(page);
      const retrieval = panel.getByTestId('research-stage-retrieval');
      await expect(retrieval.getByTestId('research-provider')).toHaveCount(5);
      await expect(retrieval.locator('[data-provider="tavily"]')).toHaveText(/20 hits.*×4 calls/);
      await expect(retrieval.getByTestId('research-tavily-credits')).toHaveText(/4\/50 used · 46 left/);
      await shot(page, `research-trace-deep-providers-derived-${theme}`);
    });

    test('deep research pipeline failure is visible', async ({ page }) => {
      await sendTurn(page, deepFailed, theme);
      await expandedPanel(page);
      await expect(page.getByTestId('research-trace')).toHaveAttribute('data-status', 'failed');
      await expect(page.getByTestId('research-stage-deep_pipeline')).toHaveText(/No API key available/);
      await shot(page, `research-trace-deep-failed-${theme}`);
    });

    test('non-research turn shows the compact "No research" detection line', async ({ page }) => {
      await sendTurn(page, noResearch, theme);
      await expect(page.locator('[id^="msg-"]').filter({ hasText: noResearch.reply.content.slice(0, 20) }))
        .toBeVisible();
      await expect(page.getByTestId('research-trace')).toHaveAttribute('data-mode', 'none');
      await expect(page.getByTestId('research-trace-toggle'))
        .toHaveText(/No research · skip: greeting \/ small talk/);
      await shot(page, `research-trace-none-${theme}`);
    });
  });
}

test('provider-error fixture (real run) renders error / circuit_open / not_configured', async ({ page }) => {
  await sendTurn(page, quickErrors, 'dark');
  const panel = await expandedPanel(page);
  await expect(panel.locator('[data-provider="tavily"]')).toHaveText(/not configured/);
  await expect(panel.locator('[data-provider="openalex"]')).toHaveAttribute('data-state', 'error');
});
