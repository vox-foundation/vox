/**
 * Dual-Surface Roman Scriptorium Terminal CV Flywheel (`TERM-ROMA-CV-03`).
 *
 * Sweeps 5 viewports x 4 themes across Console + Chat TerminalDockPanel and all
 * 8 TERMINAL_CV_STATES using @ngi/platform-ssot pixel metrics, Axe WCAG 2.x,
 * DOM layout audits, and terminal-specific invariants (`terminalAudits.ts`).
 */

import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { existsSync, mkdirSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  PLATFORM_SSOT_SHA256,
  PLATFORM_SSOT_VERSION,
  computeRgbaFrameCvMetrics,
  evaluateGuiSurfaceRubric,
  runUniversalCvFlywheelPass,
  type GuiDomAuditMetrics,
  type UniversalCvFlywheelVerdict,
  type VisionRubricScores,
} from '../../src/lib/cvFlywheel';
import { installTauriMock } from '../lib/tauriMock';
import { addRichMockInitScript } from '../lib/tauriMockRich';
import { addMockInitScript } from '../lib/tauriMockShared';
import { auditDomQualityInPage, auditIconsInPage, auditOverflowInPage } from './audits';
import { TERMINAL_CV_STATES, type TerminalCvStateSpec } from './states';
import { runTerminalDomAudits, type TerminalAuditMetrics } from './terminalAudits';
import { evaluateVlmSurfaceCritique, type VlmSurfaceCritique } from './vlmVisionFlywheel';

const __dirname = dirname(fileURLToPath(import.meta.url));
const UI_ROOT = join(__dirname, '..', '..');
const OUT_DIR = join(UI_ROOT, 'test-results', 'cv-flywheel-terminal');
const SHOTS_DIR = join(OUT_DIR, 'screenshots');
const PARTS_DIR = join(OUT_DIR, 'parts');
const STAGE: 'before' | 'after' =
  (process.env.CV_STAGE ?? 'after').toLowerCase() === 'before' ? 'before' : 'after';
const BEFORE_LEDGER_FILE = join(OUT_DIR, 'ledger-before.json');
const LEDGER_FILE = join(OUT_DIR, `ledger-${STAGE}.json`);
const RUN_FLYWHEEL = process.env.VOX_CV_FLYWHEEL === '1';

const TERMINAL_VIEWPORTS = [
  { name: '900x600', width: 900, height: 600 },
  { name: '1100x720', width: 1100, height: 720 },
  { name: '1280x800', width: 1280, height: 800 },
  { name: '1440x900', width: 1440, height: 900 },
  { name: '1920x1080', width: 1920, height: 1080 },
] as const;

const TERMINAL_THEMES = ['arcane', 'slate', 'travertine', 'high-contrast'] as const;
const CORE_5X4_STATES = new Set([
  'block-stream-ansi',
  'split-panes',
  'auto-tui-alt-screen',
  'chat-terminal-dock',
]);
const BOUND_VIEWPORTS = new Set(['900x600', '1440x900']);

interface AxeNodeSummary {
  id: string;
  impact: string | null | undefined;
  targets: string[];
  summary: string;
}

interface LedgerEntry {
  key: string;
  stage: 'before' | 'after';
  surface: 'console' | 'chat';
  state: string;
  theme: (typeof TERMINAL_THEMES)[number];
  viewport: string;
  screenshotPath: string;
  pixelSignals: { edgeSharpness: number; luminanceContrast: number; noiseFloor: number; meanLuminance: number };
  metrics: GuiDomAuditMetrics;
  axeDetails: AxeNodeSummary[];
  terminalSamples: string[];
  vlmRubric: VisionRubricScores;
  weightedRubricScore: number;
  visualBeforeScore: number;
  visualAfterScore: number;
  flywheelConverged: boolean;
  vlmOverallScore: number;
  vlmCritique: VlmSurfaceCritique;
  verdict: UniversalCvFlywheelVerdict;
  failures: string[];
}

function viewportsForState(spec: TerminalCvStateSpec) {
  return CORE_5X4_STATES.has(spec.name)
    ? TERMINAL_VIEWPORTS
    : TERMINAL_VIEWPORTS.filter((v) => BOUND_VIEWPORTS.has(v.name));
}

function computeWeightedScore(r: VisionRubricScores): number {
  return Number(
    (
      0.25 * r.technicalQuality +
      0.3 * r.promptAdherence +
      0.2 * r.anatomicalAccuracy +
      0.15 * r.compositionQuality +
      0.1 * r.styleConsistency
    ).toFixed(4),
  );
}

