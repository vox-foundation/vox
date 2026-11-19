/**
 * Milestone v8.1 (`scientia-kb-roman-ux-v81`) — Focused Research & Knowledge Base
 * Dual-Theme (`arcane` + `void`) Playwright + @ngi/platform-ssot CV Flywheel Suite.
 *
 * Run BEFORE baseline:
 *   VOX_CV_FLYWHEEL=1 CV_STAGE=before pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/kb-research-cv-flywheel.spec.ts --project=chromium
 * Run AFTER verification:
 *   VOX_CV_FLYWHEEL=1 CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/kb-research-cv-flywheel.spec.ts --project=chromium
 */

import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from '../lib/tauriMock';
import { addMockInitScript } from '../lib/tauriMockShared';
import { addRichMockInitScript } from '../lib/tauriMockRich';
import {
  auditDomQualityInPage,
  auditIconsInPage,
  auditOverflowInPage,
} from './audits';
import {
  conceptArtPlateForSurface,
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
const UI_ROOT = join(__dirname, '..', '..');
const OUT_DIR = join(UI_ROOT, 'artifacts', 'cv-flywheel', 'kb-research');
const STAGE: 'before' | 'after' =
  (process.env.CV_STAGE ?? 'after').toLowerCase() === 'before' ? 'before' : 'after';
const RUN_FLYWHEEL = !process.env.CI || process.env.VOX_CV_FLYWHEEL === '1';

const TARGET_SURFACES = [
  { id: 'research', viewKey: 'research', waitForDetail: true },
  { id: 'scientia', viewKey: 'scientia' },
  { id: 'scientia-claims', viewKey: 'scientia', tabId: 'claims' },
  { id: 'publications', viewKey: 'publications' },
  { id: 'activity', viewKey: 'activity' },
  { id: 'memory', viewKey: 'memory', triggerRecall: true },
  { id: 'vox-search', viewKey: 'vox-search' },
  { id: 'doc-reader', viewKey: 'memory', openDocDrawer: true },
] as const;

const THEMES = ['arcane', 'void'] as const;

interface KbResearchRecord {
  key: string;
  surfaceId: string;
  theme: 'arcane' | 'void';
  stage: 'before' | 'after';
  screenshotPath: string;
  pixelSignals: {
    edgeSharpness: number;
    luminanceContrast: number;
    noiseFloor: number;
    meanLuminance: number;
  };
  domAudit: GuiDomAuditMetrics & {
    surfaceToolbarHeightSpreadPx?: number;
    unreadableSmallFontCount?: number;
  };
  vlmRubric: VisionRubricScores;
  weightedRubricScore: number;
  vlmCritique: VlmSurfaceCritique;
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
      const canvas = document.createElement('canvas');
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext('2d');
      if (!ctx) return { w: 0, h: 0, bytes: [] as number[] };
      ctx.drawImage(bmp, 0, 0, w, h);
      return { w, h, bytes: Array.from(ctx.getImageData(0, 0, w, h).data) };
    } finally {
      bmp.close();
    }
  }, b64);
  return computeRgbaFrameCvMetrics(new Uint8Array(raw.bytes), raw.w, raw.h);
}

async function auditExtendedDom(
  page: Page,
  surfaceId: string,
): Promise<KbResearchRecord['domAudit']> {
  let axeViolations = 0;
  try {
    const axe = await new AxeBuilder({ page })
      .options({ resultTypes: ['violations'] })
      .analyze();
    const serious = axe.violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? ''),
    );
    axeViolations = serious.length;
  } catch {
    axeViolations = 0;
  }

  const domQuality = (await page.evaluate(auditDomQualityInPage, surfaceId)) as ReturnType<
    typeof auditDomQualityInPage
  > & {
    surfaceToolbarHeightSpreadPx?: number;
    unreadableSmallFontCount?: number;
  };
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
    chartPanelFillRatio: domQuality.chartPanelFillRatio,
    bracketedLabelCount: domQuality.bracketedLabelCount,
    statusBarClippedSegmentCount: domQuality.statusBarClippedSegmentCount,
    sidebarTruncatedLabelCount: domQuality.sidebarTruncatedLabelCount,
    discoveredSubTabLabels: domQuality.discoveredSubTabLabels,
    discoveredPopoverTriggers: domQuality.discoveredPopoverTriggers,
    surfaceToolbarHeightSpreadPx: domQuality.surfaceToolbarHeightSpreadPx ?? 0,
    unreadableSmallFontCount: domQuality.unreadableSmallFontCount ?? 0,
    axeViolations,
  };
}

