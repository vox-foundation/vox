/**
 * Interactive Playwright Dogfooding Suite for Vox Axis (`v1.6` Milestone).
 *
 * Drives the real React + Dockview + Canvas 2D Axis Workbench across four
 * canonical developer journeys and captures visual evidence:
 *  1. Working on Vox itself: docking the Harness Live River panel in Chat,
 *     attaching `crates/vox-gui/src/main.rs` via `@`-mention autocomplete,
 *     opening the live Routing drawer via `Routing & Tiers`, toggling
 *     DriveConsole Clutch (Genius) + Risk (Low / Socratic Red-Team),
 *     pinning `mens/e2e-smoke-metal` via "Choose model tier" and verifying
 *     live Tier 2 Fan-Out updates in the Harness River, and submitting both
 *     a sync quick-chat turn and a background orchestrator task (`bg-task-*`).
 *  2. Working on another repository (`acme/payments-service`): docking the
 *     Repository & Harness panels side-by-side in Chat, inspecting active
 *     multi-agent conflict rows, navigating to the Repository surface to
 *     switch the default VCS isolation strategy to `separate_branches` via
 *     the confirmation modal, running `Repo health check` and `vox diff`,
 *     and dispatching an isolated multi-agent fix task.
 *  3. Performing Deep Research & epistemic synthesis: driving the Research
 *     surface (Fast -> Deep Lane switch, provider quota strip, Research Engine
 *     Drawer & Zero-Key Guarantee, Live Source Prober query, and corroborated
 *     claim inspection), then switching to Chat with `Research mode: Deep Research`
 *     and submitting a synthesis prompt that threads `force_research: true` and
 *     `research_scope: 'deep'` into `chat_turn`.
 *  4. Gamify Ludus Sandbox: toggling between `Urbs Map` and `Live River`
 *     simulation mode and verifying the Ludus Crystal yield badge.
 */
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';

const DOGFOOD_SHOT_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  '..',
  'review-bundle',
  'dogfood',
);

type Call = { cmd: string; args?: any };

async function getCalls(page: Page, pred: (c: Call) => boolean): Promise<Call[]> {
  return ((await page.evaluate(() => (window as any).__TAURI_CALLS__)) as Call[]).filter(pred);
}

async function submitComposer(page: Page, text: string): Promise<void> {
  const composer = page.getByLabel('Task composer');
  await expect(composer).toBeVisible();
  await composer.fill(text);
  await composer.press('Enter');
}