async function decodePngRgbaInPage(page: Page, pngBuffer: Buffer) {
  const b64 = pngBuffer.toString('base64');
  const raw = await page.evaluate(async (enc) => {
    const res = await fetch(`data:image/png;base64,${enc}`);
    const bmp = await createImageBitmap(await res.blob());
    try {
      const w = Math.max(8, Math.floor(bmp.width * 0.25));
      const h = Math.max(8, Math.floor(bmp.height * 0.25));
      const c = document.createElement('canvas');
      c.width = w;
      c.height = h;
      const ctx = c.getContext('2d');
      if (!ctx) throw new Error('Failed to acquire 2d canvas context for PNG RGBA decode');
      ctx.drawImage(bmp, 0, 0, w, h);
      return { w, h, bytes: Array.from(ctx.getImageData(0, 0, w, h).data) };
    } finally {
      bmp.close();
    }
  }, b64);
  return computeRgbaFrameCvMetrics(new Uint8Array(raw.bytes), raw.w, raw.h);
}

async function collectCombinedMetrics(
  page: Page,
  surfaceId: string,
): Promise<{ metrics: GuiDomAuditMetrics; terminal: TerminalAuditMetrics; axeDetails: AxeNodeSummary[] }> {
  let axeViolations = 0;
  let axeDetails: AxeNodeSummary[] = [];
  try {
    const axe = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .options({ resultTypes: ['violations'] })
      .analyze();
    const serious = axe.violations.filter((v) => ['serious', 'critical'].includes(v.impact ?? ''));
    axeViolations = serious.length;
    axeDetails = serious.map((v) => ({
      id: v.id,
      impact: v.impact,
      targets: v.nodes.slice(0, 4).map((n) => n.target.join(' ')),
      summary: v.nodes[0]?.failureSummary ?? v.help,
    }));
  } catch (err) {
    axeViolations = 1;
    axeDetails = [
      { id: 'axe-runner-error', impact: 'critical', targets: [], summary: String(err) },
    ];
  }

  const dom = await page.evaluate(auditDomQualityInPage, surfaceId);
  const icons = await page.evaluate(auditIconsInPage);
  const overflow = await page.evaluate(auditOverflowInPage);
  const terminal = await runTerminalDomAudits(page);

  return {
    terminal,
    axeDetails,
    metrics: {
      surfaceId,
      outerOverflowPx: Math.max(dom.outerOverflowPx, overflow.bodyHorizontalOverflowPx),
      scrollHostOverflowPx: Math.max(dom.scrollHostOverflowPx, overflow.scrollHostHorizontalOverflowPx),
      clippedInteractiveCount: dom.clippedInteractiveCount,
      overlappingControlPairs: dom.overlappingControlPairs,
      unstyledNativeSelectCount: dom.unstyledNativeSelectCount,
      lightThemeContrastLeakCount: dom.lightThemeContrastLeakCount,
      iconIssueCount: icons.length,
      controlHeightSpreadPx: dom.controlHeightSpreadPx,
      rawEmojiCount: dom.rawEmojiCount,
      unlitLandmarkCount: dom.unlitLandmarkCount,
      missingDockPanelIds: dom.missingDockPanelIds,
      viewportCoverageRatio: dom.viewportCoverageRatio,
      composerToolbarRowCount: dom.composerToolbarRowCount,
      composerToolbarHeightPx: dom.composerToolbarHeightPx,
      duplicateControlGroupCount: dom.duplicateControlGroupCount,
      skeletonCount: dom.skeletonCount,
      rawBacktickCount: dom.rawBacktickCount,
      digitalColorLeakCount: dom.digitalColorLeakCount,
      chartPanelFillRatio: dom.chartPanelFillRatio,
      bracketedLabelCount: dom.bracketedLabelCount,
      statusBarClippedSegmentCount: dom.statusBarClippedSegmentCount,
      sidebarTruncatedLabelCount: dom.sidebarTruncatedLabelCount,
      axeViolations,
      terminalStartupSpamCount: terminal.terminalStartupSpamCount,
      xtermHelperLeakCount: terminal.xtermHelperLeakCount,
      terminalFontProportionalLeakCount: terminal.terminalFontProportionalLeakCount,
      terminalContrastViolationCount: terminal.terminalContrastViolationCount,
    },
  };
}

