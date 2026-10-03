import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'review-bundle', 'latest');
const AGENT_EVENTS = 'vox://agent-events';
const ACTIVITY_APPENDED = 'vox://activity-appended';

interface Overrides {
  /** Tauri commands that throw. */
  reject?: string[];
  /** MCP tools (`invoke_mcp_tool` args.tool) that throw. */
  rejectTools?: string[];
  /** Command -> value to throw (an object, like a Tauri `Result::Err`). */
  rejectWith?: Record<string, unknown>;
}

/** Mock shell on `view`, with per-test failures layered over the base mock (it runs after `installTauriMock`). */
async function open(page: Page, view: string, overrides: Overrides = {}, size = { width: 1440, height: 900 }) {
  await addMockInitScript(page, installTauriMock, view);
  await page.addInitScript((o: Required<Overrides>) => {
    const internals = (window as any).__TAURI_INTERNALS__;
    const base = internals?.invoke;
    if (typeof base !== 'function') throw new Error('overrides must run after the base mock');
    internals.invoke = async (cmd: string, args?: any) => {
      if (o.reject.includes(cmd)) throw new Error(`${cmd} unavailable (test)`);
      if (Object.prototype.hasOwnProperty.call(o.rejectWith, cmd)) throw o.rejectWith[cmd];
      if (cmd === 'invoke_mcp_tool' && o.rejectTools.includes(args?.tool)) throw new Error(`${args.tool} unavailable (test)`);
      return base(cmd, args);
    };
  }, { reject: overrides.reject ?? [], rejectTools: overrides.rejectTools ?? [], rejectWith: overrides.rejectWith ?? {} });
  await page.setViewportSize(size);
  await page.goto('/');
  await expect(page.getByTestId('bottom-status-bar')).toBeVisible();
}

async function listening(page: Page, event: string) {
  await expect
    .poll(() => page.evaluate((ev) => ((window as any).__TAURI_EVENT_LISTENERS__?.[ev] ?? []).length, event))
    .toBeGreaterThan(0);
}

async function emit(page: Page, event: string, payload: unknown) {
  await page.evaluate(([ev, p]) => (window as any).__TAURI_EMIT__(ev as string, p), [event, payload] as const);
}

async function shot(page: Page, name: string) {
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, name) });
}

/** Calls to `activity_query` that are the Activity view's own (the chat rail polls it with a session_id). */
const sessionlessActivityQueries = (page: Page) =>
  page.evaluate(
    () => ((window as any).__TAURI_CALLS__ as Array<{ cmd: string; args: any }>)
      .filter((c) => c.cmd === 'activity_query' && !c.args?.filter?.session_id).length,
  );

const bell = (page: Page, name: string | RegExp) => page.getByRole('button', { name });

test.describe('Notices', () => {
  test('an engine error lands in the center, not as a toast', async ({ page }) => {
    await open(page, 'chat');
    await listening(page, AGENT_EVENTS);
    await emit(page, AGENT_EVENTS, {
      id: 1, timestamp_ms: 1, severity: 'error',
      kind: { type: 'task_failed', task_id: 7, agent_id: 1, error: 'boom' },
    });
    await expect(bell(page, 'Notifications, 1 need attention')).toBeVisible();
    await expect(page.getByTestId('toast-stack').locator('> *')).toHaveCount(0);
    await bell(page, /^Notifications/).click();
    const item = page.locator('[data-severity="error"]');
    await expect(item).toHaveCount(1);
    await expect(item).toContainText('boom');
    await shot(page, 'notices-engine-error.png');
  });

  test('repeated engine warnings coalesce into one counted item', async ({ page }) => {
    await open(page, 'chat');
    await listening(page, AGENT_EVENTS);
    for (let i = 1; i <= 3; i += 1) {
      await emit(page, AGENT_EVENTS, {
        id: i, timestamp_ms: 10 + i, severity: 'warning',
        kind: { type: 'tool_timed_out', agent_id: 4, tool_key: 'vox_run', attempted_budget_ms: 5000 },
      });
    }
    await bell(page, /^Notifications/).click();
    const items = page.locator('[data-severity="warning"]');
    await expect(items).toHaveCount(1);
    await expect(items).toContainText('×3');
    await shot(page, 'notices-coalesced.png');
  });

  test('a toast survives in the center after it expires', async ({ page }) => {
    await open(page, 'chat', {
      rejectWith: { chat_turn: { kind: 'budget_exceeded', message: 'Daily budget of $5.00 exceeded (spent $5.12)' } },
    });
    const composer = page.getByPlaceholder(/describe a task/i);
    await composer.click();
    await composer.fill('hello there');
    await composer.press('Enter');
    const stack = page.getByTestId('toast-stack');
    await expect(stack.getByText('Budget limit reached')).toBeVisible();
    await expect(stack.locator('> *')).toHaveCount(0, { timeout: 10_000 });
    await bell(page, /^Notifications/).click();
    await expect(page.getByRole('dialog', { name: 'Notifications' })).toContainText('Budget limit reached');
  });

  test('a failed inbox source shows a degraded Review badge', async ({ page }) => {
    await open(page, 'chat', { rejectTools: ['vox_pending_approvals'] });
    await expect(page.getByRole('button', { name: "Review, couldn't load approvals" })).toBeVisible();
    await expect(page.getByTestId('bottom-status-bar-needs-you')).toContainText("couldn't load approvals");
    await shot(page, 'notices-degraded-review.png');
  });

  test('the Activity view refreshes only on activity-appended', async ({ page }) => {
    await open(page, 'activity');
    await listening(page, ACTIVITY_APPENDED);
    await expect.poll(() => sessionlessActivityQueries(page)).toBeGreaterThan(0);
    // Let the mount-time queries settle, then take the baseline.
    await page.waitForTimeout(800);
    const before = await sessionlessActivityQueries(page);
    for (let i = 0; i < 5; i += 1) {
      await emit(page, AGENT_EVENTS, { id: i, timestamp_ms: i, severity: 'debug', kind: { type: 'token_streamed', agent_id: 1, text: 't' } });
    }
    await page.waitForTimeout(800);
    expect(await sessionlessActivityQueries(page)).toBe(before);
    await emit(page, ACTIVITY_APPENDED, null);
    await expect.poll(() => sessionlessActivityQueries(page)).toBe(before + 1);
  });

  // 520px: the narrowest window this shell lays out (the 212px sidebar alone is over half of a phone width, and
  // below ~480px the status bar's fixed right-hand items no longer fit, which is the shell's limit, not the bell's).
  test('the bell is reachable and the drawer fits a narrow window', async ({ page }) => {
    await open(page, 'chat', {}, { width: 520, height: 844 });
    await listening(page, AGENT_EVENTS);
    await emit(page, AGENT_EVENTS, {
      id: 1, timestamp_ms: 1, severity: 'error',
      kind: { type: 'task_failed', task_id: 7, agent_id: 1, error: 'boom' },
    });
    const trigger = bell(page, /^Notifications/);
    const triggerBox = await trigger.boundingBox();
    expect(triggerBox).not.toBeNull();
    expect(triggerBox!.x + triggerBox!.width).toBeLessThanOrEqual(520);
    await trigger.click();
    const drawer = page.getByRole('dialog', { name: 'Notifications' });
    await expect(drawer).toBeVisible();
    const box = await drawer.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width).toBeLessThanOrEqual(520);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await shot(page, 'notices-narrow.png');
  });
});
