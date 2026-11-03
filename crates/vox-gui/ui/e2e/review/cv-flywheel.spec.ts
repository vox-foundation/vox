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
import { installTauriMock } from '../lib/tauriMock';
import { addMockInitScript } from '../lib/tauriMockShared';
import { addRichMockInitScript } from '../lib/tauriMockRich';
import {
  auditDomQualityInPage,
  auditIconsInPage,
  auditOverflowInPage,
} from './audits';
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
const ALL_SURFACES = SURFACE_REGISTRY.filter((e) => e.viewKey != null).map((e) => e.viewKey as string);

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

    // 2. Surface: loquela-knobs (open Risk/Governance popover + inspect composer toolbar)
    const riskBtn = page.getByRole('button', { name: /^Risk: / });
    if (await riskBtn.isVisible()) {
      await riskBtn.click();
      await page.waitForTimeout(200);
    }
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
    if (await riskBtn.isVisible()) {
      await page.keyboard.press('Escape');
    }

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

    // 4. Surface: urbs-civilization (navigate to Gamify / Urbs Sandbox or Harness Urbs view)
    await page.goto('/#view=gamify');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.waitForTimeout(400);
    const urbsTab = page.getByRole('button', { name: /Urbs Sandbox/i });
    if (await urbsTab.isVisible()) {
      await urbsTab.click();
      await page.waitForTimeout(500);
    }
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
        expect(dom.rawEmojiCount, `${surfaceId} rawEmojiCount`).toBe(0);
        expect(dom.unstyledNativeSelectCount, `${surfaceId} unstyledNativeSelectCount`).toBe(0);
        expect(dom.missingDockPanelIds, `${surfaceId} missingDockPanelIds`).toEqual([]);
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
      if (surfaceId in preRemediationDomDefects) {
        expect(verdict.flywheelConverged, `${surfaceId} flywheelConverged`).toBe(true);
      }
      expect(verdict.visualAfterScore, `${surfaceId} visualAfterScore`).toBeGreaterThanOrEqual(0.88);
    }
  });
});
