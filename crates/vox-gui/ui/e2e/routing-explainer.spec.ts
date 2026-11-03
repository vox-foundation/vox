import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');
const PARTS = { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 };
const EXPLANATION = {
  mode: 'efficiency', task: 'codegen', complexity: 7, chosen: 'acme/widget-5.5', only_candidate: false, total_models: 120,
  candidates: [
    { id: 'acme/widget-5.5', provider: 'acme', tier: 'Fast', is_free: false, price_out_per_m: 0.9, score: 0.65,
      quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' }, parts: PARTS },
    { id: 'acme/gadget-2', provider: 'acme', tier: 'Pro', is_free: false, price_out_per_m: null, score: 0.61,
      quality: { value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null }, parts: PARTS },
  ],
  excluded: [
    { reason: 'flagship_excluded_by_mode', count: 4, examples: ['acme/flagship-9', 'acme/flagship-8', 'acme/flagship-7'] },
    { reason: 'superseded', count: 1, examples: ['acme/widget-4.8'] },
  ],
};
const HEALTHY = {
  schema_version: 1, checked_at_unix: 0, models: 120, cloud_models: 100, benchmarked: 40, inherited: 7,
  unknown_tier_cloud: 0, quality_scale: 'derived', price_bands: 'derived', efficient_pick: 'acme/widget-5.5',
  violations: [] as { invariant: string; detail: string }[],
};

async function openModels(page: Page, health = HEALTHY) {
  await page.addInitScript(({ explanation, health }) => {
    localStorage.setItem('vox_sidebar_mode', 'default');
    localStorage.setItem('vox_onboarding_dismissed', 'true');
    (window as any).__explainCalls = [];
    (window as any).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args?: any) => {
        if (cmd === 'get_initial_view') return 'models';
        if (cmd === 'list_model_cards') return [];
        if (cmd === 'get_routing_summary_live') return { decision_preview: null };
        if (cmd === 'get_active_model') return null;
        if (cmd === 'inference_provider_status') return [];
        if (cmd === 'explain_routing') { (window as any).__explainCalls.push(args); return explanation; }
        if (cmd === 'get_routing_health') return health;
        return null;
      },
    };
  }, { explanation: EXPLANATION, health });
  await page.goto('/');
  await expect(page.getByTestId('routing-explainer')).toBeVisible();
}

async function shot(page: Page, name: string) {
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, name) });
}

test.describe('Routing explainer', () => {
  test('explainer shows the chosen model, ranked candidates and provenance', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    const panel = page.getByTestId('routing-explainer');
    await expect(panel.getByRole('heading', { name: 'Why acme/widget-5.5' })).toBeVisible();
    await expect(panel.getByText('How task dispatch would choose now')).toBeVisible();
    const rows = panel.getByRole('table', { name: 'Candidates ranked by routing score' }).getByRole('row');
    await expect(rows.nth(1)).toContainText('Chosen');
    await expect(rows.nth(1)).toContainText('from acme/widget-5.0 (38.2)');
    await expect(rows.nth(2)).toContainText('—');
    await shot(page, 'routing-explainer.png');
  });

  test('excluded models open from the disclosure', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    await page.getByText('Not considered (5)').click();
    await expect(page.getByText('Flagship, kept out by this mode (4)')).toBeVisible();
    await shot(page, 'routing-explainer-excluded.png');
  });

  test('health problems are visible, and a healthy panel is quiet', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page, { ...HEALTHY, price_bands: 'fallback',
      violations: [{ invariant: 'tiers_known', detail: '30 of 100 cloud models have no tier' }] });
    const footer = page.getByTestId('routing-health');
    await expect(footer).toHaveAttribute('data-state', 'warn');
    await expect(footer).toContainText('30 of 100 cloud models have no tier');
    await shot(page, 'routing-explainer-health-warn.png');
  });

  test('the panel fits a narrow window', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openModels(page);
    const box = await page.getByTestId('routing-explainer').boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x + box!.width).toBeLessThanOrEqual(390);
    await shot(page, 'routing-explainer-narrow.png');
  });

  test('switching mode asks routing again', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    await page.getByRole('combobox', { name: 'Routing mode' }).selectOption('genius');
    await expect
      .poll(() => page.evaluate(() => (window as any).__explainCalls.some((a: any) => a?.mode === 'genius')))
      .toBe(true);
  });

  test('the Routing status card shows a dot when routing health reports a problem', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page, { ...HEALTHY, violations: [{ invariant: 'tiers_known', detail: 'x' }] });
    await expect(page.getByRole('img', { name: 'Routing health: 1 problem' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Open routing details' })).toBeVisible();
  });

  test('a healthy Routing status card has no dot', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await openModels(page);
    await expect(page.getByRole('button', { name: 'Open routing details' })).toBeVisible();
    await expect(page.getByTestId('bottom-status-bar-routing-health')).toHaveCount(0);
  });
});
