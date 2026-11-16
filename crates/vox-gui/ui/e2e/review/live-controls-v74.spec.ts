/**
 * Milestone v7.4 (`AXIS-CONCEPT-CV-03`) — Live-Control IPC Round-Trip E2E Suite
 *
 * Verifies real end-to-end state + Tauri IPC round-trips across:
 * 1. `OratioStudioView` (`#view=oratio`) rotary knobs (`set_gui_preference` + reload hydration) & Push-to-Talk (`oratio_transcribe`).
 * 2. `PopuliMeshBentoView` (`#view=populi`) gossip cadence knob, `[data-cv-chart="true"]` topology ring, `execute_command` probe, and `ImperiumInspector` routing knob.
 * 3. `RunsView` (`#view=runs`) clickable `<RomanBrassGauge>` filters, `VoxSelect` (`[data-slot="select-trigger"]`) comboboxes, and `Dashboard` Recharts `<Brush>` + series click.
 * 4. `Mercatus` (`#view=mercatus`) `<Switch>` (`[data-slot="switch"]`) + ceiling threshold persistence (`mercatus_save_config`) & zero `fake_success` honesty invariants.
 */

import { test, expect, type Page } from '@playwright/test';
import { addRichMockInitScript, buildRichOrchestratorStatus } from '../lib/tauriMockRich';
import { checkHonestyInvariant } from '../../src/debugger/sentries/sentryHonesty';

interface RecordedIpcCall {
  cmd: string;
  args: Record<string, unknown> | null;
}

async function installLiveControlHarness(page: Page, initialView: string): Promise<string[]> {
  const pageErrors: string[] = [];
  page.on('pageerror', (err) => {
    pageErrors.push(err.message || String(err));
  });

  await page.setViewportSize({ width: 1440, height: 900 });
  await addRichMockInitScript(page, initialView);
  await page.addInitScript(() => {
    const PREF_PREFIX = 'vox.__e2e_pref__.';
    const MERCATUS_CFG_KEY = 'vox.__e2e_mercatus_cfg__';
    const w = window as unknown as {
      __LIVE_IPC_CALLS__?: RecordedIpcCall[];
      __TAURI_CALLS__?: RecordedIpcCall[];
      liveIpcFail?: { cmd: string; message: string };
      __TAURI_INTERNALS__?: {
        invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
    };

    w.__LIVE_IPC_CALLS__ = [];
    const baseInvoke = w.__TAURI_INTERNALS__?.invoke;
    if (typeof baseInvoke !== 'function' || !w.__TAURI_INTERNALS__) {
      throw new Error('Expected installTauriMockRich invoke before live-control wrapper');
    }

    w.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      const record: RecordedIpcCall = { cmd, args: args ?? null };
      w.__LIVE_IPC_CALLS__!.push(record);
      if (Array.isArray(w.__TAURI_CALLS__)) {
        w.__TAURI_CALLS__.push(record);
      }
      if (w.liveIpcFail && w.liveIpcFail.cmd === cmd) {
        throw new Error(w.liveIpcFail.message);
      }

      if (cmd === 'set_gui_preference' && typeof args?.key === 'string') {
        window.localStorage.setItem(`${PREF_PREFIX}${args.key}`, String(args.value ?? ''));
        return null;
      }
      if (cmd === 'get_gui_preference' && typeof args?.key === 'string') {
        const persisted = window.localStorage.getItem(`${PREF_PREFIX}${args.key}`);
        if (persisted !== null) {
          return persisted;
        }
      }
      if (cmd === 'mercatus_save_config') {
        window.localStorage.setItem(MERCATUS_CFG_KEY, JSON.stringify(args?.config ?? null));
        return baseInvoke(cmd, args);
      }
      if (cmd === 'mercatus_load_config') {
        const rawCfg = window.localStorage.getItem(MERCATUS_CFG_KEY);
        if (rawCfg) {
          return JSON.parse(rawCfg) as unknown;
        }
      }
      return baseInvoke(cmd, args);
    };
  });

  return pageErrors;
}

