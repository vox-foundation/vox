import type { Page } from '@playwright/test';
import { discoverAllGuiSurfacesAndPanels } from '../../src/lib/cvSurfaceDiscovery';

export interface ReviewViewport { name: 'wide' | 'laptop' | 'compact'; width: number; height: number; }
export const VIEWPORTS: ReviewViewport[] = [
  { name: 'wide', width: 1440, height: 900 },
  { name: 'laptop', width: 1100, height: 720 },
  { name: 'compact', width: 900, height: 600 },
];

export type MockKind = 'rich' | 'empty' | 'error' | 'none';

export interface ReviewState {
  name: string;
  /** Drive the page into the state AFTER the surface has rendered. */
  setup?: (page: Page) => Promise<void>;
  /** Restrict to viewports where this state's UI exists. */
  viewports?: Array<'wide' | 'laptop' | 'compact'>;
  /** Which mock installer backs this capture (default 'rich').
   * 'empty'/'error' subsume screenshots-variants.spec.ts; 'none' captures
   * true no-backend browser mode (BackendBanner regression). */
  mock?: MockKind;
  /** Window flags seeded before initial page load so mount-time IPC calls observe them. */
  initFlags?: Record<string, boolean>;
}

export interface TerminalCvStateSpec extends ReviewState {
  surface: 'console' | 'chat';
  initFlags?: Record<string, boolean>;
}

const DEFAULT: ReviewState = { name: 'default' };
/** Empty/error coverage inherited from screenshots-variants KEY_SURFACES. */
const VARIANT: ReviewState[] = [
  { name: 'empty', mock: 'empty' },
  { name: 'error', mock: 'error' },
];
const VARIANT_SURFACES = new Set([
  'dashboard', 'chat', 'runs', 'approvals', 'models',
  'memory', 'vox-search', 'policies', 'gamify', 'settings',
]);

/** Selector ground truth verified dynamically from SURFACE_REGISTRY; a failed setup records
 * state_ok:false in the entry (evidence), it does not fail the run. */
export const SURFACE_STATES: Record<string, ReviewState[]> = Object.fromEntries(
  discoverAllGuiSurfacesAndPanels().registeredViewKeys.map((k) => [k, [DEFAULT]]),
);

const CHAT_TERMINAL_DOCK_STATE: TerminalCvStateSpec = {
  name: 'chat-terminal-dock',
  surface: 'chat',
  initFlags: { __MOCK_CHAT_TERM_DOCK__: true },
  setup: async (p) => {
    await p.evaluate(() => {
      (window as any).__MOCK_CHAT_TERM_DOCK__ = true;
    });
    const dock = p.getByTestId('chat-dock-terminal');
    if (!(await dock.isVisible())) {
      await p.getByRole('button', { name: 'Panels', exact: true }).click({ timeout: 5_000 });
      await p.getByTestId('dock-preset-engineering').click({ timeout: 5_000 });
    }
    await dock.waitFor({ state: 'visible', timeout: 5_000 });
  },
};

