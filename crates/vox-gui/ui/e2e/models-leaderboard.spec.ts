import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

const MODES = ['free', 'efficiency', 'balanced', 'genius'];
const CATEGORIES = ['chat', 'codegen', 'planning', 'research', 'tool_orchestration', 'parsing'];

function leaderboard(newcomerAdopted: boolean) {
  const routes = MODES.flatMap(mode =>
    CATEGORIES.map(category => {
      if (mode === 'free') {
        return { mode, category, model_id: 'meta-llama/llama-3.3-70b-instruct:free', only_candidate: category === 'tool_orchestration', candidates: 3, quality: 0.55, quality_source: 'measured', bench_cases: 14 };
      }
      if (category === 'chat' && mode !== 'genius') {
        return newcomerAdopted
          ? { mode, category, model_id: 'acme/newcomer-2', only_candidate: false, candidates: 9, quality: 0.93, quality_source: 'measured', bench_cases: 17 }
          : { mode, category, model_id: 'openai/gpt-4.1-mini', only_candidate: false, candidates: 8, quality: 0.78, quality_source: 'benchmark', bench_cases: 0 };
      }
      if (mode === 'genius') {
        return { mode, category, model_id: 'anthropic/claude-sonnet-4.5', only_candidate: false, candidates: 5, quality: 0.95, quality_source: 'benchmark', bench_cases: 0 };
      }
      return { mode, category, model_id: 'deepseek/deepseek-chat-v3.1', only_candidate: false, candidates: 7, quality: 0.84, quality_source: 'measured', bench_cases: 16 };
    }),
  );
  const bench = [
    { model_id: 'deepseek/deepseek-chat-v3.1', served_as: null, cases: 16, passed: 14, mean_score: 0.88, error_rate: 0, p50_ttft_ms: 620, p95_latency_ms: 5200, tokens_per_sec: 41.2, cost_per_solve_usd: 0.0009, tokens_per_solve: 240, confidence: 'confirmed', last_run_ms: 1 },
    { model_id: 'meta-llama/llama-3.3-70b-instruct:free', served_as: null, cases: 14, passed: 9, mean_score: 0.66, error_rate: 0.07, p50_ttft_ms: 900, p95_latency_ms: 8800, tokens_per_sec: 25.3, cost_per_solve_usd: 0, tokens_per_solve: 310, confidence: 'confirmed', last_run_ms: 1 },
    { model_id: 'acme/broken-preview', served_as: null, cases: 12, passed: 1, mean_score: 0.12, error_rate: 0.5, p50_ttft_ms: null, p95_latency_ms: null, tokens_per_sec: null, cost_per_solve_usd: 0.04, tokens_per_solve: 3000, confidence: 'shadowed', last_run_ms: 1 },
  ];
  if (newcomerAdopted) {
    bench.unshift({ model_id: 'acme/newcomer-2', served_as: 'acme/newcomer-2-20261001', cases: 17, passed: 16, mean_score: 0.94, error_rate: 0, p50_ttft_ms: 280, p95_latency_ms: 2100, tokens_per_sec: 96.4, cost_per_solve_usd: 0.0011, tokens_per_solve: 180, confidence: 'confirmed', last_run_ms: 2 });
  }
  return { routes, bench, window_days: 30 };
}

async function installLeaderboard(page: Page, board: unknown): Promise<void> {
  await addMockInitScript(page, installTauriMock, 'models');
  await page.addInitScript((payload: unknown) => {
    const internals = (window as any).__TAURI_INTERNALS__;
    const base = internals.invoke;
    internals.invoke = async (cmd: string, args?: unknown) =>
      cmd === 'get_model_leaderboard' ? payload : base(cmd, args);
  }, board);
}

test.describe('Models leaderboard', () => {
  test.beforeAll(() => mkdirSync(OUT_DIR, { recursive: true }));

  test('routing chips show each mode pick with measured provenance', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await installLeaderboard(page, leaderboard(false));
    await page.goto('/');
    const grid = page.getByRole('table', { name: 'Routing by mode' });
    await expect(grid).toBeVisible();
    await expect(grid.getByLabel(/balanced chat routes to openai\/gpt-4\.1-mini, public index/)).toBeVisible();
    await expect(grid.getByLabel(/balanced codegen routes to deepseek\/deepseek-chat-v3\.1, measured on 16 cases/)).toBeVisible();
    await expect(grid.getByLabel(/free tool_orchestration .*outside the mode's tiers/)).toBeVisible();
    const bench = page.getByRole('table', { name: 'Benchmark results' });
    await expect(bench.getByRole('row', { name: 'acme/broken-preview' })).toContainText('shadowed');
    await grid.scrollIntoViewIfNeeded();
    await page.screenshot({ path: join(OUT_DIR, 'models-leaderboard-1440.png'), fullPage: true });
  });

  test('a benchmarked newcomer takes over chat routing', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await installLeaderboard(page, leaderboard(true));
    await page.goto('/');
    const grid = page.getByRole('table', { name: 'Routing by mode' });
    for (const mode of ['efficiency', 'balanced']) {
      await expect(grid.getByLabel(new RegExp(`${mode} chat routes to acme/newcomer-2, measured on 17 cases`))).toBeVisible();
    }
    const row = page.getByRole('table', { name: 'Benchmark results' }).getByRole('row', { name: 'acme/newcomer-2' });
    await expect(row).toContainText('16/17');
    await expect(row).toContainText('served as acme/newcomer-2-20261001');
    await page.screenshot({ path: join(OUT_DIR, 'models-leaderboard-newcomer-1440.png'), fullPage: true });
  });

  test('narrow viewport keeps the tables scrollable instead of overlapping', async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 900 });
    await installLeaderboard(page, leaderboard(true));
    await page.goto('/');
    const grid = page.getByRole('table', { name: 'Routing by mode' });
    await expect(grid).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    await page.screenshot({ path: join(OUT_DIR, 'models-leaderboard-900.png'), fullPage: true });
  });
});
