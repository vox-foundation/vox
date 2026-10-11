import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';
import { addInvokeOverrides } from './lib/invokeOverrides';
import { ACTION_REGISTRY } from '../src/lib/keybinds';

/**
 * Universal palette (phase 06.3): capability-index rows in the Cmd/Ctrl+K palette.
 *
 * Run: pnpm exec playwright test e2e/universal-palette.spec.ts --project=chromium
 */
const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');

async function shot(page: Page, name: string) {
  await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, `${name}.png`) });
}

test.describe('Universal palette', () => {
  test.beforeEach(async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);
  });

  test('index CLI row opens the prefilled Catalog form', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await expect(input).toBeVisible();
    await input.fill('vox check');

    // The MCP twin (vox_check) also mentions the verb in its detail now that every kind is
    // listed, so anchor on the row label.
    const row = page.getByRole('button', { name: /^vox check\b/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toContainText('CLI');
    await expect(row).toContainText('MCP');
    await shot(page, 'universal-palette-01-cli-row');

    // Activate the row (D-01: opens the Catalog form, never executes). Enter-on-top-row is
    // covered by the Omnibar unit test; Best match (a later plan) will put this row first.
    await row.click();
    await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=catalog');
    await expect(page.getByRole('heading', { name: 'vox check' })).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('[data-catalog-path="check"]')).toHaveAttribute('aria-current', 'true');
    await shot(page, 'universal-palette-02-prefilled-form');
  });

  test('a full CLI path ranks the exact verb first without fuzzy noise (D-19)', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await input.fill('vox fabrica fmt');

    const row = page.getByRole('button', { name: /vox fabrica fmt/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    // Jaro-Winkler is a typo fallback only: an exact path must not drag in near-miss verbs.
    await expect(page.getByRole('button', { name: /vox fmt\b/ })).toHaveCount(0);
    await shot(page, 'universal-palette-03-ranked-exact-path');
  });

  test('every kind is reachable from ⌘K (D-05, D-08, D-08a, D-09, D-10)', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await expect(input).toBeVisible();
    const row = (text: string | RegExp) => page.getByRole('button').filter({ hasText: text }).first();
    // Best match MOVES the top hit out of its facet, so the Best match row is the first button
    // under the "Best match" heading.
    // Scoped to the dialog: the page behind it has nav items called "Commands" etc.
    const heading = (name: string) => page.getByRole('dialog', { name: 'Command palette' }).getByText(name, { exact: true });
    const bestRow = () => heading('Best match').locator('xpath=ancestor::div[1]/following-sibling::button[1]');

    // CLI verb: exact full path is the Best match.
    await input.fill('vox fabrica fmt');
    await expect(bestRow()).toContainText('vox fabrica fmt', { timeout: 10_000 });
    await shot(page, 'universal-palette-03-best-match');

    // MCP tool, with its CLI equivalent chip.
    await input.fill('');
    await input.fill('vox_capability_explain');
    await expect(row('vox_capability_explain')).toBeVisible({ timeout: 10_000 });
    await expect(row('vox_capability_explain')).toContainText('MCP');
    await expect(row('vox_capability_explain')).toContainText('CLI');

    // Tools & crates facet, capped with an honest count only when it overflows.
    await input.fill('');
    await input.fill('capability');
    await expect(heading('Tools & crates')).toBeVisible({ timeout: 10_000 });
    await expect(row('vox_capability_features')).toBeVisible();
    await row('vox_capability_features').scrollIntoViewIfNeeded(); // the facet sits below the Commands rows
    await shot(page, 'universal-palette-04-tools');

    // # mode: crates only, layer query, "N of M" narrowing hint on overflow (D-08a, D-10).
    await input.fill('');
    await input.fill('#L3');
    await expect(heading('Best match')).toBeVisible({ timeout: 10_000 });
    await expect(bestRow()).toContainText('vox-');
    await expect(heading('Tools & crates')).toBeVisible();
    await expect(page.getByText(/^6 of \d+$/)).toBeVisible();
    await expect(page.getByText(/Keep typing to narrow/)).toBeVisible();
    await expect(heading('Surfaces')).toHaveCount(0);
    await expect(heading('Commands')).toHaveCount(0);
    await shot(page, 'universal-palette-05-crates');

    await input.fill('');
    await input.fill('#vox-actor-runtime');
    await expect(row('vox-actor-runtime')).toBeVisible({ timeout: 10_000 });

    // Surface.
    await input.fill('');
    await input.fill('mesh');
    await expect(bestRow()).toContainText('Mesh', { timeout: 10_000 });
    await expect(bestRow()).toContainText('GUI');

    // Doc (title-exact match, so default mode shows it).
    await input.fill('');
    await input.fill('where things live');
    await expect(row('Where Things Live')).toBeVisible({ timeout: 10_000 });

    // Setting (federated).
    await input.fill('');
    await input.fill('Max concurrent agents');
    await expect(row('Max concurrent agents')).toBeVisible({ timeout: 10_000 });

    // Skill (index gap-fill: not installed in the mock).
    await input.fill('');
    await input.fill('algorithmic-art');
    await expect(row('algorithmic-art')).toBeVisible({ timeout: 10_000 });
  });

  test('MCP tool opens the explain card (D-07)', async ({ page }) => {
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    const openCard = async () => {
      await page.keyboard.press('Control+k');
      const input = page.getByPlaceholder(/search surfaces, commands/i);
      await expect(input).toBeVisible();
      await input.fill('vox_check');
      // Best match = the exact MCP name; Enter opens the card instead of doing nothing.
      await expect(page.getByRole('dialog', { name: 'Command palette' }).getByText('Best match', { exact: true })).toBeVisible({ timeout: 10_000 });
      await page.keyboard.press('Enter');
      const card = page.getByRole('dialog').filter({ hasText: 'Equivalents' });
      await expect(card).toBeVisible({ timeout: 10_000 });
      return card;
    };

    const card = await openCard();
    await expect(card.getByText('vox check', { exact: true })).toBeVisible();
    await shot(page, 'universal-palette-06-explain-card');

    await card.getByRole('button', { name: 'Copy MCP name' }).click();
    await expect(page.getByTestId('toast-stack').getByText('Copied')).toBeVisible();
    await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe('vox_check');

    await page.keyboard.press('Escape');
    await expect(card).toHaveCount(0);

    const again = await openCard();
    await again.getByRole('button', { name: 'Open form' }).click();
    await expect.poll(async () => page.evaluate(() => window.location.hash)).toContain('view=catalog');
    await expect(page.getByRole('heading', { name: 'vox check' })).toBeVisible({ timeout: 10_000 });
  });
});