const CONSOLE_CV_STATES: TerminalCvStateSpec[] = [
  {
    name: 'block-stream-ansi',
    surface: 'console',
    setup: async (p) => {
      await p.getByTestId('term-session-bar').first().waitFor({ state: 'visible', timeout: 5_000 });
      await p.getByTestId('term-block-output').first().waitFor({ state: 'visible', timeout: 5_000 });
      await p.evaluate(() => {
        const stream = document.querySelector<HTMLElement>('[data-testid="block-stream-view"]');
        if (stream) stream.scrollTop = 0;
      });
    },
  },
  {
    name: 'split-panes',
    surface: 'console',
    setup: async (p) => {
      await p.getByTestId('term-split-right').first().click({ timeout: 5_000 });
      await p.locator('[data-testid="terminal-session-pane"]').nth(1).waitFor({ state: 'visible', timeout: 5_000 });
    },
  },
  {
    name: 'history-search-open',
    surface: 'console',
    setup: async (p) => {
      await p.getByLabel('console input').first().focus({ timeout: 5_000 });
      await p.keyboard.press('Control+r');
      await p.getByTestId('term-history-search').waitFor({ state: 'visible', timeout: 5_000 });
    },
  },
  {
    name: 'json-ledger-table',
    surface: 'console',
    setup: async (p) => {
      await p.getByTestId('term-ledger-tab-table').first().click({ timeout: 5_000 });
      await p.getByTestId('term-ledger-table').first().waitFor({ state: 'visible', timeout: 5_000 });
    },
  },
  {
    name: 'runbooks-open',
    surface: 'console',
    setup: async (p) => {
      const restoreBtn = p.getByTestId('discovery-rail-restore-button');
      if (await restoreBtn.isVisible()) {
        await restoreBtn.click({ timeout: 5_000 });
      }
      await p.getByTestId('discovery-tab-runbooks').click({ timeout: 5_000 });
      await p.getByTestId('term-runbook-panel').waitFor({ state: 'visible', timeout: 5_000 });
      await p.locator('[data-testid^="term-runbook-card-"]').first().waitFor({ state: 'visible', timeout: 5_000 });
      await p.evaluate(() => {
        const stream = document.querySelector<HTMLElement>('[data-testid="block-stream-view"]');
        if (stream) stream.scrollTop = 0;
      });
    },
  },
  {
    name: 'approval-card-pending',
    surface: 'console',
    initFlags: { __MOCK_TERM_APPROVAL_ACTIVE__: true },
    setup: async (p) => {
      await p.evaluate(() => {
        (window as any).__MOCK_TERM_APPROVAL_ACTIVE__ = true;
      });
      await p.getByTestId('term-session-bar').first().waitFor({ state: 'visible', timeout: 5_000 });
      await p.getByTestId('term-approval-card').first().waitFor({ state: 'visible', timeout: 5_000 });
    },
  },
  {
    name: 'auto-tui-alt-screen',
    surface: 'console',
    initFlags: { __VOX_E2E_REAL_XTERM__: true, __MOCK_TERM_ALT_SCREEN__: true },
    setup: async (p) => {
      await p.evaluate(() => {
        (window as any).__VOX_E2E_REAL_XTERM__ = true;
        (window as any).__MOCK_TERM_ALT_SCREEN__ = true;
      });
      const vp = p.getByTestId('terminal-viewport').first();
      if (!(await vp.isVisible())) {
        await p.getByTestId('term-view-mode-toggle').first().click({ timeout: 5_000 });
      }
      await p.locator('[data-testid="terminal-viewport"] .xterm').first().waitFor({ state: 'visible', timeout: 5_000 });
      await p.waitForFunction(
        () => {
          const rows = document.querySelector('[data-testid="terminal-viewport"] .xterm-rows');
          return Boolean(rows && (rows.textContent || '').trim().length > 10);
        },
        undefined,
        { timeout: 5_000 },
      );
    },
  },
];

export const TERMINAL_CV_STATES: TerminalCvStateSpec[] = [
  ...CONSOLE_CV_STATES,
  CHAT_TERMINAL_DOCK_STATE,
];

SURFACE_STATES['console'] = CONSOLE_CV_STATES.map(({ name, setup, viewports, mock, initFlags }) => ({
  name,
  ...(setup ? { setup } : {}),
  ...(viewports ? { viewports } : {}),
  ...(mock ? { mock } : {}),
  ...(initFlags ? { initFlags } : {}),
}));

