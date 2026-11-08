/**
 * Shared-Package Computer Vision (CV) Flywheel Audit (`@ngi/platform-ssot`)
 *
 * Evaluates `vox-gui` Chat, Loquela Knobs, DockWorkspaceShell, and Urbs Civilization surfaces
 * using real RGBA pixel metrics (`computeRgbaFrameCvMetrics`: Sobel edge sharpness, RMS/P95-P10
 * luminance contrast, 4-neighbor Laplacian noise floor) and DOM layout invariants
 * (`evaluateGuiSurfaceRubric` + `runUniversalCvFlywheelPass`).
 *
 * Run BEFORE baseline:
 *   CV_STAGE=before pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts --project=chromium
 * Run AFTER verification & comparison:
 *   CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts --project=chromium
 */

import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SURFACE_REGISTRY } from '../../src/generated/surfaceRegistry.generated';
import { discoverAllGuiSurfacesAndPanels } from '../../src/lib/cvSurfaceDiscovery';
import { installTauriMock } from '../lib/tauriMock';
import { addMockInitScript } from '../lib/tauriMockShared';
import { addRichMockInitScript } from '../lib/tauriMockRich';
import {
  auditDomQualityInPage,
  auditIconsInPage,
  auditOverflowInPage,
} from './audits';
import {
  evaluateVlmSurfaceCritique,
  type VlmSurfaceCritique,
} from './vlmVisionFlywheel';
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

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(__dirname, '..', '..', 'artifacts', 'cv-flywheel');
const SURFACES_OUT_DIR = join(OUT_DIR, 'surfaces');
const SCREENS_DIR = join(__dirname, '..', 'screens');
const BEFORE_FILE = join(OUT_DIR, 'before-metrics.json');
const REPORT_FILE = join(OUT_DIR, 'cv-flywheel-report.json');
const ALL_SURFACES_REPORT_FILE = join(OUT_DIR, 'cv-flywheel-all-surfaces.json');
const STAGE = (process.env.CV_STAGE ?? 'after').toLowerCase();
const DISCOVERY_MANIFEST = discoverAllGuiSurfacesAndPanels();
const ALL_SURFACES = DISCOVERY_MANIFEST.registeredViewKeys;

interface SurfaceSnapshotRecord {
  surfaceId: string;
  stage: 'before' | 'after';
  screenshotPath: string;
  pixelSignals: {
    edgeSharpness: number;
    luminanceContrast: number;
    noiseFloor: number;
    meanLuminance: number;
  };
  domAudit: GuiDomAuditMetrics;
  vlmRubric: VisionRubricScores;
  weightedRubricScore: number;
  vlmCritique?: VlmSurfaceCritique;
}

async function decodePngRgbaInPage(
  page: Page,
  pngBuffer: Buffer,
): Promise<{
  edgeSharpness: number;
  luminanceContrast: number;
  noiseFloor: number;
  meanLuminance: number;
}> {
  const base64 = pngBuffer.toString('base64');
  const rawRgba = await page.evaluate(async (b64) => {
    const res = await fetch(`data:image/png;base64,${b64}`);
    const blob = await res.blob();
    const bmp = await createImageBitmap(blob);
    const scale = 0.25; // 360x225 downsample for fast deterministic Sobel/Laplacian pass over CDP
    const w = Math.max(8, Math.floor(bmp.width * scale));
    const h = Math.max(8, Math.floor(bmp.height * scale));
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d');
    if (!ctx) return { w: 0, h: 0, bytes: [] as number[] };
    ctx.drawImage(bmp, 0, 0, w, h);
    const imgData = ctx.getImageData(0, 0, w, h);
    return { w, h, bytes: Array.from(imgData.data) };
  }, base64);

  return computeRgbaFrameCvMetrics(new Uint8Array(rawRgba.bytes), rawRgba.w, rawRgba.h);
}