test.describe('run from the palette (D-01, D-03, D-04)', () => {
  const arg = (over: Record<string, unknown>) => ({
    name: 'x', short: null, long: null, help: null, required: false, takes_value: true, value_kind: 'value', ...over,
  });
  const entry = (path: string[], args: unknown[]) => ({
    path, command: `vox ${path.join(' ')}`, about: `Run vox ${path.join(' ')}`, aliases: [], has_subcommands: false,
    compiled_in: true, source_group: 'core', feature_gate: null, tier: 'recommended', arguments: args,
  });

  test.beforeEach(async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await addInvokeOverrides(page, {
      responses: {
        get_command_catalog: {
          generated_from: 'e2e',
          entries: [
            entry(['features'], [arg({ name: 'domain', long: 'domain' })]),
            entry(['check'], [arg({ name: 'path', long: 'path' })]),
            entry(['plugin', 'remove'], [arg({ name: 'name', required: true })]),
          ],
        },
      },
    });
    // Self-contained gate mock (same shape as invokeOverrides): records every execute_command and
    // refuses `plugin remove` unless it carries confirmed: true, like the Rust gate (plan 02).
    await page.addInitScript(() => {
      const internals = (window as any).__TAURI_INTERNALS__;
      const base = internals.invoke;
      (window as any).__EXEC_CALLS__ = [];
      internals.invoke = async (cmd: string, args?: any) => {
        if (cmd === 'execute_command') {
          (window as any).__EXEC_CALLS__.push({ path: args?.path, args: args?.args, confirmed: args?.confirmed });
          const p = (args?.path ?? []).join(' ');
          if (p === 'plugin remove' && args?.confirmed !== true) throw 'confirmation_required: cli.plugin.remove';
          if (p === 'features') return { exit_code: 0, stdout: 'features: parser, codegen', stderr: '' };
        }
        return base(cmd, args);
      };
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);
  });

  test('a read-only verb runs from ⌘K with a toast and a full-output dialog', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const palette = page.getByRole('dialog', { name: 'Command palette' });
    await palette.getByPlaceholder(/search surfaces, commands/i).fill('vox features');
    const row = palette.getByRole('button', { name: /^vox features\b/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toContainText('↵ runs');
    await page.keyboard.press('Enter');

    const stack = page.getByTestId('toast-stack');
    await expect(stack).toContainText('vox features — exit 0', { timeout: 10_000 });
    await expect(stack).toContainText('features: parser, codegen');
    await shot(page, 'universal-palette-07-run-toast');

    await stack.getByRole('button', { name: 'View full output' }).click();
    const out = page.getByRole('dialog').filter({ hasText: 'stdout' });
    await expect(out).toContainText('features: parser, codegen');
    await shot(page, 'universal-palette-08-run-output');

    const calls = await page.evaluate(() => (window as any).__EXEC_CALLS__);
    expect(calls).toHaveLength(1);
    expect(calls[0].path).toEqual(['features']);
    expect(calls[0].confirmed).toBeUndefined();
  });

  test('a destructive verb opens its form on Enter and needs explicit confirmation', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const palette = page.getByRole('dialog', { name: 'Command palette' });
    await palette.getByPlaceholder(/search surfaces, commands/i).fill('vox plugin remove');
    const row = palette.getByRole('button', { name: /^vox plugin remove\b/ });
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toContainText('↵ opens form');
    await page.keyboard.press('Enter');

    await expect(page.getByRole('heading', { name: 'vox plugin remove' })).toBeVisible({ timeout: 10_000 });
    // Enter alone ran nothing.
    expect(await page.evaluate(() => (window as any).__EXEC_CALLS__)).toHaveLength(0);

    await page.getByLabel('name', { exact: true }).fill('demo');
    await page.getByRole('button', { name: 'Run', exact: true }).click();
    const confirm = page.getByRole('dialog', { name: 'Run a destructive command?' });
    await expect(confirm).toBeVisible();
    await shot(page, 'universal-palette-09-confirm');
    expect(await page.evaluate(() => (window as any).__EXEC_CALLS__)).toHaveLength(1);

    await confirm.getByRole('button', { name: 'Run anyway' }).click();
    await expect(page.getByText('Exit Code: 0')).toBeVisible({ timeout: 10_000 });

    const calls = await page.evaluate(() => (window as any).__EXEC_CALLS__);
    expect(calls).toHaveLength(2);
    expect(calls[0].confirmed).toBeUndefined();
    expect(calls[1].confirmed).toBe(true);
    expect(calls[1].args.__positionals).toEqual(['demo']);
  });
});