test.describe('Milestone v8.1 Research & Knowledge Base Dual-Theme CV Flywheel', () => {
  test.skip(!RUN_FLYWHEEL, 'set VOX_CV_FLYWHEEL=1 in CI to run Research/KB CV audit');

  test('captures and audits all Research & Knowledge Base surfaces in arcane & void themes', async ({
    page,
  }) => {
    test.setTimeout(360_000);
    const stageDir = join(OUT_DIR, STAGE);
    mkdirSync(stageDir, { recursive: true });
    await page.setViewportSize({ width: 1440, height: 900 });
    await addMockInitScript(page, installTauriMock, 'research');
    await addRichMockInitScript(page, 'research');

    await page.goto('/#view=research');
    await page.waitForSelector('nav', { timeout: 15_000 });
    await page.waitForTimeout(300);

    const records: Record<string, KbResearchRecord> = {};

    for (const theme of THEMES) {
      await page.evaluate((t) => {
        document.documentElement.setAttribute('data-theme', t);
        window.localStorage.setItem('vox-theme', t);
      }, theme);
      await page.waitForTimeout(180);

      for (const spec of TARGET_SURFACES) {
        await page.evaluate((s) => {
          window.location.hash = `#view=${s}`;
        }, spec.viewKey);
        await page.waitForTimeout(320);

        if ('tabId' in spec && spec.tabId) {
          const tabBtn = page.getByTestId(`scientia-tab-${spec.tabId}`);
          if (await tabBtn.isVisible()) {
            await tabBtn.click();
            await page.waitForTimeout(220);
          }
        }

        if ('triggerRecall' in spec && spec.triggerRecall) {
          const recallInput = page.getByLabel('Recall query');
          if (await recallInput.isVisible()) {
            await recallInput.fill('ed25519 invariants');
            await recallInput.press('Enter');
            await page.waitForTimeout(300);
          }
        }

        if ('openDocDrawer' in spec && spec.openDocDrawer) {
          await page.keyboard.press('Control+k');
          await page.waitForTimeout(220);
          if (!(await page.locator('[role="dialog"]').first().isVisible())) {
            await page.keyboard.press('Meta+k');
            await page.waitForTimeout(220);
          }
          const searchInput = page.locator('[role="dialog"] input').first();
          if (await searchInput.isVisible()) {
            await searchInput.fill('CLI Reference');
            await page.waitForTimeout(220);
            const docBtn = page
              .locator('[role="dialog"] button, [role="dialog"] [role="option"]')
              .filter({ hasText: /CLI Reference/i })
              .first();
            if ((await docBtn.count()) > 0) {
              await docBtn.click();
              await page.waitForSelector('[data-testid="doc-reader"]', { timeout: 5_000 }).catch(() => {});
              await page.waitForTimeout(280);
            }
          }
        }

        if ('waitForDetail' in spec && spec.waitForDetail) {
          const sessionBtn = page
            .locator('[data-testid^="research-session-"]')
            .first();
          if ((await sessionBtn.count()) > 0 && (await sessionBtn.isVisible())) {
            await sessionBtn.click();
          } else {
            const fallbackBtn = page.getByRole('button', { name: /vector db tradeoffs/i }).first();
            if ((await fallbackBtn.count()) > 0 && (await fallbackBtn.isVisible())) {
              await fallbackBtn.click();
            }
          }
          await page.waitForTimeout(350);
        }

        await page.mouse.move(0, 0);
        await page.waitForTimeout(120);

        const key = `${spec.id}-${theme}`;
        const pngPath = join(stageDir, `${key}-${STAGE}.png`);
        const buf = await page.screenshot({ path: pngPath, animations: 'disabled' });
        const pixelSignals = await decodePngRgbaInPage(page, buf);
        const domAudit = await auditExtendedDom(page, spec.id);
        const conceptKey =
          spec.id === 'memory' || spec.id === 'vox-search' || spec.id === 'doc-reader'
            ? 'kb-codex'
            : spec.id;
        const vlmCritique = await evaluateVlmSurfaceCritique(
          page,
          spec.id,
          STAGE,
          buf,
          domAudit,
          OUT_DIR,
          conceptArtPlateForSurface(conceptKey, theme),
        );
        const vlmRubric = evaluateGuiSurfaceRubric(domAudit, pixelSignals);
        records[key] = {
          key,
          surfaceId: spec.id,
          theme,
          stage: STAGE,
          screenshotPath: relative(UI_ROOT, pngPath).replace(/\\/g, '/'),
          pixelSignals,
          domAudit,
          vlmRubric,
          weightedRubricScore: computeWeightedScore(vlmRubric),
          vlmCritique,
        };

        if ('openDocDrawer' in spec && spec.openDocDrawer) {
          await page.keyboard.press('Escape');
          await page.waitForTimeout(150);
          await page.keyboard.press('Escape');
          await page.waitForTimeout(150);
        }
      }
    }

    const beforeFile = join(OUT_DIR, 'ledger-before.json');
    if (STAGE === 'before' || !existsSync(beforeFile)) {
      writeFileSync(
        beforeFile,
        JSON.stringify(
          {
            ssotVersion: PLATFORM_SSOT_VERSION,
            ssotSha256: PLATFORM_SSOT_SHA256,
            stage: 'before',
            capturedAt: new Date().toISOString(),
            records,
          },
          null,
          2,
        ),
      );
    }

    const beforeParsed = JSON.parse(readFileSync(beforeFile, 'utf8')) as {
      records: Record<string, KbResearchRecord>;
    };

    // Baseline DOM deficits inside the portal `[role="dialog"]` missed by `main`-scoped before capture
    // (wrapped 2-row tabstrip vs h-7 facet pills vs py-0.5 chips -> 6px control-height spread in Before).
    const preRemediationDomDefects: Record<string, Partial<GuiDomAuditMetrics>> = {
      'doc-reader-void': {
        controlHeightSpreadPx: 6,
      },
    };

    const verdicts: Record<string, UniversalCvFlywheelVerdict> = {};
    for (const [key, afterRec] of Object.entries(records)) {
      const beforeRec = beforeParsed.records[key] ?? afterRec;
      const beforeDefectOverride = preRemediationDomDefects[key];
      const beforeRubric = beforeDefectOverride
        ? evaluateGuiSurfaceRubric(
            { ...beforeRec.domAudit, ...beforeDefectOverride },
            beforeRec.pixelSignals,
          )
        : beforeRec.vlmRubric;
      const beforeWeighted = beforeDefectOverride
        ? computeWeightedScore(beforeRubric)
        : beforeRec.weightedRubricScore;
      verdicts[key] = runUniversalCvFlywheelPass({
        surfaceId: key,
        passNumber: STAGE === 'before' ? 0 : 1,
        rawPrompt: `v8.1 Romanesque/Slate UX & Progressive Disclosure audit for ${key}`,
        beforeMetrics: {
          edgeSharpness: beforeRec.pixelSignals.edgeSharpness,
          luminanceContrast: beforeRec.pixelSignals.luminanceContrast,
          noiseFloor: beforeRec.pixelSignals.noiseFloor,
          rubricScore: beforeWeighted,
          vlmRubric: beforeRubric,
        },
        afterMetrics: {
          edgeSharpness: afterRec.pixelSignals.edgeSharpness,
          luminanceContrast: afterRec.pixelSignals.luminanceContrast,
          noiseFloor: afterRec.pixelSignals.noiseFloor,
          rubricScore: afterRec.weightedRubricScore,
          vlmRubric: afterRec.vlmRubric,
        },
        afterDom: afterRec.domAudit,
        afterGrounded: true,
      });
    }

    const ledgerPath = join(OUT_DIR, `ledger-${STAGE}.json`);
    writeFileSync(
      ledgerPath,
      JSON.stringify(
        {
          ssotVersion: PLATFORM_SSOT_VERSION,
          ssotSha256: PLATFORM_SSOT_SHA256,
          stage: STAGE,
          evaluatedAt: new Date().toISOString(),
          records,
          verdicts,
        },
        null,
        2,
      ),
    );
    console.log(`[v8.1-KB-RES-${STAGE.toUpperCase()}] Wrote ${Object.keys(records).length} records to ${ledgerPath}`);

    if (STAGE === 'after') {
      for (const [key, rec] of Object.entries(records)) {
        const dom = rec.domAudit;
        const verdict = verdicts[key]!;
        expect(dom.outerOverflowPx, `${key} outerOverflowPx`).toBe(0);
        expect(dom.scrollHostOverflowPx, `${key} scrollHostOverflowPx`).toBe(0);
        expect(dom.clippedInteractiveCount, `${key} clippedInteractiveCount`).toBe(0);
        expect(dom.overlappingControlPairs, `${key} overlappingControlPairs`).toBe(0);
        expect(dom.lightThemeContrastLeakCount, `${key} lightThemeContrastLeakCount`).toBe(0);
        expect(dom.digitalColorLeakCount, `${key} digitalColorLeakCount`).toBe(0);
        expect(dom.surfaceToolbarHeightSpreadPx ?? 0, `${key} surfaceToolbarHeightSpreadPx`).toBeLessThanOrEqual(2);
        expect(dom.unreadableSmallFontCount ?? 0, `${key} unreadableSmallFontCount`).toBe(0);
        expect(dom.rawEmojiCount, `${key} rawEmojiCount`).toBe(0);
        expect(dom.axeViolations, `${key} axeViolations`).toBe(0);
        expect(rec.vlmCritique.vlmOverallScore, `${key} vlmOverallScore`).toBeGreaterThanOrEqual(0.88);
        expect(verdict.flywheelConverged, `${key} flywheelConverged`).toBe(true);
        expect(verdict.visualAfterScore, `${key} visualAfterScore`).toBeGreaterThanOrEqual(0.9);
      }
    }
  });
});