function findEntryFailures(
  m: GuiDomAuditMetrics,
  verdict: UniversalCvFlywheelVerdict,
  vlmOverallScore: number,
): string[] {
  const errs: string[] = [];
  if (m.outerOverflowPx !== 0) errs.push(`outerOverflowPx=${m.outerOverflowPx}`);
  if (m.clippedInteractiveCount !== 0) errs.push(`clippedInteractiveCount=${m.clippedInteractiveCount}`);
  if (m.overlappingControlPairs !== 0) errs.push(`overlappingControlPairs=${m.overlappingControlPairs}`);
  if (m.unstyledNativeSelectCount !== 0) errs.push(`unstyledNativeSelectCount=${m.unstyledNativeSelectCount}`);
  if (m.axeViolations !== 0) errs.push(`axeViolations=${m.axeViolations}`);
  if ((m.terminalStartupSpamCount ?? 0) !== 0) errs.push(`terminalStartupSpamCount=${m.terminalStartupSpamCount}`);
  if ((m.xtermHelperLeakCount ?? 0) !== 0) errs.push(`xtermHelperLeakCount=${m.xtermHelperLeakCount}`);
  if ((m.terminalFontProportionalLeakCount ?? 0) !== 0) {
    errs.push(`terminalFontProportionalLeakCount=${m.terminalFontProportionalLeakCount}`);
  }
  if ((m.terminalContrastViolationCount ?? 0) !== 0) {
    errs.push(`terminalContrastViolationCount=${m.terminalContrastViolationCount}`);
  }
  if (verdict.visualAfterScore < 0.92) errs.push(`visualAfterScore=${verdict.visualAfterScore}<0.92`);
  if (vlmOverallScore < 0.9) errs.push(`vlmOverallScore=${vlmOverallScore}<0.90`);
  return errs;
}

function mergeStageLedger(): void {
  const order = new Map(TERMINAL_CV_STATES.map((s, idx) => [s.name, idx]));
  const partFiles = readdirSync(PARTS_DIR).filter((f) => f.startsWith(`${STAGE}-`) && f.endsWith('.json'));
  const entries: LedgerEntry[] = [];
  for (const file of partFiles) {
    const parsed = JSON.parse(readFileSync(join(PARTS_DIR, file), 'utf8')) as LedgerEntry[];
    entries.push(...parsed);
  }
  entries.sort((a, b) => (order.get(a.state) ?? 99) - (order.get(b.state) ?? 99));
  const failedCells = entries.filter((e) => e.failures.length > 0).length;
  const ledger = {
    ssotVersion: PLATFORM_SSOT_VERSION,
    ssotSha256: PLATFORM_SSOT_SHA256,
    stage: STAGE,
    capturedAt: new Date().toISOString(),
    summary: {
      totalCells: entries.length,
      passedCells: entries.length - failedCells,
      failedCells,
      minVisualAfterScore: entries.reduce((acc, e) => Math.min(acc, e.visualAfterScore), 1),
      minVlmOverallScore: entries.reduce((acc, e) => Math.min(acc, e.vlmOverallScore), 1),
    },
    entries,
  };
  const tmpFile = `${LEDGER_FILE}.${process.pid}.tmp`;
  writeFileSync(tmpFile, JSON.stringify(ledger, null, 2));
  renameSync(tmpFile, LEDGER_FILE);
}