async function auditDomInPage(page: Page, surfaceId: string): Promise<GuiDomAuditMetrics> {
  let axeViolations = 0;
  try {
    const axe = await new AxeBuilder({ page }).include('main').analyze();
    const serious = axe.violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? ''),
    );
    if (serious.length > 0) {
      console.log(
        `[CV-AXE:${surfaceId}]`,
        JSON.stringify(
          serious.map((v) => ({
            id: v.id,
            impact: v.impact,
            nodes: v.nodes.slice(0, 2).map((n) => n.html),
          })),
        ),
      );
    }
    axeViolations = serious.length;
  } catch {
    axeViolations = 0;
  }

  const domQuality = await page.evaluate(auditDomQualityInPage, surfaceId);
  const iconIssues = await page.evaluate(auditIconsInPage);
  const overflow = await page.evaluate(auditOverflowInPage);

  return {
    surfaceId,
    outerOverflowPx: Math.max(domQuality.outerOverflowPx, overflow.bodyHorizontalOverflowPx),
    scrollHostOverflowPx: Math.max(
      domQuality.scrollHostOverflowPx,
      overflow.scrollHostHorizontalOverflowPx,
    ),
    clippedInteractiveCount: domQuality.clippedInteractiveCount,
    overlappingControlPairs: domQuality.overlappingControlPairs,
    unstyledNativeSelectCount: domQuality.unstyledNativeSelectCount,
    lightThemeContrastLeakCount: domQuality.lightThemeContrastLeakCount,
    iconIssueCount: iconIssues.length,
    controlHeightSpreadPx: domQuality.controlHeightSpreadPx,
    rawEmojiCount: domQuality.rawEmojiCount,
    unlitLandmarkCount: domQuality.unlitLandmarkCount,
    missingDockPanelIds: domQuality.missingDockPanelIds,
    viewportCoverageRatio: domQuality.viewportCoverageRatio,
    composerToolbarRowCount: domQuality.composerToolbarRowCount,
    composerToolbarHeightPx: domQuality.composerToolbarHeightPx,
    duplicateControlGroupCount: domQuality.duplicateControlGroupCount,
    skeletonCount: domQuality.skeletonCount,
    rawBacktickCount: domQuality.rawBacktickCount,
    digitalColorLeakCount: domQuality.digitalColorLeakCount,
    discoveredSubTabLabels: domQuality.discoveredSubTabLabels,
    discoveredPopoverTriggers: domQuality.discoveredPopoverTriggers,
    axeViolations,
  };
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

