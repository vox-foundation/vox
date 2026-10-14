import { test, expect } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installOperatorShellMock } from './lib/operatorShellMock';
import { addRichMockInitScript } from './lib/tauriMockRich';
import { addInvokeOverrides } from './lib/invokeOverrides';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

/** `acme/…` is fictional: a version-shaped id is needed to prove the bar hides it off-catalog. */
const routingSummary = (resolvedFrom: 'catalog' | 'bootstrap') => ({
  active_model: null,
  exploration_spent_usd: 0,
  exploration_budget_usd: 50,
  routing_priority: { efficiency: 50, precision: 50, latency: 50, availability: 50, balance: 50, mobile: 50 },
  arm_count: 3,
  model_count: 12,
  decision_preview: {
    selected_model: 'acme/widget-flash-20260901',
    discovery_state: 'confirmed',
    alternatives: ['acme/gizmo-pro-20260801'],
    rejection_reasons: [],
    intelligence_score: 0.7,
    efficiency_score: 0.8,
    latency_score: 0.6,
  },
  family: 'acme/widget-flash',
  resolved_from: resolvedFrom,
  reason: 'lowest cost that fits the mode',
});

/**
 * BottomStatusBar visibility on operator surfaces (Phase 1.4).
 *
 * Run: pnpm exec playwright test e2e/status-bar-surfaces.spec.ts --project=chromium
 */
test.describe('BottomStatusBar on operator surfaces', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(installOperatorShellMock, { initialView: 'dashboard' });
  });

  async function expectStatusBarVisible(page: import('@playwright/test').Page) {
    await expect(page.getByTestId('bottom-status-bar')).toBeVisible();
    await expect(page.getByRole('status', { name: /operator status/i })).toBeVisible();
  }

  test('status bar visible on dashboard, chat, policies, and console', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });

    await page.goto('/#view=dashboard');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expectStatusBarVisible(page);

    await page.goto('/#view=chat');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expectStatusBarVisible(page);

    await page.goto('/#view=policies');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expectStatusBarVisible(page);

    await page.goto('/#view=console');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expectStatusBarVisible(page);
  });

  test('status bar visible when navigating via sidebar', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });

    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expectStatusBarVisible(page);

    const sidebarNav = page.getByRole('navigation');
    await sidebarNav.getByRole('button', { name: 'Chat', exact: true }).click();
    await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=chat');
    await expectStatusBarVisible(page);

    await sidebarNav.getByRole('button', { name: 'Workspace', exact: true }).click();
    await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=console');
    await expectStatusBarVisible(page);
  });
});

test.describe('status bar cards (chat-surfaces plan 3a)', () => {
  test('Engine, Spend, Mesh, Routing and Needs you each read one source', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await addRichMockInitScript(page, 'chat');
    await addInvokeOverrides(page, {
      responses: {
        get_routing_summary_live: routingSummary('bootstrap'),
        get_llm_spend: { sessionUsd: 0.25, dayUsd: 1.5, totalUsd: 1.5, dailyBudgetUsd: 50, perSessionBudgetUsd: 10 },
      },
    });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const bar = page.getByTestId('bottom-status-bar');
    await expect(bar.getByTestId('bottom-status-bar-engine')).toContainText('9 agents · 44 queued');
    await expect(bar.getByTestId('bottom-status-bar-spend')).toContainText('$12.34 / $50.00');
    await expect(bar.getByTestId('bottom-status-bar-mesh')).toContainText('2/2 online');
    await expect(bar.getByTestId('bottom-status-bar-routing')).toContainText('Auto → acme/widget-flash (offline)');
    await expect(bar.getByTestId('bottom-status-bar-routing')).not.toContainText('20260901');
    await expect(bar.getByTestId('bottom-status-bar-needs-you')).toContainText(/Needs you\d+/);
    for (const gone of ['agents', 'queue', 'budget', 'model', 'openrouter', 'approvals']) {
      await expect(page.getByTestId(`bottom-status-bar-${gone}`)).toHaveCount(0);
    }

    await bar.getByTestId('bottom-status-bar-spend').click();
    const popover = page.getByRole('dialog', { name: 'Spend detail' });
    await expect(popover).toContainText('Engine total');
    await expect(popover).toContainText('$1.50');
    await expect(popover).toContainText('This session');
    await expect(popover).toContainText('not metered');

    mkdirSync(OUT_DIR, { recursive: true });
    await page.screenshot({ path: join(OUT_DIR, 'status-bar-cards.png'), clip: { x: 0, y: 620, width: 1440, height: 280 } });

    await page.getByRole('button', { name: /configure status bar/i }).click();
    for (const name of ['Engine', 'Spend', 'Mesh', 'Routing', 'Needs you']) {
      await expect(page.getByRole('checkbox', { name, exact: true })).toBeVisible();
    }
  });

  test('Routing shows the concrete id only for a catalog-resolved pick', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await addRichMockInitScript(page, 'chat');
    await addInvokeOverrides(page, { responses: { get_routing_summary_live: routingSummary('catalog') } });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await expect(page.getByTestId('bottom-status-bar-routing')).toContainText('Auto → acme/widget-flash-20260901');
  });
});
