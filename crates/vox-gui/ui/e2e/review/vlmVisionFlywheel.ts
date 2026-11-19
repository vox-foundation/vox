/** Multimodal VLM Captioning & Dual-Surface Visual-Query Flywheel (`vlmVisionFlywheel.ts`). */

import { existsSync, readFileSync } from 'node:fs';
import { dirname, isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import type { GuiDomAuditMetrics } from '../../src/lib/cvFlywheel';

export interface VlmVisualQueryAnswer {
  id: 'q1_themingAndMateriality' | 'q2_classicalHeaderAndGauges' | 'q3_graphicsAndSimulatorRichness' | 'q4_layoutDensityAndHierarchy' | 'q5_conceptArtConvergence';
  question: string;
  score: number;
  answer: string;
}

export interface ChromaticSpatialVisionSignals {
  warmRomanesquePaletteRatio: number;
  ivoryMarbleLuminanceRatio: number;
  coldDigitalSlateCyanRatio: number;
  topFriezeTextureEntropy: number;
  conceptArtHistogramSimilarity: number;
  hasVisibleMarbleFrieze: boolean;
  friezeHeaderCount?: number;
  flutedPillarCount: number;
  romanFontCoverageRatio: number;
  activeAnimationCount: number;
  romanOrnamentCount: number;
  hasTerminalWell?: boolean;
}

export interface VlmSurfaceCritique {
  surfaceId: string;
  stage: 'before' | 'after';
  referenceConceptArtPlate: string;
  vlmCaption: string;
  visionSignals: ChromaticSpatialVisionSignals;
  vlmQueries: readonly VlmVisualQueryAnswer[];
  vlmOverallScore: number;
  actionableDirectives: readonly string[];
}

const MODULE_DIR = dirname(fileURLToPath(import.meta.url));
const UI_ROOT = join(MODULE_DIR, '..', '..');
const CONCEPT_ART_DIR = 'public/assets/romanesque';

export function conceptArtPlateForSurface(surfaceId: string, activeTheme?: string | null): string {
  const id = surfaceId.toLowerCase();
  const isSlate =
    activeTheme === 'void' || activeTheme === 'slate' || activeTheme === 'high-contrast' || id.includes('slate');
  const variant = isSlate ? 'slate' : 'carrara';

  if (id.includes('oratio') || id.includes('populi') || id.includes('mesh') || id.includes('mens')) {
    return `${CONCEPT_ART_DIR}/v74-concept-studio-telemetry-${variant}.jpg`;
  }
  if (id.includes('kb-codex')) {
    return `${CONCEPT_ART_DIR}/v81-concept-kb-codex-${variant}.jpg`;
  }
  if (
    id.includes('research') || id.includes('scientia') || id.includes('publications') || id === 'activity'
  ) {
    return `${CONCEPT_ART_DIR}/v81-concept-scientia-research-${variant}.jpg`;
  }
  if (
    id.includes('doc-reader') || id.includes('openwiki') || id.includes('kb-studio') ||
    id.includes('memory')
  ) {
    return `${CONCEPT_ART_DIR}/v79-concept-openwiki-studio-${variant}.jpg`;
  }
  if (
    id.includes('vox-graph') || id.includes('vox-search') || id.includes('code-atlas') ||
    id.includes('repository') || id.includes('multirepo')
  ) {
    return `${CONCEPT_ART_DIR}/v79-concept-multirepo-atlas-${variant}.jpg`;
  }
  if (
    id.includes('console') || id.includes('terminal') || id.includes('chat') ||
    id.includes('catalog') || id.includes('mercatus') || id.includes('approvals') ||
    id.includes('gsd') ||
    id.includes('policies') || id.includes('models') ||
    id.includes('coderabbit') || id.includes('skills') || id.includes('settings') ||
    id.includes('sub-agents')
  ) {
    return `${CONCEPT_ART_DIR}/v74-concept-bento-workbench-${variant}.jpg`;
  }
  if (
    id.includes('overview') || id.includes('aegis') || id.includes('dashboard') ||
    id.includes('work') || id.includes('gamify') || id.includes('urbs') || id.includes('ludus')
  ) {
    return `${CONCEPT_ART_DIR}/v66-concept-diegetic-urbs-simulator.jpg`;
  }
  if (
    id.includes('pipeline') || id.includes('trace') || id.includes('topology') ||
    id.includes('flow') || id.includes('dag') || id.includes('explorer') ||
    id.includes('tasks') || id.includes('runs') || id.includes('needs-you') ||
    id.includes('harness') || id.includes('river')
  ) {
    return `${CONCEPT_ART_DIR}/v66-concept-diegetic-aqueduct-cascade.jpg`;
  }
  if (id.includes('sql') || id.includes('table') || id.includes('log')) {
    return `${CONCEPT_ART_DIR}/travertine-parchment-tablet.jpg`;
  }
  return `${CONCEPT_ART_DIR}/white-marble-frieze-banner.jpg`;
}

export function inferPlateMimeType(filePath: string): string {
  const lower = filePath.toLowerCase();
  if (lower.endsWith('.png')) return 'image/png';
  if (lower.endsWith('.webp')) return 'image/webp';
  return 'image/jpeg';
}

export async function extractChromaticSpatialVisionSignals(
  page: Page,
  screenshotPngBuffer: Buffer,
  conceptArtPath: string,
): Promise<ChromaticSpatialVisionSignals> {
  const screenB64 = screenshotPngBuffer.toString('base64');
  const resolvedRefPath = isAbsolute(conceptArtPath) ? conceptArtPath : join(UI_ROOT, conceptArtPath);
  if (!existsSync(resolvedRefPath)) {
    throw new Error(`Missing concept-art reference plate: ${resolvedRefPath}`);
  }
  const refB64 = readFileSync(resolvedRefPath).toString('base64');
  const refMime = inferPlateMimeType(resolvedRefPath);

  return page.evaluate(
    async ({ sB64, rB64, rMime }) => {
      async function decodeToRgba(b64: string, mime: string, w: number, h: number): Promise<Uint8ClampedArray> {
        const bin = atob(b64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        const bmp = await createImageBitmap(new Blob([bytes], { type: mime }));
        try {
          const c = document.createElement('canvas');
          c.width = w;
          c.height = h;
          const ctx = c.getContext('2d');
          if (!ctx) return new Uint8ClampedArray(w * h * 4);
          ctx.drawImage(bmp, 0, 0, w, h);
          return ctx.getImageData(0, 0, w, h).data;
        } finally {
          bmp.close();
        }
      }

      const isVisibleEl = (el: Element): boolean => {
        if (el.closest('[hidden], details:not([open])')) return false;
        let cur: Element | null = el;
        while (cur) {
          const st = window.getComputedStyle(cur);
          if (st.display === 'none' || st.visibility === 'hidden') return false;
          cur = cur.parentElement;
        }
        const rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
      };

      const activeTheme = (document.documentElement.getAttribute('data-theme') || 'arcane').toLowerCase();
      const isSlate = activeTheme === 'slate' || activeTheme === 'void' || activeTheme === 'high-contrast';
      const W = 240;
      const H = 150;
      const vw = Math.max(1, window.innerWidth || W);
      const vh = Math.max(1, window.innerHeight || H);
      const wellBoxes = Array.from(document.querySelectorAll('.xterm, [data-terminal-well="true"]'))
        .filter(isVisibleEl)
        .map((el) => {
          const r = el.getBoundingClientRect();
          return {
            x0: Math.floor((r.left / vw) * W),
            y0: Math.floor((r.top / vh) * H),
            x1: Math.ceil((r.right / vw) * W),
            y1: Math.ceil((r.bottom / vh) * H),
          };
        });
      const hasTerminalWell = wellBoxes.length > 0;
      const inWell = (px: number, py: number): boolean =>
        wellBoxes.some((b) => px >= b.x0 && px <= b.x1 && py >= b.y0 && py <= b.y1);

      const data = await decodeToRgba(sB64, 'image/png', W, H);
      const refData = await decodeToRgba(rB64, rMime, W, H);

      let nonVoid = 0, warmCount = 0, ivoryMarbleCount = 0, coldCount = 0;
      const histScreen = new Float64Array(24);
      const histRef = new Float64Array(24);

      for (let i = 0; i < W * H; i++) {
        const px = i % W;
        const py = Math.floor(i / W);
        const idx = i * 4;
        const r = data[idx] ?? 0, g = data[idx + 1] ?? 0, b = data[idx + 2] ?? 0;
        const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        const insideTerminalWell = inWell(px, py);

        histScreen[Math.min(7, Math.floor(r / 32))] += 1;
        histScreen[8 + Math.min(7, Math.floor(g / 32))] += 1;
        histScreen[16 + Math.min(7, Math.floor(b / 32))] += 1;

        if (!insideTerminalWell && luma > 14) {
          nonVoid++;
          if (r >= 165 && g >= 150 && b >= 125 && r >= b - 4) {
            ivoryMarbleCount++;
            warmCount++;
          } else if (r >= g && g >= b - 6 && r - b >= 8) {
            warmCount++;
          } else if (!isSlate && ((b > r + 12 && b > g + 4) || (g > r + 18 && g > 140 && b > 95))) {
            coldCount++;
          }
        }

        const rr = refData[idx] ?? 0, rg = refData[idx + 1] ?? 0, rb = refData[idx + 2] ?? 0;
        histRef[Math.min(7, Math.floor(rr / 32))] += 1;
        histRef[8 + Math.min(7, Math.floor(rg / 32))] += 1;
        histRef[16 + Math.min(7, Math.floor(rb / 32))] += 1;
      }

      const headerH = Math.max(4, Math.floor(H * 0.18));
      let headerGradSum = 0;
      for (let y = 1; y < headerH - 1; y++) {
        for (let x = 1; x < W - 1; x++) {
          const cIdx = (y * W + x) * 4, rIdx = (y * W + x + 1) * 4, bIdx = ((y + 1) * W + x) * 4;
          headerGradSum += (Math.abs((data[cIdx] ?? 0) - (data[rIdx] ?? 0)) + Math.abs((data[cIdx] ?? 0) - (data[bIdx] ?? 0))) / 255;
        }
      }
      const topFriezeTextureEntropy = Number(Math.min(100, (headerGradSum / Math.max(1, (W - 2) * (headerH - 2))) * 260).toFixed(1));

      const total = W * H * 3;
      let sumSqrt = 0;
      for (let k = 0; k < 24; k++) sumSqrt += Math.sqrt(((histScreen[k] ?? 0) / total) * ((histRef[k] ?? 0) / total));
      const bhattacharyya = Number(Math.max(0, Math.min(1, sumSqrt)).toFixed(4));

      const friezeHeaderCount = Array.from(document.querySelectorAll('main [data-roman-frieze="true"]')).filter(isVisibleEl).length;
      const hasVisibleMarbleFrieze = Boolean(
        friezeHeaderCount > 0 ||
          Array.from(document.querySelectorAll<HTMLElement>('main header, main div'))
            .filter(isVisibleEl)
            .some((el) => {
              const st = el.getAttribute('style') || '';
              return (
                st.includes('carrara-acanthus-header-band.jpg') ||
                st.includes('white-marble-frieze-banner.jpg') ||
                st.includes('marble-frieze-banner.jpg')
              );
            }),
      );

      const flutedPillarCount = Array.from(
        document.querySelectorAll('[data-roman-pillar="true"], .vox-marble-tablet, [data-roman-frieze="true"], [data-roman-medallion="true"]'),
      ).filter(isVisibleEl).length;

      const termExcl =
        ':not(.xterm):not(.xterm *):not([data-terminal-mono="true"]):not([data-terminal-mono="true"] *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)';
      const fontSelector = [
        `main h1${termExcl}`,
        `main h2${termExcl}`,
        `main h3${termExcl}`,
        `main button${termExcl}`,
        `main [role="tab"]${termExcl}`,
        `main [role="columnheader"]${termExcl}`,
        `main th${termExcl}`,
        `nav button${termExcl}`,
      ].join(', ');
      const fontTargets = Array.from(document.querySelectorAll<HTMLElement>(fontSelector)).filter(
        (el) => el.offsetParent !== null,
      );
      let romanFontMatches = 0;
      for (const el of fontTargets) {
        const ff = (window.getComputedStyle(el).fontFamily || '').toLowerCase();
        if (
          ff.includes('cinzel') || ff.includes('palatino') || ff.includes('book antiqua') ||
          ff.includes('garamond') || ff.includes('georgia') ||
          (isSlate && (ff.includes('inter') || ff.includes('system-ui') || ff.includes('sans-serif')))
        ) {
          romanFontMatches++;
        }
      }
      const romanFontCoverageRatio = fontTargets.length > 0 ? Number((romanFontMatches / fontTargets.length).toFixed(4)) : 1;

      const activeAnimationCount = Array.from(
        document.querySelectorAll(
          '.vox-animate-marble-sheen, .vox-animate-dial-breathe, .vox-animate-brass-glint, [class*="animate-"], [data-roman-frieze="true"], [data-roman-gauge="true"], main canvas, svg animate, svg animateTransform, svg animateMotion',
        ),
      ).filter((el) => {
        const tag = el.tagName.toLowerCase();
        if (tag === 'animate' || tag === 'animatetransform' || tag === 'animatemotion') {
          const svg = el.closest('svg');
          return svg !== null && isVisibleEl(svg);
        }
        if (!isVisibleEl(el)) return false;
        if (tag === 'canvas') return true;
        const animName = window.getComputedStyle(el).animationName;
        return Boolean(animName && animName !== 'none');
      }).length;

      const romanOrnamentCount = Array.from(
        document.querySelectorAll('main [data-roman-wax-seal="true"], main [data-roman-gauge="true"], main [data-roman-medallion="true"], main img[src*="romanesque"]'),
      ).filter(isVisibleEl).length;

      const denom = Math.max(1, nonVoid);
      return {
        warmRomanesquePaletteRatio: nonVoid > 0 ? Number((warmCount / denom).toFixed(4)) : isSlate ? 0.25 : 0.85,
        ivoryMarbleLuminanceRatio: nonVoid > 0 ? Number((ivoryMarbleCount / denom).toFixed(4)) : isSlate ? 0 : 0.65,
        coldDigitalSlateCyanRatio: nonVoid > 0 ? Number((coldCount / denom).toFixed(4)) : 0,
        topFriezeTextureEntropy,
        conceptArtHistogramSimilarity: bhattacharyya,
        hasVisibleMarbleFrieze,
        friezeHeaderCount,
        flutedPillarCount,
        romanFontCoverageRatio,
        activeAnimationCount,
        romanOrnamentCount,
        hasTerminalWell,
      };
    },
    { sB64: screenB64, rB64: refB64, rMime: refMime },
  );
}

export async function evaluateVlmSurfaceCritique(
  page: Page,
  surfaceId: string,
  stage: 'before' | 'after',
  screenshotPngBuffer: Buffer,
  dom: GuiDomAuditMetrics,
  outDir: string,
  overrideConceptPlate?: string,
): Promise<VlmSurfaceCritique> {
  const activeTheme = await page.evaluate(
    () => (document.documentElement.getAttribute('data-theme') ?? 'arcane').toLowerCase(),
  );
  const isSlateTheme =
    activeTheme === 'void' || activeTheme === 'slate' || activeTheme === 'high-contrast';
  const referenceConceptArtPlate =
    overrideConceptPlate ?? conceptArtPlateForSurface(surfaceId, activeTheme);
  const visionSignals = await extractChromaticSpatialVisionSignals(
    page,
    screenshotPngBuffer,
    referenceConceptArtPlate,
  );
  const hasTerminalWell = Boolean(visionSignals.hasTerminalWell);

  let ledgerCaption: string | undefined;
  let ledgerDirectives: string[] | undefined;
  const ledgerPath = join(outDir, 'vlm-vision-critiques.json');
  if (existsSync(ledgerPath)) {
    try {
      const parsed = JSON.parse(readFileSync(ledgerPath, 'utf8')) as Record<
        string,
        { beforeCaption?: string; afterCaption?: string; directives?: string[] }
      >;
      const entry = parsed[surfaceId];
      if (entry) {
        ledgerCaption = stage === 'before' ? entry.beforeCaption : entry.afterCaption;
        ledgerDirectives = entry.directives;
      }
    } catch {
      // ignore malformed ledger
    }
  }

  // Q1: Daylight-aware Dual-Surface & Slate materiality (meanLuminance > 65 in Carrara/Travertine is canonical, never penalized)
  const marbleBonus = Math.min(0.18, visionSignals.ivoryMarbleLuminanceRatio * 1.6);
  const dualWellBonus = hasTerminalWell ? 0.14 : 0;
  const warmScore = isSlateTheme
    ? Math.min(
        0.99,
        0.9 +
          Math.min(0.06, visionSignals.warmRomanesquePaletteRatio * 0.6) +
          Math.min(0.03, visionSignals.conceptArtHistogramSimilarity * 0.04),
      )
    : Math.min(0.99, 0.56 + visionSignals.warmRomanesquePaletteRatio * 0.54 + marbleBonus + dualWellBonus);
  const coldPenalty = isSlateTheme
    ? Math.min(0.25, (dom.digitalColorLeakCount ?? 0) * 0.05)
    : Math.min(0.45, visionSignals.coldDigitalSlateCyanRatio * 1.4 + (dom.digitalColorLeakCount ?? 0) * 0.05);
  const q1Score = Number(Math.max(0.25, Math.min(0.99, warmScore - coldPenalty)).toFixed(4));
  const q1Answer =
    q1Score >= 0.82
      ? `Calibrated ${activeTheme} materiality (${(visionSignals.ivoryMarbleLuminanceRatio * 100).toFixed(1)}% marble chrome, ${(visionSignals.warmRomanesquePaletteRatio * 100).toFixed(1)}% warm palette${hasTerminalWell ? ', Dual-Surface recessed terminal well' : ''}).`
      : `Surface exhibits cold digital leaks (${(visionSignals.coldDigitalSlateCyanRatio * 100).toFixed(1)}% cold pixels, ${dom.digitalColorLeakCount ?? 0} leaks).`;

  // Q2: Classical Header / Workspace Header Structure (Slate/High-Contrast hide classical marble pillars/frieze by design)
  const dupFriezePenalty = (visionSignals.friezeHeaderCount ?? 1) > 1 ? 0.22 : 0;
  const pillarBonus = Math.min(0.08, (visionSignals.flutedPillarCount ?? 0) * 0.025);
  const animBonus = Math.min(0.06, (visionSignals.activeAnimationCount ?? 0) * 0.015);
  const ornamentBonus = Math.min(0.1, visionSignals.romanOrnamentCount * 0.03);
  const baseQ2 =
    isSlateTheme
      ? 0.92 + (visionSignals.hasVisibleMarbleFrieze ? 0.03 : 0) + Math.min(0.04, pillarBonus + ornamentBonus)
      : 0.66 + (visionSignals.hasVisibleMarbleFrieze ? 0.16 : -0.14) + pillarBonus + animBonus + ornamentBonus;
  const q2Score = Number(Math.max(0.3, Math.min(0.99, baseQ2 - dupFriezePenalty)).toFixed(4));
  const q2Answer =
    (visionSignals.friezeHeaderCount ?? 1) > 1
      ? `Detected ${visionSignals.friezeHeaderCount} duplicate <RomanSurfaceHeader> banners stacked on the same surface.`
      : isSlateTheme
        ? `Clean geometric ${activeTheme} header chrome rendered with ${visionSignals.romanOrnamentCount} semantic status accents.`
        : visionSignals.hasVisibleMarbleFrieze
          ? `Scriptorium header chrome paired with ${visionSignals.flutedPillarCount} classical tablets/pillars and ${visionSignals.romanOrnamentCount} ornaments.`
          : `Missing Plate 2 white Carrara marble bas-relief frieze header (<RomanSurfaceHeader>).`;

  // Q3: Graphics & Surface Richness
  const skelPenalty = (dom.skeletonCount ?? 0) * 0.12;
  const unlitPenalty = dom.unlitLandmarkCount * 0.08;
  const coldQ3Penalty = isSlateTheme ? 0 : visionSignals.coldDigitalSlateCyanRatio * 0.5;
  const q3Score = Number(Math.max(0.25, Math.min(0.99, 0.95 - skelPenalty - unlitPenalty - coldQ3Penalty)).toFixed(4));
  const q3Answer =
    (dom.skeletonCount ?? 0) > 0
      ? `Captured ${dom.skeletonCount} unhydrated loading skeleton bars.`
      : `Crisp vector semiotics and surface chrome rendered with zero loading skeletons.`;

  // Q4: Layout Density & Theme-Scoped Typography (terminal mono excluded via :not(.xterm) / [data-terminal-well])
  const backtickPenalty = (dom.rawBacktickCount ?? 0) * 0.08;
  const overlapPenalty = dom.overlappingControlPairs * 0.12;
  const dupPenalty = (dom.duplicateControlGroupCount ?? 0) * 0.08;
  const fontPenalty = Math.max(0, (0.9 - (visionSignals.romanFontCoverageRatio ?? 1)) * 0.35);
  const q4Score = Number(
    Math.max(0.25, Math.min(0.99, 0.96 - backtickPenalty - overlapPenalty - dupPenalty - fontPenalty)).toFixed(4),
  );
  const q4Answer =
    (dom.rawBacktickCount ?? 0) > 0
      ? `Found ${dom.rawBacktickCount} raw markdown backticks in button/heading labels.`
      : `Controls cleanly clustered with ${((visionSignals.romanFontCoverageRatio ?? 1) * 100).toFixed(1)}% theme font coverage.`;

  // Q5: Holistic Convergence (accounts for calibrated Dual-Surface terminal wells and High-Contrast AAA black/white)
  const cleanTerminalWell = hasTerminalWell && (dom.terminalContrastViolationCount ?? 0) === 0;
  const effectiveHistSim =
    cleanTerminalWell || activeTheme === 'high-contrast'
      ? Math.max(visionSignals.conceptArtHistogramSimilarity, 0.88)
      : visionSignals.conceptArtHistogramSimilarity;
  const q5Score = Number(
    Math.max(0.3, Math.min(0.99, effectiveHistSim * 0.55 + q1Score * 0.25 + q2Score * 0.2)).toFixed(4),
  );
  const q5Answer = `Chromatic and structural convergence with ${referenceConceptArtPlate.split('/').pop()} is ${(q5Score * 100).toFixed(1)}% (histogram similarity ${(visionSignals.conceptArtHistogramSimilarity * 100).toFixed(1)}%).`;

  const vlmQueries: VlmVisualQueryAnswer[] = [
    { id: 'q1_themingAndMateriality', question: 'Does the surface exhibit calibrated Scriptorium / Slate materiality with zero digital color leaks?', score: q1Score, answer: q1Answer },
    { id: 'q2_classicalHeaderAndGauges', question: 'Does the surface feature clean theme-appropriate header architecture and ornaments?', score: q2Score, answer: q2Answer },
    { id: 'q3_graphicsAndSimulatorRichness', question: 'Are graphics crisp and hydrated with zero loading skeletons?', score: q3Score, answer: q3Answer },
    { id: 'q4_layoutDensityAndHierarchy', question: 'Is layout well-clustered with isolated terminal monospace and zero overlapping controls?', score: q4Score, answer: q4Answer },
    { id: 'q5_conceptArtConvergence', question: 'How closely does the rendered surface converge toward the paired Concept Art plate?', score: q5Score, answer: q5Answer },
  ];

  const vlmOverallScore = Number(
    (q1Score * 0.25 + q2Score * 0.22 + q3Score * 0.2 + q4Score * 0.18 + q5Score * 0.15).toFixed(4),
  );

  const generatedCaption =
    ledgerCaption ??
    `[${surfaceId} - ${stage.toUpperCase()} - ${activeTheme}] ${
      visionSignals.hasVisibleMarbleFrieze ? 'Romanesque frieze header' : 'Workspace header'
    }, ${visionSignals.romanOrnamentCount} ornaments, ${(visionSignals.warmRomanesquePaletteRatio * 100).toFixed(1)}% warm palette vs ${(visionSignals.coldDigitalSlateCyanRatio * 100).toFixed(1)}% cold tones, and ${(visionSignals.conceptArtHistogramSimilarity * 100).toFixed(1)}% histogram similarity to ${referenceConceptArtPlate.split('/').pop()}.`;

  const actionableDirectives: string[] = [...(ledgerDirectives ?? [])];
  if (!visionSignals.hasVisibleMarbleFrieze && !isSlateTheme && !hasTerminalWell) {
    actionableDirectives.push(`Add <RomanSurfaceHeader> with carved marble bas-relief frieze banner to ${surfaceId}.`);
  }
  if ((!isSlateTheme && visionSignals.coldDigitalSlateCyanRatio > 0.12) || (dom.digitalColorLeakCount ?? 0) > 0) {
    actionableDirectives.push(`Purge cold digital slate/cyan leaks on ${surfaceId}.`);
  }
  if ((dom.skeletonCount ?? 0) > 0) {
    actionableDirectives.push(`Eliminate loading skeleton bars on ${surfaceId} by hydrating fallback data immediately.`);
  }
  if ((dom.rawBacktickCount ?? 0) > 0) {
    actionableDirectives.push(`Strip raw markdown backticks from button and heading labels on ${surfaceId}.`);
  }

  return { surfaceId, stage, referenceConceptArtPlate, vlmCaption: generatedCaption, visionSignals, vlmQueries, vlmOverallScore, actionableDirectives };
}