test.describe('Vox Axis Interactive Developer Dogfooding (Playwright E2E)', () => {
  test.beforeAll(() => {
    mkdirSync(DOGFOOD_SHOT_DIR, { recursive: true });
  });

  test('Scenario 1: Working on Vox itself — docked Live River, @-file context, Routing drawer, DriveConsole clutch/risk, model pin, and sync + background chat turns', async ({
    page,
  }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(err.message));

    await addMockInitScript(page, installTauriMock, 'chat');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    // 1. Open Panels menu and dock the Harness Live River panel beside Chat
    await page.getByRole('button', { name: /panels/i }).click();
    const harnessCheckbox = page.getByRole('checkbox', { name: /^harness$/i });
    await expect(harnessCheckbox).toBeVisible();
    await harnessCheckbox.click();
    await page.keyboard.press('Escape');

    const harnessDock = page.getByTestId('chat-dock-harness');
    await expect(harnessDock).toBeVisible();
    await expect(page.getByTestId('axis-harness-river')).toBeVisible();
    await expect(page.getByTestId('harness-river-canvas')).toBeVisible();

    // 2. Click "Routing & Tiers" inside HarnessRiverPanel to inspect the Routing drawer, then close it
    await harnessDock.getByRole('button', { name: /Routing & Tiers/i }).click();
    const routingDialog = page.getByRole('dialog', { name: 'Routing' });
    await expect(routingDialog).toBeVisible();
    await routingDialog.getByRole('button', { name: 'Close routing panel' }).click();
    await expect(routingDialog).not.toBeVisible();

    // 3. Toggle DriveConsole Clutch -> Genius and Risk -> Low (Strict Socratic Red-Team)
    const geniusRadio = page.getByRole('radio', { name: /genius/i });
    await expect(geniusRadio).toBeVisible();
    await geniusRadio.click();
    await expect(geniusRadio).toHaveAttribute('aria-checked', 'true');

    const riskTrigger = page.getByRole('button', { name: /Risk: .* — click to configure/i });
    await riskTrigger.click();
    await page.getByRole('button', { name: /^Low risk/i }).click();

    // Verify docked Harness River reactively updates Tier 3 Verifier to SOCRATIC RED-TEAM
    await expect(page.getByTestId('tier-card-verifier')).toContainText(/SOCRATIC RED-TEAM/i);

    // 4. Pick a concrete model in Loquela ("Choose model tier") and verify Tier 2 Fan-Out updates live
    await page.getByRole('button', { name: /choose model tier/i }).click();
    const mensModel = page.getByText('mens/e2e-smoke-metal').first();
    await expect(mensModel).toBeAttached();
    await mensModel.scrollIntoViewIfNeeded();
    await mensModel.click();

    await expect(page.getByTestId('tier-card-fanout')).toContainText('Pinned → mens/e2e-smoke-metal');

    // 5. Attach a Vox crate file via @-mention autocomplete in the composer
    const composer = page.getByLabel('Task composer');
    await composer.fill('@crates/');
    const fileOption = page.getByRole('button', { name: /crates\/vox-gui\/src\/main\.rs/i });
    await expect(fileOption).toBeVisible({ timeout: 5_000 });
    await fileOption.click();
    await expect(page.getByRole('button', { name: 'Remove crates/vox-gui/src/main.rs' })).toBeVisible();

    // 6. Submit a Vox compiler quick-chat developer query
    const voxPrompt =
      'Audit crates/vox-compiler/src/typeck/checker.rs for exhaustive match invariants and suggest a property test';
    await submitComposer(page, voxPrompt);

    await expect(page.getByText(voxPrompt)).toBeVisible();
    await expect(page.getByText('Mock quick-chat reply.')).toBeVisible();

    const syncTurns = await getCalls(page, (c) => c.cmd === 'chat_turn');
    expect(syncTurns.length).toBe(1);
    expect(syncTurns[0].args.input).toMatchObject({
      execution: 'sync',
      content: voxPrompt,
      model_override: 'mens/e2e-smoke-metal',
      context_files: ['crates/vox-gui/src/main.rs'],
    });

    await page.screenshot({ path: join(DOGFOOD_SHOT_DIR, '01-vox-self-dogfood-chat-harness.png') });

    // 7. Switch send mode to Background task and dispatch a multi-agent verification task
    await page.getByRole('button', { name: 'Choose send mode' }).click();
    await page.getByRole('button', { name: 'Set send mode: Background task' }).click();

    const bgPrompt = 'Run cargo test -p vox-compiler and verify AST diff hashes across 15 parallel branches';
    await submitComposer(page, bgPrompt);

    await expect
      .poll(async () => (await getCalls(page, (c) => c.cmd === 'chat_turn')).length)
      .toBe(2);
    const allTurns = await getCalls(page, (c) => c.cmd === 'chat_turn');
    expect(allTurns[1].args.input.execution).toBe('background');
    expect(allTurns[1].args.input.session_id).toMatch(/^bg-task-/);

    expect(pageErrors).toEqual([]);
  });

  test('Scenario 2: Working on another repository — VCS multi-agent isolation, conflict triage, repo health check, and isolated task dispatch', async ({
    page,
  }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(err.message));

    await addMockInitScript(page, installTauriMock, 'chat');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    // 1. Open Panels menu and dock both Repository and Harness panels
    await page.getByRole('button', { name: /panels/i }).click();
    await page.getByRole('checkbox', { name: /^repository$/i }).click();
    await page.getByRole('checkbox', { name: /^harness$/i }).click();
    await page.keyboard.press('Escape');

    const repoDock = page.getByTestId('chat-dock-repository');
    await expect(repoDock).toBeVisible();

    // 2. Verify active conflict detection on external repo `acme/payments-service`
    await expect(repoDock).toContainText(/1 active conflict|acme\/payments-service\/src\/webhooks\/retry\.rs/i);

    // 3. Navigate via Sidebar (Expand Workspace -> Repository) to change VCS isolation strategy and run repo checks
    await page.getByRole('button', { name: /Expand Workspace/i }).click();
    await page.getByRole('button', { name: /^Repository$/i }).click();

    await expect(page.getByText('acme/payments-service/src/webhooks/retry.rs')).toBeVisible();

    // Switch default isolation strategy from shared_branch -> separate_branches via confirmation dialog
    const strategySelect = page.locator('#isolation-default');
    await expect(strategySelect).toBeVisible();
    await strategySelect.selectOption('separate_branches');

    const confirmDialog = page.getByRole('dialog');
    await expect(confirmDialog).toBeVisible();
    await confirmDialog.getByRole('button', { name: /change strategy/i }).click();

    await expect
      .poll(async () => (await getCalls(page, (c) => c.cmd === 'set_vcs_isolation_strategy')).length)
      .toBe(1);

    // Run `Repo health check` and `vox diff` against the repository
    await page.getByRole('button', { name: 'Repo health check' }).click();
    await expect
      .poll(
        async () =>
          (
            await getCalls(
              page,
              (c) => c.cmd === 'execute_command' && Array.isArray(c.args?.path) && c.args.path.join(' ') === 'check workspace',
            )
          ).length,
      )
      .toBe(1);

    await page.getByRole('button', { name: '`vox diff`' }).click();
    await expect
      .poll(
        async () =>
          (
            await getCalls(
              page,
              (c) => c.cmd === 'execute_command' && Array.isArray(c.args?.path) && c.args.path.join(' ') === 'diff',
            )
          ).length,
      )
      .toBe(1);

    await page.screenshot({ path: join(DOGFOOD_SHOT_DIR, '02-external-repo-isolation.png') });

    // Return to Chat via Sidebar and dispatch an isolated fix task for acme/payments-service
    await page.getByRole('button', { name: /^Chat$/i }).first().click();
    await submitComposer(
      page,
      'Resolve webhook retry conflict in acme/payments-service/src/webhooks/retry.rs on separate_branches',
    );
    await expect
      .poll(async () => (await getCalls(page, (c) => c.cmd === 'chat_turn')).length)
      .toBe(1);

    expect(pageErrors).toEqual([]);
  });

  test('Scenario 3: Performing Deep Research & epistemic synthesis — Dual Lanes, Engine Drawer, Live Source Prober, and Deep Research Chat turn', async ({
    page,
  }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(err.message));

    await addMockInitScript(page, installTauriMock, 'research');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    // 1. Switch Research Lane from Fast -> Deep
    const fastBtn = page.getByTestId('lane-switch-fast');
    const deepBtn = page.getByTestId('lane-switch-deep');
    await expect(fastBtn).toBeVisible({ timeout: 10_000 });
    await deepBtn.click();
    await expect(deepBtn).toHaveAttribute('aria-selected', 'true');

    // 2. Verify Provider Quota Strip & Open Research Engine Drawer
    const badgeStrip = page.getByTestId('provider-badge-strip');
    await expect(badgeStrip).toContainText('Wikipedia');
    await expect(badgeStrip).toContainText('Tavily');

    await page.getByTestId('configure-engines-btn').click();
    const drawerOverlay = page.getByTestId('research-engine-drawer-overlay');
    await expect(drawerOverlay).toBeVisible();
    await expect(page.getByTestId('zero-key-guarantee')).toContainText('Zero-Key Guarantee');
    await page.keyboard.press('Escape');
    await expect(drawerOverlay).not.toBeVisible();

    // 3. Toggle Live Source Prober Diagnostics and execute a live retrieval probe
    await page.getByTestId('toggle-diagnostics-btn').click();
    const prober = page.getByTestId('live-source-prober');
    await expect(prober).toBeVisible();

    const probeInput = page.getByTestId('probe-query-input');
    await probeInput.fill('Reciprocal Rank Fusion k=60 vs ColBERTv2 late interaction');
    await page.getByTestId('probe-submit-btn').click();
    await expect(page.getByTestId('status-badge-200').first()).toBeVisible({ timeout: 10_000 });

    await page.screenshot({ path: join(DOGFOOD_SHOT_DIR, '03-deep-research-prober.png') });

    // 4. Switch to Chat tab and set Research Mode to Deep Research for synthesis
    const chatTab = page.getByRole('tab', { name: /chat/i }).first();
    if (await chatTab.isVisible()) {
      await chatTab.click();
    } else {
      await page.getByRole('button', { name: /^chat$/i }).first().click();
    }

    const researchDropdown = page.getByRole('button', { name: /Research mode:/i });
    await expect(researchDropdown).toBeVisible();
    await researchDropdown.click();
    await page.getByRole('button', { name: /Set research mode: Deep Research/i }).click();
    await expect(page.getByRole('button', { name: /Research mode: Deep Research/i })).toBeVisible();

    const researchPrompt =
      'Synthesize our CRAG + RRF retrieval benchmark findings into a formal ADR for vox-search';
    await submitComposer(page, researchPrompt);

    await expect
      .poll(async () => (await getCalls(page, (c) => c.cmd === 'chat_turn')).length)
      .toBe(1);
    const [turn] = await getCalls(page, (c) => c.cmd === 'chat_turn');
    expect(turn.args.input).toMatchObject({
      content: researchPrompt,
      force_research: true,
      research_scope: 'deep',
    });

    expect(pageErrors).toEqual([]);
  });

  test('Scenario 4: Gamify Ludus Sandbox — switching between Urbs Map and Live River simulation mode', async ({
    page,
  }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(err.message));

    await addMockInitScript(page, installTauriMock, 'gamify');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 15_000 });

    const urbsBtn = page.getByTestId('gamify-sim-mode-urbs');
    const riverBtn = page.getByTestId('gamify-sim-mode-river');
    await expect(urbsBtn).toBeVisible({ timeout: 10_000 });
    await expect(riverBtn).toBeVisible();

    await riverBtn.click();
    await expect(page.getByTestId('gamify-live-river-container')).toBeVisible();
    await expect(page.getByTestId('axis-harness-river')).toBeVisible();
    await page.getByTestId('ludus-crystal-yield').scrollIntoViewIfNeeded();
    await page.screenshot({ path: join(DOGFOOD_SHOT_DIR, '04-gamify-live-river.png') });

    expect(pageErrors).toEqual([]);
  });
});