test.describe('Dual-Surface Roman Scriptorium Terminal CV Flywheel', () => {
  test.skip(!RUN_FLYWHEEL, 'set VOX_CV_FLYWHEEL=1 to run the 5x4 terminal CV flywheel');

  for (const spec of TERMINAL_CV_STATES) {
    test(`${spec.surface} :: ${spec.name} across viewports and 4 themes`, async ({ browser }) => {
      test.setTimeout(300_000);
      mkdirSync(SHOTS_DIR, { recursive: true });
      mkdirSync(PARTS_DIR, { recursive: true });
      const baseURL = test.info().project.use.baseURL ?? 'http://localhost:1420';

      const beforeMap = new Map<string, LedgerEntry>();
      if (STAGE === 'after' && existsSync(BEFORE_LEDGER_FILE)) {
        const parsed = JSON.parse(readFileSync(BEFORE_LEDGER_FILE, 'utf8')) as { entries?: LedgerEntry[] };
        for (const e of parsed.entries ?? []) beforeMap.set(e.key, e);
      }

      const entries: LedgerEntry[] = [];
      const failedSummaries: string[] = [];

      for (const vp of viewportsForState(spec)) {
        const context = await browser.newContext({ baseURL, viewport: { width: vp.width, height: vp.height } });
        const page = await context.newPage();
        try {
          await addMockInitScript(page, installTauriMock, spec.surface);
          await addRichMockInitScript(page, spec.surface);
          await page.addInitScript(
            ({ flags, initialTheme }) => {
              try {
                window.localStorage.setItem('vox-theme', initialTheme);
                window.localStorage.setItem('gui.theme', initialTheme);
              } catch { /* ignore restricted storage */ }
              if (document.documentElement) document.documentElement.setAttribute('data-theme', initialTheme);
              for (const [k, v] of Object.entries(flags)) (window as unknown as Record<string, unknown>)[k] = v;
            },
            { flags: spec.initFlags ?? {}, initialTheme: TERMINAL_THEMES[0] },
          );

          await page.goto(`/#view=${spec.surface}`);
          const anchor = spec.surface === 'console' ? '[data-testid="term-session-bar"]' : '[data-testid="chat-surface-layout"]';
          await page.waitForSelector(anchor, { timeout: 15_000 });
          await page.evaluate(() => (document as unknown as { fonts?: { ready: Promise<unknown> } }).fonts?.ready);
          if (spec.setup) await spec.setup(page);
          if (spec.name === 'chat-terminal-dock') {
            const menu = page.locator('[aria-label="Workspace panels"]');
            if (await menu.isVisible().catch(() => false)) await page.keyboard.press('Escape');
          }

          for (const theme of TERMINAL_THEMES) {
            await page.evaluate((t) => {
              try {
                window.localStorage.setItem('vox-theme', t);
                window.localStorage.setItem('gui.theme', t);
              } catch { /* ignore restricted storage */ }
              document.documentElement.setAttribute('data-theme', t);
            }, theme);
            await page.waitForTimeout(100);
            await page.evaluate(() => (document as unknown as { fonts?: { ready: Promise<unknown> } }).fonts?.ready);

            const key = `${spec.name}:${theme}:${vp.name}`;
            const surfaceLabel = `${spec.surface}-${spec.name}-${theme}-${vp.name}`;
            const shotPath = join(SHOTS_DIR, `${spec.name}--${theme}--${vp.name}--${STAGE}.png`);
            const pngBuf = await page.screenshot({ path: shotPath, animations: 'disabled' });
            const pixelSignals = await decodePngRgbaInPage(page, pngBuf);
            const { metrics, terminal, axeDetails } = await collectCombinedMetrics(page, surfaceLabel);
            const vlmRubric = evaluateGuiSurfaceRubric(metrics, pixelSignals);
            const weightedRubricScore = computeWeightedScore(vlmRubric);
            const vlmCritique = await evaluateVlmSurfaceCritique(page, surfaceLabel, STAGE, pngBuf, metrics, OUT_DIR);
            const prev = beforeMap.get(key);
            const verdict = runUniversalCvFlywheelPass({
              surfaceId: surfaceLabel,
              passNumber: STAGE === 'before' ? 0 : 1,
              rawPrompt: `Dual-Surface Roman Scriptorium audit for ${surfaceLabel}`,
              beforeMetrics: prev
                ? { ...prev.pixelSignals, rubricScore: prev.weightedRubricScore, vlmRubric: prev.vlmRubric }
                : { ...pixelSignals, rubricScore: weightedRubricScore, vlmRubric },
              afterMetrics: { ...pixelSignals, rubricScore: weightedRubricScore, vlmRubric },
              afterDom: metrics,
              afterGrounded: true,
            });
            const failures = findEntryFailures(metrics, verdict, vlmCritique.vlmOverallScore);
            if (failures.length > 0) failedSummaries.push(`${key}: ${failures.join(', ')}`);

            entries.push({
              key, stage: STAGE, surface: spec.surface, state: spec.name, theme, viewport: vp.name,
              screenshotPath: relative(UI_ROOT, shotPath).replace(/\\/g, '/'),
              pixelSignals, metrics, axeDetails, terminalSamples: terminal.samples,
              vlmRubric, weightedRubricScore,
              visualBeforeScore: verdict.visualBeforeScore, visualAfterScore: verdict.visualAfterScore,
              flywheelConverged: verdict.flywheelConverged, vlmOverallScore: vlmCritique.vlmOverallScore,
              vlmCritique, verdict, failures,
            });
          }
        } finally {
          await page.close();
          await context.close();
        }
      }

      writeFileSync(join(PARTS_DIR, `${STAGE}-${spec.name}.json`), JSON.stringify(entries, null, 2));
      mergeStageLedger();
      expect(failedSummaries, `${spec.name} failures in ${LEDGER_FILE}`).toEqual([]);
    });
  }
});