SURFACE_STATES['chat'] = [
  DEFAULT,
  {
    name: 'model-picker-open',
    // ChatModelPicker was deleted (never mounted); model choice is the composer's
    // model-tier trigger. Scoped to the chat layout so transcript text cannot collide.
    // 5 s click timeout: a stale selector records state_ok:false instead of timing the test out.
    setup: async (p) => {
      await p
        .getByTestId('chat-surface-layout')
        .getByRole('button', { name: 'Choose model tier' })
        .click({ timeout: 5_000 });
    },
  },
  {
    name: 'panels-menu-open',
    // Replaces session-menu-open: sessions moved to the sidebar and have no actions
    // menu. The Panels menu lists every dock panel by its lexicon label.
    setup: async (p) => {
      await p.getByRole('button', { name: 'Panels', exact: true }).click({ timeout: 5_000 });
    },
  },
  {
    name: 'composer-filled',
    setup: async (p) => {
      await p.getByLabel('Task composer').fill(
        'A deliberately long composer draft that should wrap across multiple lines and reveal any clipping or overlap issues in the dock '.repeat(2),
      );
    },
  },
  {
    name: CHAT_TERMINAL_DOCK_STATE.name,
    setup: CHAT_TERMINAL_DOCK_STATE.setup,
    ...(CHAT_TERMINAL_DOCK_STATE.initFlags ? { initFlags: CHAT_TERMINAL_DOCK_STATE.initFlags } : {}),
  },
  ...VARIANT,
];

SURFACE_STATES['tasks'] = [
  DEFAULT,
  {
    name: 'composer-filled',
    setup: async (p) => {
      await p.getByLabel('Add a task').fill(
        'Draft task with an intentionally very long title to probe truncation and row overflow behavior in the composer',
      );
    },
  },
  // NOTE: priority-select-open is intentionally omitted — native <select>
  // popups render outside the page and cannot be screenshotted.
];

SURFACE_STATES['settings'] = [
  DEFAULT,
  { name: 'search-filtered', setup: async (p) => { await p.getByLabel('Search settings').fill('key'); } },
  { name: 'section-keybinds', setup: async (p) => { await p.getByRole('button', { name: 'Keybinds' }).click(); } },
  ...VARIANT,
];

SURFACE_STATES['approvals'] = [
  DEFAULT,
  {
    name: 'row-focused',
    setup: async (p) => { for (let i = 0; i < 4; i++) await p.keyboard.press('Tab'); },
  },
  ...VARIANT,
];

SURFACE_STATES['dashboard'] = [
  DEFAULT,
  { name: 'omnibar-open', setup: async (p) => { await p.keyboard.press('Control+k'); } },
  { name: 'sidebar-collapsed', setup: async (p) => { await p.getByRole('button', { name: 'Collapse sidebar' }).click(); } },
  { name: 'achievements-open', setup: async (p) => { await p.getByRole('button', { name: 'Open achievements' }).click(); } },
  {
    name: 'hud-hidden',
    setup: async (p) => { await p.keyboard.press('Control+Shift+H'); await p.keyboard.press('Control+Shift+H'); },
  },
  {
    name: 'focus-visible',
    setup: async (p) => { for (let i = 0; i < 4; i++) await p.keyboard.press('Tab'); },
  },
  // The Phase A regression: no mock at all -> banner must render, zero raw
  // TypeErrors; the banner's own placement gets AI-reviewed.
  { name: 'no-backend', mock: 'none', viewports: ['wide'] },
  ...VARIANT,
];

SURFACE_STATES['research'] = [
  DEFAULT,
  {
    name: 'prober-open',
    setup: async (p) => {
      const btn = p.getByTestId('toggle-diagnostics-btn');
      if (await btn.isVisible()) await btn.click();
    },
  },
];

SURFACE_STATES['mens'] = [
  DEFAULT,
  {
    name: 'corpus-studio-inspector',
    setup: async (p) => {
      const studio = p.getByTestId('mens-corpus-studio-panel');
      if (await studio.isVisible()) {
        await studio.scrollIntoViewIfNeeded();
        const row2 = p.getByTestId('mens-jsonl-row-2');
        if (await row2.isVisible()) await row2.click();
      }
    },
  },
];

SURFACE_STATES['gsd'] = [DEFAULT];

for (const k of VARIANT_SURFACES) {
  if (!SURFACE_STATES[k].some((s) => s.name === 'empty')) SURFACE_STATES[k].push(...VARIANT);
}

/** Themes captured for default states at wide/chromium (Task 7). */
export const AUDIT_THEMES = ['high-contrast'] as const;

export function statesFor(viewKey: string): ReviewState[] {
  return SURFACE_STATES[viewKey] ?? [DEFAULT];
}