async function readIpcCalls(page: Page): Promise<RecordedIpcCall[]> {
  return page.evaluate(() => {
    const w = window as unknown as { __LIVE_IPC_CALLS__?: RecordedIpcCall[] };
    return (w.__LIVE_IPC_CALLS__ ?? []).slice();
  });
}

test.describe('Milestone v7.4 Live-Control IPC Round-Trip Suite (AXIS-CONCEPT-CV-03)', () => {
  test('1. OratioStudioView (#view=oratio): Rotary knobs, Push-to-Talk transcription & reload persistence', async ({
    page,
  }) => {
    const pageErrors = await installLiveControlHarness(page, 'oratio');
    await page.goto('/#view=oratio');
    await expect(page.getByTestId('oratio-studio-view')).toBeVisible({ timeout: 15_000 });

    const vadKnob = page.getByRole('slider', { name: 'Oratio VAD threshold' });
    const windowKnob = page.getByRole('slider', { name: 'Oratio capture window seconds' });
    await expect(vadKnob).toHaveAttribute('aria-valuenow', '45');
    await expect(windowKnob).toHaveAttribute('aria-valuenow', '5');

    // Keyboard ArrowUp commit on VAD threshold knob (45% -> 50%)
    await vadKnob.focus();
    await page.keyboard.press('ArrowUp');
    await expect(vadKnob).toHaveAttribute('aria-valuenow', '50');
    await expect(vadKnob).toHaveAttribute('aria-valuetext', '50%');

    // Vertical pointer drag commit on Capture Window knob (5s -> >5s)
    const winBox = await windowKnob.boundingBox();
    expect(winBox).not.toBeNull();
    const cx = winBox!.x + winBox!.width / 2;
    const cy = winBox!.y + winBox!.height / 2;
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.move(cx, cy - 34, { steps: 5 });
    await page.mouse.up();
    const updatedSeconds = Number(await windowKnob.getAttribute('aria-valuenow'));
    expect(updatedSeconds).toBeGreaterThan(5);

    // Push-to-Talk capture & deterministic transcript ledger persistence
    const pttBtn = page.getByTestId('oratio-ptt-capture-btn');
    await expect(pttBtn).toContainText(`Push-to-Talk (${updatedSeconds}s)`);
    await pttBtn.click();

    const utteranceRow = page.getByTestId('oratio-utterance-row').first();
    await expect(utteranceRow).toBeVisible();
    await expect(utteranceRow).toContainText('vox check --workspace');

    const calls = await readIpcCalls(page);
    expect(
      calls.some(
        (c) =>
          c.cmd === 'set_gui_preference' &&
          c.args?.key === 'vox.oratio.vad_threshold.v1' &&
          c.args?.value === '50',
      ),
    ).toBe(true);
    expect(
      calls.some(
        (c) =>
          c.cmd === 'set_gui_preference' &&
          c.args?.key === 'vox.oratio.capture_seconds.v1' &&
          c.args?.value === String(updatedSeconds),
      ),
    ).toBe(true);
    expect(
      calls.some((c) => c.cmd === 'oratio_transcribe' && c.args?.seconds === updatedSeconds),
    ).toBe(true);
    expect(
      calls.some((c) => c.cmd === 'set_gui_preference' && c.args?.key === 'vox.oratio.history.v1'),
    ).toBe(true);

    // Reload page and confirm persisted preferences & transcript ledger hydrate cleanly
    await page.reload();
    await expect(page.getByTestId('oratio-studio-view')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('slider', { name: 'Oratio VAD threshold' })).toHaveAttribute(
      'aria-valuenow',
      '50',
    );
    await expect(
      page.getByRole('slider', { name: 'Oratio capture window seconds' }),
    ).toHaveAttribute('aria-valuenow', String(updatedSeconds));
    await expect(page.getByTestId('oratio-utterance-row').first()).toContainText(
      'vox check --workspace',
    );
    expect(pageErrors).toEqual([]);
  });

  test('2. PopuliMeshBentoView (#view=populi) & ImperiumInspector: Gossip/Routing rotary knobs & overlay probe IPC', async ({
    page,
  }) => {
    const pageErrors = await installLiveControlHarness(page, 'populi');
    await page.goto('/#view=populi');
    await expect(page.getByTestId('populi-mesh-bento-view')).toBeVisible({ timeout: 15_000 });

    // Verify Populi topology SVG chart (`[data-cv-chart="true"]`) renders full-width
    const topologyChart = page.locator('[data-testid="populi-quic-topology-svg"][data-cv-chart="true"]');
    await expect(topologyChart).toBeVisible();
    const chartBox = await topologyChart.boundingBox();
    const stageBox = await page.getByTestId('populi-topology-stage').boundingBox();
    expect(chartBox).not.toBeNull();
    expect(stageBox).not.toBeNull();
    expect(chartBox!.width).toBeGreaterThanOrEqual((stageBox!.width - 32) * 0.9);

    // Adjust Gossip Cadence rotary knob (250ms -> 300ms)
    const gossipKnob = page.getByRole('slider', { name: 'Populi gossip heartbeat cadence' });
    await expect(gossipKnob).toHaveAttribute('aria-valuenow', '250');
    await gossipKnob.focus();
    await page.keyboard.press('ArrowUp');
    await expect(gossipKnob).toHaveAttribute('aria-valuenow', '300');
    await expect(page.getByTestId('populi-topology-stage')).toContainText('GOSSIP 300MS');

    // Trigger Run Overlay Probe and verify execute_command(['populi', ...]) IPC round-trip
    const beforeProbeCount = (await readIpcCalls(page)).length;
    await page.getByTestId('populi-run-probe-btn').click();
    await expect
      .poll(async () => {
        const afterCalls = (await readIpcCalls(page)).slice(beforeProbeCount);
        return afterCalls.filter(
          (c) =>
            c.cmd === 'execute_command' &&
            Array.isArray(c.args?.path) &&
            (c.args?.path as string[])[0] === 'populi',
        ).length;
      })
      .toBeGreaterThanOrEqual(2);

    const calls = await readIpcCalls(page);
    expect(
      calls.some(
        (c) =>
          c.cmd === 'set_gui_preference' &&
          c.args?.key === 'vox.populi.gossip_cadence_ms.v1' &&
          c.args?.value === '300',
      ),
    ).toBe(true);

    // Reload and verify gossip cadence restores from persisted preference
    await page.reload();
    await expect(page.getByTestId('populi-mesh-bento-view')).toBeVisible({ timeout: 15_000 });
    await expect(
      page.getByRole('slider', { name: 'Populi gossip heartbeat cadence' }),
    ).toHaveAttribute('aria-valuenow', '300');

    // Also verify Gamify Urbs ImperiumInspector District Model Routing Tier <RomanRotaryKnob>
    await page.goto('/#view=gamify');
    const urbsTab = page.getByTestId('ludus-subtab-urbs');
    if (await urbsTab.isVisible()) {
      await urbsTab.click();
    }
    await expect(page.getByTestId('urbs-landmark-bar')).toBeVisible({ timeout: 15_000 });
    await page.getByTestId('urbs-alerts-history-menu').click();
    await page.getByTestId('urbs-priority-item-diff').first().click();
    const inspector = page.getByTestId('urbs-imperium-inspector');
    await expect(inspector).toBeVisible();

    const routingKnob = inspector.getByRole('slider', { name: 'District Model Routing Tier' });
    await expect(routingKnob).toHaveAttribute('aria-valuenow', '65');
    await routingKnob.focus();
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('ArrowUp');
    await expect(routingKnob).toHaveAttribute('aria-valuenow', '75');
    await expect(inspector).toContainText('Frontier Cloud');
    await expect(inspector).toContainText(
      /Exploration routing persisted: 75%|PRAETORIVM routing posture: QUALITY/,
    );

    const gamifyCalls = await readIpcCalls(page);
    expect(
      gamifyCalls.some(
        (c) =>
          c.cmd === 'set_gui_preference' &&
          c.args?.key === 'vox.routing.exploration_pct.v1' &&
          c.args?.value === '75',
      ),
    ).toBe(true);
    expect(gamifyCalls.some((c) => c.cmd === 'set_routing_priority')).toBe(true);
    expect(pageErrors).toEqual([]);
  });

  test('3. RunsView / Models / Dashboard: Clickable RomanBrassGauge, VoxSelect combobox & Recharts Brush', async ({
    page,
  }) => {
    const pageErrors = await installLiveControlHarness(page, 'runs');
    await page.goto('/#view=runs');
    await expect(
      page.getByRole('heading', { name: /Workflow Runs & Model Scoreboard/i }),
    ).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('#gui-run-1')).toBeVisible();
    await expect(page.getByText('#gui-run-4')).toBeVisible();

    // Click RomanBrassGauge ("Failed") in header frieze and verify active filter toggles cleanly
    const failedGauge = page
      .locator('header button[data-roman-gauge="true"]')
      .filter({ hasText: 'Failed' });
    await expect(failedGauge).toHaveAttribute('aria-pressed', 'false');
    await failedGauge.click();
    await expect(failedGauge).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByText('#gui-run-4')).toBeVisible();
    await expect(page.getByText('#gui-run-1')).toHaveCount(0);

    await failedGauge.click();
    await expect(failedGauge).toHaveAttribute('aria-pressed', 'false');
    await expect(page.getByText('#gui-run-1')).toBeVisible();

    // Exercise VoxSelect (`[data-slot="select-trigger"]` -> `[role="listbox"]`) on Models routing
    await page.goto('/#view=models');
    const routingSelect = page.locator('#models-routing-mode[data-slot="select-trigger"]');
    await expect(routingSelect).toBeVisible({ timeout: 15_000 });
    await routingSelect.click();
    const listbox = page.locator('[role="listbox"]');
    await expect(listbox).toBeVisible();
    await listbox.getByRole('option', { name: /balanced/i }).click();
    await expect(routingSelect).toContainText(/balanced/i);

    const modelCalls = await readIpcCalls(page);
    expect(
      modelCalls.some((c) => c.cmd === 'explain_routing' && c.args?.mode === 'balanced'),
    ).toBe(true);

    // Navigate to #view=dashboard, stream >=6 live orchestrator metric samples, add Recharts widgets, verify Brush + series click
    await page.goto('/#view=dashboard');
    await expect(page.getByTestId('dashboard-kpi-strip')).toBeVisible({ timeout: 15_000 });

    const baseStatus = buildRichOrchestratorStatus();
    for (const [queued, cost] of [
      [12, 3.4],
      [18, 5.1],
      [25, 7.8],
      [31, 9.6],
      [39, 11.2],
      [44, 12.34],
    ] as const) {
      await page.evaluate(
        ([ev, payload]) => {
          const w = window as unknown as {
            __TAURI_EMIT__?: (event: string, data: unknown) => void;
          };
          w.__TAURI_EMIT__?.(ev, payload);
        },
        ['vox://orch-status', { ...baseStatus, total_queued: queued, total_cost: cost }] as const,
      );
      await page.evaluate(() => new Promise<void>((r) => requestAnimationFrame(() => r())));
    }

    await page.getByRole('button', { name: 'Customize dashboard' }).click();
    await page.getByRole('button', { name: 'Add widget' }).click();
    const picker = page.getByRole('dialog', { name: 'Add dashboard widget' });
    await picker.getByRole('button', { name: 'Budget Burn', exact: true }).click();
    await page.getByRole('button', { name: 'Add widget' }).click();
    await picker.getByRole('button', { name: 'Queue Depth', exact: true }).click();
    await page.getByRole('button', { name: 'Done customizing' }).click();

    const queueChartFrame = page.getByRole('img', { name: 'Queue Depth chart' });
    await queueChartFrame.scrollIntoViewIfNeeded();
    await expect(queueChartFrame).toBeVisible();

    const rechartsWrapper = queueChartFrame.locator('.recharts-wrapper').first();
    const brush = queueChartFrame.locator('.recharts-brush').first();
    const travellers = queueChartFrame.locator('.recharts-brush-traveller');
    await expect(brush).toBeVisible();
    await expect(travellers).toHaveCount(2);

    const frameBox = await queueChartFrame.boundingBox();
    const wrapperBox = await rechartsWrapper.boundingBox();
    expect(frameBox).not.toBeNull();
    expect(wrapperBox).not.toBeNull();
    expect(wrapperBox!.width).toBeGreaterThanOrEqual(frameBox!.width * 0.9);

    // Drag a Recharts <Brush> traveller handle and click a bar series datum without error
    const travBox = await travellers.first().boundingBox();
    expect(travBox).not.toBeNull();
    await page.mouse.move(travBox!.x + travBox!.width / 2, travBox!.y + travBox!.height / 2);
    await page.mouse.down();
    await page.mouse.move(travBox!.x + 18, travBox!.y + travBox!.height / 2, { steps: 3 });
    await page.mouse.up();

    const barTarget = queueChartFrame.locator('.recharts-bar-rectangle path').last();
    await barTarget.click({ force: true });
    await expect(page.getByTestId('chart-selected-pill')).toBeVisible();
    expect(pageErrors).toEqual([]);
  });

  test('4. Mercatus (#view=mercatus): Live Switch / threshold save IPC & zero fake_success honesty check', async ({
    page,
  }) => {
    const pageErrors = await installLiveControlHarness(page, 'mercatus');
    await page.goto('/#view=mercatus');
    await expect(page.getByTestId('mercatus-part-row-rtx-4090-24gb')).toBeVisible({
      timeout: 15_000,
    });

    const vastSwitch = page.locator(
      '[data-testid="mercatus-source-switch-vast_ai"][data-slot="switch"]',
    );
    await expect(vastSwitch).toBeVisible();
    await expect(vastSwitch).toHaveAttribute('aria-checked', 'true');

    // Toggle source <Switch> (`vast_ai` -> unchecked) and update target USD ceiling (`0.39` -> `0.45`)
    await vastSwitch.click();
    await expect(vastSwitch).toHaveAttribute('aria-checked', 'false');

    const targetInput = page.getByTestId('mercatus-target-usd-rtx-4090-24gb');
    await targetInput.fill('0.45');
    await expect(page.getByTestId('mercatus-inspector-pane')).toContainText('$0.45');

    const calls = await readIpcCalls(page);
    const saveCalls = calls.filter((c) => c.cmd === 'mercatus_save_config');
    expect(saveCalls.length).toBeGreaterThanOrEqual(2);
    const latestSaved = saveCalls[saveCalls.length - 1]?.args?.config as
      | Record<string, unknown>
      | undefined;
    expect(latestSaved).toBeDefined();
    const savedMeta = latestSaved?._meta as
      | { sources?: Record<string, { enabled?: boolean }> }
      | undefined;
    const savedWatchlist = latestSaved?.watchlist as
      | Array<{ id: string; target_usd?: number }>
      | undefined;
    expect(savedMeta?.sources?.vast_ai?.enabled).toBe(false);
    expect(savedWatchlist?.find((p) => p.id === 'rtx-4090-24gb')?.target_usd).toBe(0.45);

    // Reload and verify saved Mercatus config restores identically
    await page.reload();
    await expect(page.getByTestId('mercatus-part-row-rtx-4090-24gb')).toBeVisible({
      timeout: 15_000,
    });
    await expect(
      page.locator('[data-testid="mercatus-source-switch-vast_ai"][data-slot="switch"]'),
    ).toHaveAttribute('aria-checked', 'false');
    await expect(page.getByTestId('mercatus-target-usd-rtx-4090-24gb')).toHaveValue('0.45');

    // Verify checkHonestyInvariant + DOM honesty (0 `_demo` tokens, 0 `fake_success` notices, 0 error banners)
    const honestyViolation = checkHonestyInvariant({
      status: 'completed',
      payload: latestSaved,
      requiredFields: ['watchlist', '_meta'],
      stageName: 'mercatus_save_config',
    });
    expect(honestyViolation).toBeNull();

    const domHonesty = await page.evaluate(() => {
      const bodyText = document.body.innerText || '';
      const demoTokenMatches = bodyText.match(/\b_demo\b|demo_stub/gi) ?? [];
      const fakeSuccessMatches = bodyText.match(/\bfake_success\b/gi) ?? [];
      const errorBanners = document.querySelectorAll('[data-testid="mercatus-error-banner"]').length;
      return {
        demoTokenCount: demoTokenMatches.length,
        fakeSuccessCount: fakeSuccessMatches.length,
        errorBanners,
      };
    });
    expect(domHonesty).toEqual({
      demoTokenCount: 0,
      fakeSuccessCount: 0,
      errorBanners: 0,
    });
    expect(pageErrors).toEqual([]);
  });

  test('5. Optimistic state rollback & sanitizeErrorForToast leak guard on rejected IPC', async ({
    page,
  }) => {
    const dialogAlerts: string[] = [];
    page.on('dialog', (dlg) => {
      dialogAlerts.push(dlg.message());
      void dlg.dismiss();
    });

    const pageErrors = await installLiveControlHarness(page, 'oratio');
    await page.goto('/#view=oratio');
    await expect(page.getByTestId('oratio-studio-view')).toBeVisible({ timeout: 15_000 });

    const vadKnob = page.getByRole('slider', { name: 'Oratio VAD threshold' });
    await expect(vadKnob).toHaveAttribute('aria-valuenow', '45');

    await page.evaluate(() => {
      (window as unknown as { liveIpcFail?: { cmd: string; message: string } }).liveIpcFail = {
        cmd: 'set_gui_preference',
        message: 'window.__TAURI_INTERNALS__.invoke failed: disk full at stack\n    at internal:1:1',
      };
    });

    await vadKnob.focus();
    await page.keyboard.press('ArrowUp');

    // 1. ARIA state rolls back to prior confirmed value (45%)
    await expect(vadKnob).toHaveAttribute('aria-valuenow', '45');
    await expect(vadKnob).toHaveAttribute('aria-valuetext', '45%');

    // 2. Warning toast appears with sanitized error & zero raw Tauri internals leaked
    const rollbackToast = page
      .locator('[data-testid="toast-stack"] [data-slot="toast"]')
      .filter({ hasText: /VAD threshold rollback/i });
    await expect(rollbackToast).toBeVisible();
    await expect(rollbackToast).toContainText('An unexpected error occurred.');
    const toastText = await page.getByTestId('toast-stack').innerText();
    const docText = await page.evaluate(() => document.body.innerText || '');
    expect(toastText).not.toContain('__TAURI_INTERNALS__');
    expect(docText).not.toContain('__TAURI_INTERNALS__');

    // 3. Honesty invariant + 0 alerts + 0 pageerrors
    const honestyViolation = checkHonestyInvariant({
      status: 'failed',
      payload: null,
      requiredFields: ['vad_threshold'],
      stageName: 'set_gui_preference',
    });
    expect(honestyViolation).toBeNull();
    await expect(page.locator('[role="alert"]')).toHaveCount(0);
    expect(dialogAlerts).toEqual([]);
    expect(pageErrors).toEqual([]);
  });
});