test.describe('shortcuts (D-11, D-12, D-17)', () => {
  test.beforeEach(async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(() => (document as any).fonts?.ready);
  });

  test('? opens the generated sheet; ? in a field types', async ({ page }) => {
    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press('?');
    const sheet = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
    await expect(sheet).toBeVisible();
    for (const a of ACTION_REGISTRY) await expect(sheet.getByText(a.label)).toBeVisible();
    await shot(page, 'universal-palette-10-shortcut-sheet');

    await page.keyboard.press('Escape');
    await expect(sheet).toBeHidden();
    await page.keyboard.press('Control+k');
    const input = page.getByPlaceholder(/search surfaces, commands/i);
    await expect(input).toBeVisible();
    await input.pressSequentially('?');
    await expect(input).toHaveValue('?');
    await expect(page.getByRole('dialog', { name: 'Keyboard shortcuts' })).toHaveCount(0);
  });

  test('a rebind in Settings shows in the sheet without restart', async ({ page }) => {
    await page.goto('/#view=settings');
    await page.getByRole('button', { name: /keybinds/i }).click();
    const btn = page.getByTestId('keybind-btn-toggle-sidebar');
    await btn.click();
    await page.keyboard.press('Control+Shift+Y');
    await expect(btn).toHaveText('Mod+Shift+Y');

    await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
    await page.keyboard.press('?');
    const sheet = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
    await expect(sheet).toBeVisible();
    await expect(sheet.getByTestId('shortcut-row-toggle-sidebar')).toContainText(/⌘⇧Y|Ctrl\+Shift\+Y/);
  });
});
