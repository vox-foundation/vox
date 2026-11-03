import { test, expect } from '@playwright/test';
import {
  launchResearchSession,
  toggleDiagnosticsProber,
  submitProbeQuery,
  openInspectorDrawer,
  stepNextStage,
  selectStageInDrawer,
  captureStageArtifact,
} from './research-hitl-driver';

test.describe('HITL Research Debugger & Mathematical Observability Walkthrough', () => {
  test('Phase 1 through Phase 5 interactive verification', async ({ page }) => {
    // ── Phase 1: Planning & Diagnostic Prober ──
    await launchResearchSession(page);
    await toggleDiagnosticsProber(page);
    await submitProbeQuery(page, 'hybrid search architecture');

    // Assert 200 badges across providers
    const statusBadges = page.getByTestId('status-badge-200');
    await expect(statusBadges.first()).toBeVisible({ timeout: 10_000 });
    expect(await statusBadges.count()).toBeGreaterThanOrEqual(1);

    // Open Inspector Drawer
    await openInspectorDrawer(page);

    // Advance to Stage 1: planning
    await stepNextStage(page);
    await selectStageInDrawer(page, 'planning');

    // Capture Stage 1 artifact
    await captureStageArtifact(page, 1, 'planning');

    // ── Phase 2: Retrieval & Reciprocal Rank Fusion ──
    await stepNextStage(page); // transitions to retrieving
    await selectStageInDrawer(page, 'retrieving');

    // Verify retrieving stage is active or completed
    const retrievingBtn = page.getByTestId('inspector-drawer').getByRole('button', { name: /^retrieving/i }).first();
    await expect(retrievingBtn).toBeVisible();

    // Capture Stage 2 artifact
    await captureStageArtifact(page, 2, 'retrieving');

    // ── Phase 3: Claim Extraction & Grounding ──
    await stepNextStage(page); // transitions to verifying_claims
    await selectStageInDrawer(page, 'verifying_claims');

    // Temporarily close drawer so main view has unoccluded interaction
    await page.keyboard.press('Control+Shift+D');
    await expect(page.getByTestId('inspector-drawer')).toBeHidden({ timeout: 5_000 });

    // In ResearchView, select the completed session to view claims accordion
    const sessionItem = page.locator('button').filter({ hasText: 'vector db tradeoffs' }).first();
    if (await sessionItem.isVisible()) {
      await sessionItem.click();
    }

    // Verify claims accordion is visible in ResearchView
    const claimAccordion = page.getByText(/Vector DBs trade exact recall/i).first();
    await expect(claimAccordion).toBeVisible({ timeout: 5_000 });
    await claimAccordion.scrollIntoViewIfNeeded();

    // Capture Stage 3 artifact (Full Research View with Claims Accordion)
    await captureStageArtifact(page, 3, 'verifying-claims');

    // ── Phase 4: Epistemic Judge & Rubric Transparency ──
    // Toggle Judge Inspector in session detail
    const toggleJudgeBtn = page.getByTestId('toggle-judge-btn');
    await expect(toggleJudgeBtn).toBeVisible({ timeout: 5_000 });
    await toggleJudgeBtn.click();

    // Verify Judge Inspector is visible with badges
    const judgeInspector = page.getByTestId('judge-inspector');
    await expect(judgeInspector).toBeVisible();
    await expect(page.getByTestId('metric-confidence-tier')).toHaveText('DeepResearch');
    await expect(page.getByTestId('metric-source-count')).toHaveText('3');
    await expect(page.getByTestId('count-supported')).toHaveText('2');
    await expect(page.getByTestId('count-contested')).toHaveText('1');
    await judgeInspector.scrollIntoViewIfNeeded();

    // Capture Stage 4 artifact (Full Epistemic Judge Breakdown)
    await captureStageArtifact(page, 4, 'judge-inspector');

    // ── Phase 5: Synthesis & Invariant Sentry Audit ──
    // Reopen Inspector Drawer
    await openInspectorDrawer(page);

    // Step through synthesizing, auditing_citations, persisting to completed
    await stepNextStage(page); // synthesizing
    await stepNextStage(page); // auditing_citations
    await stepNextStage(page); // persisting
    await stepNextStage(page); // completed
    await selectStageInDrawer(page, 'completed');

    // Click Audit button in drawer
    const auditBtn = page.getByRole('button', { name: 'Audit Invariants' });
    await expect(auditBtn).toBeVisible();
    await auditBtn.click();

    // Switch to Violations tab
    const violationsTab = page.getByRole('button', { name: /violations/i });
    await violationsTab.click();

    // Assert zero invariant violations reported
    await expect(page.getByText('No invariant violations')).toBeVisible();
    await expect(page.getByText('Pipeline sentries report clean state')).toBeVisible();

    // Capture Stage 5 artifact (Sentry Audit Clear)
    await captureStageArtifact(page, 5, 'sentry-audit');
  });
});