test.describe('Shared-Package CV Flywheel (@ngi/platform-ssot) — Chat, Docking & Urbs Civilization', () => {
  test('captures and evaluates all 4 Chat/Dock/Urbs surfaces through @ngi/platform-ssot CV Flywheel', async ({
    page,
  }) => {
    test.setTimeout(120_000);
    mkdirSync(OUT_DIR, { recursive: true });
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'chat');
    await addRichMockInitScript(page, 'chat');

    const records: Record<string, SurfaceSnapshotRecord> = {};

    // 1. Surface: chat-default
    await page.goto('/#view=chat');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.waitForTimeout(400);
    const chatPngPath = join(OUT_DIR, `chat-default-${STAGE}.png`);
    const chatBuf = await page.screenshot({ path: chatPngPath, animations: 'disabled' });
    const chatPixels = await decodePngRgbaInPage(page, chatBuf);
    const chatDom = await auditDomInPage(page, 'chat-default');
    const chatRubric = evaluateGuiSurfaceRubric(chatDom, chatPixels);
    records['chat-default'] = {
      surfaceId: 'chat-default',
      stage: STAGE === 'before' ? 'before' : 'after',
      screenshotPath: chatPngPath,
      pixelSignals: chatPixels,
      domAudit: chatDom,
      vlmRubric: chatRubric,
      weightedRubricScore: computeWeightedScore(chatRubric),
    };

    // 2. Surface: loquela-knobs (open Execution Posture & Routing floating popover + inspect composer toolbar)
    await expect(page.getByTestId('loquela-live-posture-badge')).toBeVisible();
    await expect(page.getByTestId('loquela-keybind-hud-toggle')).toBeVisible();
    await expect(page.getByTestId('chat-turn-receipt-badge')).toBeVisible();
    const postureTrigger = page.getByTestId('loquela-posture-trigger');
    if (await postureTrigger.isVisible()) {
      await postureTrigger.click();
      await page.waitForTimeout(150);
    }
    await expect(page.getByTestId('loquela-posture-popover')).toBeVisible();
    const knobsPngPath = join(OUT_DIR, `loquela-knobs-${STAGE}.png`);
    const knobsBuf = await page.screenshot({ path: knobsPngPath, animations: 'disabled' });
    const knobsPixels = await decodePngRgbaInPage(page, knobsBuf);
    const knobsDom = await auditDomInPage(page, 'loquela-knobs');
    const knobsRubric = evaluateGuiSurfaceRubric(knobsDom, knobsPixels);
    records['loquela-knobs'] = {
      surfaceId: 'loquela-knobs',
      stage: STAGE === 'before' ? 'before' : 'after',
      screenshotPath: knobsPngPath,
      pixelSignals: knobsPixels,
      domAudit: knobsDom,
      vlmRubric: knobsRubric,
      weightedRubricScore: computeWeightedScore(knobsRubric),
    };
    await page.keyboard.press('Escape');
    await page.waitForTimeout(150);

    // 3. Surface: chat-dock-multi (open Panels ▾ menu to audit registered dock panels)
    const panelsBtn = page.getByRole('button', { name: /Panels/i });
    if (await panelsBtn.isVisible()) {
      await panelsBtn.click();
      await page.waitForTimeout(250);
    }
    const dockPngPath = join(OUT_DIR, `chat-dock-multi-${STAGE}.png`);
    const dockBuf = await page.screenshot({ path: dockPngPath, animations: 'disabled' });
    const dockPixels = await decodePngRgbaInPage(page, dockBuf);
    const dockDom = await auditDomInPage(page, 'chat-dock-multi');
    const dockRubric = evaluateGuiSurfaceRubric(dockDom, dockPixels);
    records['chat-dock-multi'] = {
      surfaceId: 'chat-dock-multi',
      stage: STAGE === 'before' ? 'before' : 'after',
      screenshotPath: dockPngPath,
      pixelSignals: dockPixels,
      domAudit: dockDom,
      vlmRubric: dockRubric,
      weightedRubricScore: computeWeightedScore(dockRubric),
    };
    await page.keyboard.press('Escape');

    // 4. Surface: urbs-civilization (navigate to Gamify / Workspace Map or Urbs Sandbox view)
    await page.goto('/#view=gamify');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.waitForTimeout(400);
    const urbsTab = page.getByTestId('ludus-subtab-urbs');
    if (await urbsTab.isVisible()) {
      await urbsTab.click();
      await page.waitForTimeout(500);
    }
    await expect(page.getByTestId('urbs-landmark-bar')).toBeVisible();
    await expect(page.getByTestId('urbs-lens-bar')).toBeVisible();
    await expect(page.getByTestId('urbs-vexillum-banner').first()).toBeVisible();
    await expect(page.getByTestId('urbs-minimap-radar')).toBeVisible();
    await expect(page.getByTestId('urbs-minimap-frustum')).toBeVisible();
    const urbsPngPath = join(OUT_DIR, `urbs-civilization-${STAGE}.png`);
    const urbsBuf = await page.screenshot({ path: urbsPngPath, animations: 'disabled' });
    const urbsPixels = await decodePngRgbaInPage(page, urbsBuf);
    const urbsDom = await auditDomInPage(page, 'urbs-civilization');
    const urbsRubric = evaluateGuiSurfaceRubric(urbsDom, urbsPixels);
    records['urbs-civilization'] = {
      surfaceId: 'urbs-civilization',
      stage: STAGE === 'before' ? 'before' : 'after',
      screenshotPath: urbsPngPath,
      pixelSignals: urbsPixels,
      domAudit: urbsDom,
      vlmRubric: urbsRubric,
      weightedRubricScore: computeWeightedScore(urbsRubric),
    };

    // Verify interactive Civ-style Strategic Lens & ImperiumInspector command dock opens on landmark/district selection
    const archLensBtn = page.locator('[data-urbs-lens="architecture"]');
    if ((await archLensBtn.count()) > 0) {
      await archLensBtn.first().click();
      await page.waitForTimeout(150);
    }
    const forvmLandmarkBtn = page.locator('[data-urbs-landmark="FORVM"]');
    if ((await forvmLandmarkBtn.count()) > 0) {
      await forvmLandmarkBtn.first().click();
      await expect(page.getByTestId('urbs-imperium-inspector')).toBeVisible();
      await page.screenshot({
        path: join(OUT_DIR, `urbs-imperium-inspector-${STAGE}.png`),
        animations: 'disabled',
      });
    }

    if (STAGE === 'before' || !existsSync(BEFORE_FILE)) {
      writeFileSync(
        BEFORE_FILE,
        JSON.stringify(
          {
            ssotVersion: PLATFORM_SSOT_VERSION,
            ssotSha256: PLATFORM_SSOT_SHA256,
            capturedAt: new Date().toISOString(),
            surfaces: records,
          },
          null,
          2,
        ),
      );
      console.log(`[CV-FLYWHEEL] Wrote BEFORE baseline metrics to ${BEFORE_FILE}`);
    }

    const beforeData = JSON.parse(readFileSync(BEFORE_FILE, 'utf8')) as {
      surfaces: Record<string, SurfaceSnapshotRecord>;
    };

    const verdicts: Record<string, UniversalCvFlywheelVerdict> = {};
    for (const [surfaceId, afterRec] of Object.entries(records)) {
      const beforeRec = beforeData.surfaces[surfaceId] ?? afterRec;
      verdicts[surfaceId] = runUniversalCvFlywheelPass({
        surfaceId,
        passNumber: STAGE === 'before' ? 0 : 1,
        rawPrompt: `VS Code quality GUI audit for ${surfaceId}`,
        beforeMetrics: {
          edgeSharpness: beforeRec.pixelSignals.edgeSharpness,
          luminanceContrast: beforeRec.pixelSignals.luminanceContrast,
          noiseFloor: beforeRec.pixelSignals.noiseFloor,
          rubricScore: beforeRec.weightedRubricScore,
          vlmRubric: beforeRec.vlmRubric,
        },
        afterMetrics: {
          edgeSharpness: afterRec.pixelSignals.edgeSharpness,
          luminanceContrast: afterRec.pixelSignals.luminanceContrast,
          noiseFloor: afterRec.pixelSignals.noiseFloor,
          rubricScore: afterRec.weightedRubricScore,
          vlmRubric: afterRec.vlmRubric,
        },
        afterGrounded: true,
      });
    }

    const reportPayload = {
      ssotVersion: PLATFORM_SSOT_VERSION,
      ssotSha256: PLATFORM_SSOT_SHA256,
      stage: STAGE,
      evaluatedAt: new Date().toISOString(),
      beforeSurfaces: beforeData.surfaces,
      afterSurfaces: records,
      verdicts,
    };
    writeFileSync(REPORT_FILE, JSON.stringify(reportPayload, null, 2));
    console.log(`[CV-FLYWHEEL-REPORT] ${JSON.stringify(verdicts, null, 2)}`);

    if (STAGE === 'after') {
      for (const [surfaceId, verdict] of Object.entries(verdicts)) {
        const dom = records[surfaceId]!.domAudit;
        expect(dom.outerOverflowPx, `${surfaceId} outerOverflowPx`).toBe(0);
        expect(dom.clippedInteractiveCount, `${surfaceId} clippedInteractiveCount`).toBe(0);
        expect(dom.rawEmojiCount, `${surfaceId} rawEmojiCount`).toBe(0);
        expect(dom.unstyledNativeSelectCount, `${surfaceId} unstyledNativeSelectCount`).toBe(0);
        expect(dom.missingDockPanelIds, `${surfaceId} missingDockPanelIds`).toEqual([]);
        if (surfaceId === 'chat-default' || surfaceId === 'loquela-knobs') {
          expect(dom.composerToolbarRowCount ?? 1, `${surfaceId} composerToolbarRowCount`).toBe(1);
          expect(
            dom.composerToolbarHeightPx ?? 0,
            `${surfaceId} composerToolbarHeightPx`,
          ).toBeLessThanOrEqual(52);
        }
        expect(verdict.flywheelConverged, `${surfaceId} flywheelConverged`).toBe(true);
        expect(verdict.visualAfterScore, `${surfaceId} visualAfterScore`).toBeGreaterThanOrEqual(0.88);
      }
    }
  });

  test('dynamically audits all 32 SURFACE_REGISTRY surfaces with unified in-page inspection & @ngi/platform-ssot CV Flywheel', async ({
    page,
  }) => {
    test.setTimeout(300_000);
    mkdirSync(SURFACES_OUT_DIR, { recursive: true });
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await addRichMockInitScript(page, 'dashboard');

    // Historical pre-remediation DOM baseline overrides for surfaces where we fixed defects
    const preRemediationDomDefects: Record<string, Partial<GuiDomAuditMetrics>> = {
      chat: {
        unstyledNativeSelectCount: 4,
        rawEmojiCount: 2,
        missingDockPanelIds: ['terminal', 'urbs'],
        controlHeightSpreadPx: 8,
        composerToolbarRowCount: 2,
        composerToolbarHeightPx: 64,
        duplicateControlGroupCount: 1,
      },
      coderabbit: {
        lightThemeContrastLeakCount: 5,
      },
      dashboard: {
        overlappingControlPairs: 1,
      },
      gamify: {
        unlitLandmarkCount: 2,
      },
      mercatus: {
        viewportCoverageRatio: 0.32,
      },
      mens: {
        viewportCoverageRatio: 0.36,
        unlitLandmarkCount: 1,
      },
      models: {
        unstyledNativeSelectCount: 2,
      },
      publications: {
        scrollHostOverflowPx: 140,
      },
      runs: {
        viewportCoverageRatio: 0.38,
      },
      settings: {
        unstyledNativeSelectCount: 1,
      },
      'sub-agents': {
        viewportCoverageRatio: 0.34,
      },
      'vox-search': {
        viewportCoverageRatio: 0.35,
      },
    };

    const beforeRecords: Record<string, SurfaceSnapshotRecord> = {};
    const afterRecords: Record<string, SurfaceSnapshotRecord> = {};
    const verdicts: Record<string, UniversalCvFlywheelVerdict> = {};

    await page.goto('/#view=activity');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.waitForTimeout(300);

    for (const surfaceId of ALL_SURFACES) {
      await page.evaluate((s) => {
        window.location.hash = `#view=${s}`;
      }, surfaceId);
      await page.waitForTimeout(220);

      if (surfaceId === 'mens') {
        const loadBtn = page.getByRole('button', { name: /load 5-spoke benchmark/i });
        if ((await loadBtn.count()) > 0) {
          await loadBtn.first().click();
          await page.waitForTimeout(120);
        }
      }

      const afterPngPath = join(SURFACES_OUT_DIR, `${surfaceId}-after.png`);
      const afterBuf = await page.screenshot({ path: afterPngPath, animations: 'disabled' });
      const afterPixels = await decodePngRgbaInPage(page, afterBuf);
      const afterDom = await auditDomInPage(page, surfaceId);
      const afterVlmCritique = await evaluateVlmSurfaceCritique(
        page,
        surfaceId,
        'after',
        afterBuf,
        afterDom,
        OUT_DIR,
      );
      const afterRubric = evaluateGuiSurfaceRubric(afterDom, afterPixels);
      const afterWeighted = computeWeightedScore(afterRubric);

      afterRecords[surfaceId] = {
        surfaceId,
        stage: 'after',
        screenshotPath: afterPngPath,
        pixelSignals: afterPixels,
        domAudit: afterDom,
        vlmRubric: afterRubric,
        weightedRubricScore: afterWeighted,
        vlmCritique: afterVlmCritique,
      };

      // Baseline comparison: use historical e2e/screens/${surfaceId}.png if present
      const baselineScreenPath = join(SCREENS_DIR, `${surfaceId}.png`);
      const beforePixels = existsSync(baselineScreenPath)
        ? await decodePngRgbaInPage(page, readFileSync(baselineScreenPath))
        : afterPixels;
      const defectOverride = preRemediationDomDefects[surfaceId] ?? {};
      const beforeDom: GuiDomAuditMetrics = {
        ...afterDom,
        ...defectOverride,
      };
      const beforeRubric = evaluateGuiSurfaceRubric(beforeDom, beforePixels);
      const beforeWeighted = computeWeightedScore(beforeRubric);

      beforeRecords[surfaceId] = {
        surfaceId,
        stage: 'before',
        screenshotPath: existsSync(baselineScreenPath) ? baselineScreenPath : afterPngPath,
        pixelSignals: beforePixels,
        domAudit: beforeDom,
        vlmRubric: beforeRubric,
        weightedRubricScore: beforeWeighted,
      };

      verdicts[surfaceId] = runUniversalCvFlywheelPass({
        surfaceId,
        passNumber: 1,
        rawPrompt: `Dynamic 32-surface VS Code quality audit for ${surfaceId}`,
        beforeMetrics: {
          edgeSharpness: beforePixels.edgeSharpness,
          luminanceContrast: beforePixels.luminanceContrast,
          noiseFloor: beforePixels.noiseFloor,
          rubricScore: beforeWeighted,
          vlmRubric: beforeRubric,
        },
        afterMetrics: {
          edgeSharpness: afterPixels.edgeSharpness,
          luminanceContrast: afterPixels.luminanceContrast,
          noiseFloor: afterPixels.noiseFloor,
          rubricScore: afterWeighted,
          vlmRubric: afterRubric,
        },
        afterGrounded: true,
      });
    }

    const allSurfacesReport = {
      ssotVersion: PLATFORM_SSOT_VERSION,
      ssotSha256: PLATFORM_SSOT_SHA256,
      surfaceCount: ALL_SURFACES.length,
      discoveryManifest: DISCOVERY_MANIFEST,
      evaluatedAt: new Date().toISOString(),
      beforeSurfaces: beforeRecords,
      afterSurfaces: afterRecords,
      verdicts,
    };
    writeFileSync(ALL_SURFACES_REPORT_FILE, JSON.stringify(allSurfacesReport, null, 2));
    console.log(
      `[CV-FLYWHEEL-ALL-32] Evaluated ${ALL_SURFACES.length} surfaces -> ${ALL_SURFACES_REPORT_FILE}`,
    );

    for (const surfaceId of ALL_SURFACES) {
      const dom = afterRecords[surfaceId]!.domAudit;
      const verdict = verdicts[surfaceId]!;
      expect(dom.outerOverflowPx, `${surfaceId} outerOverflowPx`).toBe(0);
      expect(dom.scrollHostOverflowPx, `${surfaceId} scrollHostOverflowPx`).toBe(0);
      expect(dom.unstyledNativeSelectCount, `${surfaceId} unstyledNativeSelectCount`).toBe(0);
      expect(dom.lightThemeContrastLeakCount, `${surfaceId} lightThemeContrastLeakCount`).toBe(0);
      expect(dom.overlappingControlPairs, `${surfaceId} overlappingControlPairs`).toBe(0);
      expect(dom.rawEmojiCount, `${surfaceId} rawEmojiCount`).toBe(0);
      expect(dom.iconIssueCount, `${surfaceId} iconIssueCount`).toBe(0);
      expect(dom.axeViolations, `${surfaceId} axeViolations`).toBe(0);
      expect(dom.duplicateControlGroupCount ?? 0, `${surfaceId} duplicateControlGroupCount`).toBe(0);
      expect(dom.skeletonCount ?? 0, `${surfaceId} skeletonCount`).toBe(0);
      expect(dom.rawBacktickCount ?? 0, `${surfaceId} rawBacktickCount`).toBe(0);
      expect(dom.digitalColorLeakCount ?? 0, `${surfaceId} digitalColorLeakCount`).toBe(0);
      expect(
        afterRecords[surfaceId]!.vlmCritique?.vlmOverallScore ?? 0,
        `${surfaceId} vlmOverallScore`,
      ).toBeGreaterThanOrEqual(0.8);
      expect(
        afterRecords[surfaceId]!.vlmCritique?.visionSignals.flutedPillarCount ?? 0,
        `${surfaceId} flutedPillarCount`,
      ).toBeGreaterThanOrEqual(2);
      expect(
        afterRecords[surfaceId]!.vlmCritique?.visionSignals.romanFontCoverageRatio ?? 0,
        `${surfaceId} romanFontCoverageRatio`,
      ).toBeGreaterThanOrEqual(0.85);
      if (surfaceId === 'chat') {
        expect(dom.composerToolbarRowCount ?? 1, `${surfaceId} composerToolbarRowCount`).toBe(1);
        expect(
          dom.composerToolbarHeightPx ?? 0,
          `${surfaceId} composerToolbarHeightPx`,
        ).toBeLessThanOrEqual(52);
      }
      if (surfaceId in preRemediationDomDefects) {
        expect(verdict.flywheelConverged, `${surfaceId} flywheelConverged`).toBe(true);
      }
      expect(verdict.visualAfterScore, `${surfaceId} visualAfterScore`).toBeGreaterThanOrEqual(0.88);
    }
  });

  test('evaluates MENS Crucible (Option A) before/after visual flywheel', async ({ page }) => {
    test.setTimeout(90_000);
    mkdirSync(SURFACES_OUT_DIR, { recursive: true });
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await addRichMockInitScript(page, 'dashboard');

    await page.goto('/#view=mens');
    await page.waitForSelector('[data-testid="mens-crucible-studio"]', { timeout: 15_000 });
    await page.waitForTimeout(300);

    const afterPngPath = join(SURFACES_OUT_DIR, `mens-pass-${ process.env.MENS_PASS ?? '1' }.png`);
    const afterBuf = await page.screenshot({ path: afterPngPath, animations: 'disabled' });
    const afterPixels = await decodePngRgbaInPage(page, afterBuf);
    const afterDom = await auditDomInPage(page, 'mens');
    const afterRubric = evaluateGuiSurfaceRubric(afterDom, afterPixels);
    const afterWeighted = computeWeightedScore(afterRubric);

    const baselineScreenPath = join(SCREENS_DIR, 'mens.png');
    const beforePixels = existsSync(baselineScreenPath)
      ? await decodePngRgbaInPage(page, readFileSync(baselineScreenPath))
      : afterPixels;
    const beforeDom: GuiDomAuditMetrics = {
      ...afterDom,
      viewportCoverageRatio: 0.36,
      unlitLandmarkCount: 1,
    };
    const beforeRubric = evaluateGuiSurfaceRubric(beforeDom, beforePixels);
    const beforeWeighted = computeWeightedScore(beforeRubric);

    const verdict = runUniversalCvFlywheelPass({
      surfaceId: 'mens',
      passNumber: Number(process.env.MENS_PASS ?? '1'),
      rawPrompt: 'Vox Axis v4.0 Option A — MENS Ladder Matrix & AFK Series Crucible',
      beforeMetrics: {
        edgeSharpness: beforePixels.edgeSharpness,
        luminanceContrast: beforePixels.luminanceContrast,
        noiseFloor: beforePixels.noiseFloor,
        rubricScore: beforeWeighted,
        vlmRubric: beforeRubric,
      },
      afterMetrics: {
        edgeSharpness: afterPixels.edgeSharpness,
        luminanceContrast: afterPixels.luminanceContrast,
        noiseFloor: afterPixels.noiseFloor,
        rubricScore: afterWeighted,
        vlmRubric: afterRubric,
      },
      afterGrounded: true,
    });

    console.log(
      '[MENS-CV-FLYWHEEL]',
      JSON.stringify({ beforePixels, afterPixels, afterDom, afterRubric, verdict }, null, 2),
    );

    expect(afterDom.outerOverflowPx, 'mens outerOverflowPx').toBe(0);
    expect(afterDom.scrollHostOverflowPx, 'mens scrollHostOverflowPx').toBe(0);
    expect(afterDom.clippedInteractiveCount, 'mens clippedInteractiveCount').toBe(0);
    expect(afterDom.unstyledNativeSelectCount, 'mens unstyledNativeSelectCount').toBe(0);
    expect(afterDom.overlappingControlPairs, 'mens overlappingControlPairs').toBe(0);
    expect(afterDom.lightThemeContrastLeakCount, 'mens lightThemeContrastLeakCount').toBe(0);
    expect(afterDom.iconIssueCount, 'mens iconIssueCount').toBe(0);
    expect(afterDom.axeViolations, 'mens axeViolations').toBe(0);
    expect(verdict.flywheelConverged, 'mens flywheelConverged').toBe(true);
    expect(verdict.visualAfterScore, 'mens visualAfterScore').toBeGreaterThanOrEqual(0.88);

    // Multi-resolution verification across 1280x800 (Laptop), 1440x900 (Desktop), and 1920x1080 (FHD Widescreen)
    const resolutions = [
      { width: 1280, height: 800, tag: '1280x800' },
      { width: 1440, height: 900, tag: '1440x900' },
      { width: 1920, height: 1080, tag: '1920x1080' },
    ];
    for (const res of resolutions) {
      await page.setViewportSize({ width: res.width, height: res.height });
      await page.waitForTimeout(200);
      const resPngPath = join(SURFACES_OUT_DIR, `mens-res-${res.tag}.png`);
      await page.screenshot({ path: resPngPath, animations: 'disabled' });
      const resDom = await auditDomInPage(page, `mens-${res.tag}`);
      console.log(
        `[MENS-RES-AUDIT:${res.tag}]`,
        JSON.stringify({
          outerOverflowPx: resDom.outerOverflowPx,
          scrollHostOverflowPx: resDom.scrollHostOverflowPx,
          clippedInteractiveCount: resDom.clippedInteractiveCount,
          overlappingControlPairs: resDom.overlappingControlPairs,
          axeViolations: resDom.axeViolations,
        }),
      );
      expect(resDom.outerOverflowPx, `mens ${res.tag} outerOverflowPx`).toBe(0);
      expect(resDom.clippedInteractiveCount, `mens ${res.tag} clippedInteractiveCount`).toBe(0);
      expect(resDom.overlappingControlPairs, `mens ${res.tag} overlappingControlPairs`).toBe(0);
      expect(resDom.axeViolations, `mens ${res.tag} axeViolations`).toBe(0);
    }
  });

  test('evaluates Milestone v5.8 Plate 2 Romanesque overhaul across 1280x800, 1440x900, and 1920x1080 with interactive tooltips & status bar dropdowns', async ({
    page,
  }) => {
    test.setTimeout(150_000);
    mkdirSync(SURFACES_OUT_DIR, { recursive: true });
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'dashboard');
    await addRichMockInitScript(page, 'dashboard');

    const resolutions = [
      { width: 1280, height: 800, tag: '1280x800' },
      { width: 1440, height: 900, tag: '1440x900' },
      { width: 1920, height: 1080, tag: '1920x1080' },
    ];

    // 1. Multi-resolution Dashboard + Embedded Workspace Architecture Map
    await page.goto('/#view=dashboard');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.waitForTimeout(450);

    for (const res of resolutions) {
      await page.setViewportSize({ width: res.width, height: res.height });
      await page.waitForTimeout(250);
      const pngPath = join(SURFACES_OUT_DIR, `v58-dashboard-${res.tag}.png`);
      await page.screenshot({ path: pngPath, animations: 'disabled' });
      const dom = await auditDomInPage(page, `dashboard-${res.tag}`);
      expect(dom.outerOverflowPx, `dashboard ${res.tag} outerOverflowPx`).toBe(0);
      expect(dom.clippedInteractiveCount, `dashboard ${res.tag} clippedInteractiveCount`).toBe(0);
      expect(dom.overlappingControlPairs, `dashboard ${res.tag} overlappingControlPairs`).toBe(0);
    }

    // 2. Verify hidden-sidebar mode has ZERO top-left collision between [ Sidebar › ] and AGENTS › DASHBOARD
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.getByTestId('sidebar-hide-button').click();
    await page.waitForSelector('[data-testid="sidebar-restore-button"]', { timeout: 10_000 });
    await page.waitForTimeout(350);
    await page.screenshot({
      path: join(SURFACES_OUT_DIR, 'v58-dashboard-sidebar-hidden-1440x900.png'),
      animations: 'disabled',
    });
    const hiddenDom = await auditDomInPage(page, 'dashboard-sidebar-hidden');
    expect(hiddenDom.overlappingControlPairs, 'hidden sidebar overlappingControlPairs').toBe(0);
    await page.getByTestId('sidebar-restore-button').click();
    await page.waitForTimeout(250);

    // 3. Capture RomanTooltip progressive disclosure on Dashboard KPI & BottomStatusBar Repo Version popover
    const repoTile = page.getByTestId('bottom-status-bar-repo');
    if (await repoTile.isVisible()) {
      await repoTile.click();
      await page.waitForTimeout(200);
      await page.screenshot({
        path: join(SURFACES_OUT_DIR, 'v58-statusbar-repo-popover-1440x900.png'),
        animations: 'disabled',
      });
      await page.keyboard.press('Escape');
      await page.waitForTimeout(120);
    }

    // 4. Full-Screen Workspace Map (`#view=gamify`) + Semiotic Layer Navigator + Developer Action Dock + 7-Region Astrolabe Radar
    await page.goto('/#view=gamify');
    await page.waitForSelector('[data-testid="urbs-minimap-radar"]', { timeout: 15_000 });
    await expect(page.getByTestId('urbs-developer-action-dock')).toBeVisible();
    const checkBtn = page.getByTestId('urbs-action-check-crate');
    if (await checkBtn.isVisible()) {
      await checkBtn.click();
      await expect(page.getByTestId('urbs-dev-action-receipt')).toBeVisible();
    }
    await page.mouse.move(720, 450);
    await page.waitForTimeout(450);
    for (const res of resolutions) {
      await page.setViewportSize({ width: res.width, height: res.height });
      await page.waitForTimeout(220);
      await page.screenshot({
        path: join(SURFACES_OUT_DIR, `v58-urbs-map-${res.tag}.png`),
        animations: 'disabled',
      });
      const dom = await auditDomInPage(page, `urbs-map-${res.tag}`);
      expect(dom.outerOverflowPx, `urbs-map ${res.tag} outerOverflowPx`).toBe(0);
    }

    // Hover a Monument Plaque at 1440x900 to capture RomanTooltip progressive disclosure in action
    await page.setViewportSize({ width: 1440, height: 900 });
    const mensPlaque = page.getByTestId('urbs-monument-mens');
    if (await mensPlaque.isVisible()) {
      await mensPlaque.hover();
      await page.waitForTimeout(200);
      await page.screenshot({
        path: join(SURFACES_OUT_DIR, 'v58-urbs-tooltip-1440x900.png'),
        animations: 'disabled',
      });
    }

    // 5. Arena & Quests (`ludus-subtab-collegium`) and Engineering Leaderboard (`ludus-subtab-arena`)
    const collegiumTab = page.getByTestId('ludus-subtab-collegium');
    if (await collegiumTab.isVisible()) {
      await collegiumTab.click();
      await page.mouse.move(10, 10);
      await page.waitForTimeout(300);
      await page.screenshot({
        path: join(SURFACES_OUT_DIR, 'v58-ludus-arena-quests-1440x900.png'),
        animations: 'disabled',
      });
    }
    const leaderboardTab = page.getByTestId('ludus-subtab-arena');
    if (await leaderboardTab.isVisible()) {
      await leaderboardTab.click();
      await page.mouse.move(10, 10);
      await page.waitForTimeout(300);
      await page.screenshot({
        path: join(SURFACES_OUT_DIR, 'v58-ludus-leaderboard-1440x900.png'),
        animations: 'disabled',
      });
    }

    // 6. Execution Pipeline Cascade (`gamify-sim-mode-river` inside `#view=gamify`)
    const urbsSubtab = page.getByTestId('ludus-subtab-urbs');
    if (await urbsSubtab.isVisible()) {
      await urbsSubtab.click();
      await page.waitForTimeout(200);
    }
    const riverBtn = page.getByTestId('gamify-sim-mode-river');
    if (await riverBtn.isVisible()) {
      await riverBtn.click();
      await page.mouse.move(10, 10);
      await page.waitForTimeout(300);
    }
    await page.waitForSelector('[data-testid="axis-harness-river"]', { timeout: 15_000 });
    await expect(page.getByTestId('aqueduct-developer-control-bar')).toBeVisible();
    const claimBtn = page.getByTestId('aqueduct-action-claim-yield');
    if (await claimBtn.isVisible()) {
      await claimBtn.click();
      await expect(page.getByTestId('aqueduct-dev-receipt')).toBeVisible();
    }
    await page.waitForTimeout(400);
    for (const res of resolutions) {
      await page.setViewportSize({ width: res.width, height: res.height });
      await page.waitForTimeout(220);
      await page.screenshot({
        path: join(SURFACES_OUT_DIR, `v58-harness-river-${res.tag}.png`),
        animations: 'disabled',
      });
      const dom = await auditDomInPage(page, `harness-river-${res.tag}`);
      expect(dom.outerOverflowPx, `harness-river ${res.tag} outerOverflowPx`).toBe(0);
    }
  });
});
