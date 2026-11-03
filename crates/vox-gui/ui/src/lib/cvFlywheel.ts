/**
 * Shared-Package Computer Vision (CV) Flywheel for `vox-gui` (`@ngi/platform-ssot`).
 *
 * Bridges the synced `platformSsot.generated.ts` (`evaluateBeforeAfterCvFlywheel`,
 * `evaluateWcagContrastRatio`, `PLATFORM_SSOT_VERSION`, `PLATFORM_SSOT_SHA256`) with:
 * - Real RGBA pixel signal extraction (Sobel gradient edge sharpness, RMS + P95/P10 luminance contrast,
 *   and 4-neighbor Laplacian flat-region noise floor).
 * - Universal Before/After CV Flywheel pass runner (`runUniversalCvFlywheelPass`).
 * - GUI surface DOM + visual quality rubric synthesis for VS Code-grade Chat, Docking, and Urbs Civilization audits.
 */

import {
  PLATFORM_SSOT_SHA256,
  PLATFORM_SSOT_VERSION,
  evaluateBeforeAfterCvFlywheel,
  evaluateWcagContrastRatio as ssotEvaluateWcagContrastRatio,
} from './platformSsot.generated';

export {
  PLATFORM_SSOT_VERSION,
  PLATFORM_SSOT_SHA256,
  evaluateBeforeAfterCvFlywheel,
};

function srgbTupleRelativeLuminance(rgb: readonly [number, number, number]): number {
  const toLinear = (c: number) => {
    const v = Math.max(0, Math.min(255, c)) / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * toLinear(rgb[0]) + 0.7152 * toLinear(rgb[1]) + 0.0722 * toLinear(rgb[2]);
}

export function evaluateWcagContrastRatio(
  fg: number | readonly [number, number, number],
  bg: number | readonly [number, number, number],
) {
  const fgLum = typeof fg === 'number' ? fg : srgbTupleRelativeLuminance(fg);
  const bgLum = typeof bg === 'number' ? bg : srgbTupleRelativeLuminance(bg);
  const res = ssotEvaluateWcagContrastRatio(fgLum, bgLum);
  return {
    ...res,
    ratio: res.contrastRatio,
    passesAA: res.passesAa,
    passesAAA: res.passesAaa,
  };
}

export interface VisionRubricScores {
  technicalQuality: number;
  promptAdherence: number;
  anatomicalAccuracy: number;
  compositionQuality: number;
  styleConsistency: number;
}

export interface FrameCvMetrics {
  edgeSharpness: number;
  luminanceContrast: number;
  noiseFloor: number;
  vlmRubric: VisionRubricScores;
}

export interface UniversalCvFlywheelInput {
  surfaceId: string;
  passNumber: number;
  rawPrompt: string;
  targetModelId?: string;
  beforeMetrics: {
    edgeSharpness: number;
    luminanceContrast: number;
    noiseFloor: number;
    rubricScore: number;
    vlmRubric?: VisionRubricScores;
  };
  afterMetrics: {
    edgeSharpness: number;
    luminanceContrast: number;
    noiseFloor: number;
    rubricScore: number;
    vlmRubric?: VisionRubricScores;
  };
  afterGrounded?: boolean;
}

export interface UniversalCvFlywheelVerdict {
  surfaceId: string;
  passNumber: number;
  visualBeforeScore: number;
  visualAfterScore: number;
  visualDelta: number;
  edgeSharpnessDeltaPct: number;
  contrastDeltaPct: number;
  noiseReductionPct: number;
  estimatedPsnrDb: number;
  flywheelConverged: boolean;
  promoteToGoldenBaseline: boolean;
  nextBootstrapDirectives: readonly string[];
}

export interface GuiDomAuditMetrics {
  surfaceId: string;
  outerOverflowPx: number;
  scrollHostOverflowPx?: number;
  clippedInteractiveCount: number;
  overlappingControlPairs: number;
  unstyledNativeSelectCount: number;
  lightThemeContrastLeakCount?: number;
  iconIssueCount?: number;
  controlHeightSpreadPx: number;
  rawEmojiCount: number;
  unlitLandmarkCount: number;
  missingDockPanelIds: readonly string[];
  axeViolations: number;
  viewportCoverageRatio: number;
}

export function makeRubricFromScalar(score: number): VisionRubricScores {
  const normalized =
    score > 1 ? Math.max(0, Math.min(1, score / 100)) : Math.max(0, Math.min(1, score));
  return {
    technicalQuality: normalized,
    promptAdherence: normalized,
    anatomicalAccuracy: normalized,
    compositionQuality: normalized,
    styleConsistency: normalized,
  };
}

/**
 * Computes real pixel-level CV signals from an RGBA buffer (HTML5 Canvas ImageData or PNG decode):
 * - `edgeSharpness` (0..100): 3x3 Sobel gradient magnitude across interior pixels.
 * - `luminanceContrast` (0..100): BT.709 relative luminance RMS contrast + P95-P10 dynamic range.
 * - `noiseFloor` (0..100): 4-neighbor Laplacian high-frequency variance in low-gradient regions.
 */
export function computeRgbaFrameCvMetrics(
  rgba: Uint8ClampedArray | Uint8Array,
  width: number,
  height: number,
): {
  edgeSharpness: number;
  luminanceContrast: number;
  noiseFloor: number;
  meanLuminance: number;
} {
  const totalPixels = Math.max(1, width * height);
  if (width < 4 || height < 4 || rgba.length < totalPixels * 4) {
    return {
      edgeSharpness: 50,
      luminanceContrast: 50,
      noiseFloor: 20,
      meanLuminance: 0.12,
    };
  }

  const luma = new Float32Array(totalPixels);
  let sumLuma = 0;
  for (let i = 0; i < totalPixels; i++) {
    const idx = i * 4;
    const r = rgba[idx] ?? 0;
    const g = rgba[idx + 1] ?? 0;
    const b = rgba[idx + 2] ?? 0;
    const y = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0;
    luma[i] = y;
    sumLuma += y;
  }
  const meanLuma = sumLuma / totalPixels;

  let varSum = 0;
  const hist = new Uint32Array(256);
  for (let i = 0; i < totalPixels; i++) {
    const y = luma[i] ?? 0;
    const diff = y - meanLuma;
    varSum += diff * diff;
    const bin = Math.max(0, Math.min(255, Math.floor(y * 255)));
    hist[bin] = (hist[bin] ?? 0) + 1;
  }
  const rmsStd = Math.sqrt(varSum / totalPixels);

  const p10Target = totalPixels * 0.1;
  const p95Target = totalPixels * 0.95;
  let cumulative = 0;
  let p10Bin = 0;
  let p95Bin = 255;
  let foundP10 = false;
  for (let bin = 0; bin < 256; bin++) {
    cumulative += hist[bin] ?? 0;
    if (!foundP10 && cumulative >= p10Target) {
      p10Bin = bin;
      foundP10 = true;
    }
    if (cumulative >= p95Target) {
      p95Bin = bin;
      break;
    }
  }
  const dynamicRange = Math.max(0, (p95Bin - p10Bin) / 255.0);
  const luminanceContrast = Number(
    Math.min(100, Math.max(12, rmsStd * 245 + dynamicRange * 52)).toFixed(1),
  );

  let sobelSum = 0;
  let edgePixels = 0;
  let flatLaplacianSum = 0;
  let flatCount = 0;
  const interiorCount = Math.max(1, (width - 2) * (height - 2));

  for (let y = 1; y < height - 1; y++) {
    const row = y * width;
    const rowAbove = (y - 1) * width;
    const rowBelow = (y + 1) * width;
    for (let x = 1; x < width - 1; x++) {
      const tl = luma[rowAbove + x - 1] ?? 0;
      const tc = luma[rowAbove + x] ?? 0;
      const tr = luma[rowAbove + x + 1] ?? 0;
      const ml = luma[row + x - 1] ?? 0;
      const mc = luma[row + x] ?? 0;
      const mr = luma[row + x + 1] ?? 0;
      const bl = luma[rowBelow + x - 1] ?? 0;
      const bc = luma[rowBelow + x] ?? 0;
      const br = luma[rowBelow + x + 1] ?? 0;

      const gx = -tl + tr - 2 * ml + 2 * mr - bl + br;
      const gy = -tl - 2 * tc - tr + bl + 2 * bc + br;
      const mag = Math.sqrt(gx * gx + gy * gy);
      sobelSum += mag;
      if (mag > 0.14) {
        edgePixels++;
      } else if (mag < 0.045) {
        const lap = Math.abs(4 * mc - tc - bc - ml - mr);
        flatLaplacianSum += lap;
        flatCount++;
      }
    }
  }

  const meanSobel = sobelSum / interiorCount;
  const edgeDensity = edgePixels / interiorCount;
  const edgeSharpness = Number(
    Math.min(99, Math.max(18, meanSobel * 420 + edgeDensity * 260 + 38)).toFixed(1),
  );

  const rawFlatNoise = flatCount > 0 ? (flatLaplacianSum / flatCount) * 850 : 6.5;
  const noiseFloor = Number(Math.min(65, Math.max(4.5, rawFlatNoise + 4.5)).toFixed(1));

  return {
    edgeSharpness,
    luminanceContrast,
    noiseFloor,
    meanLuminance: Number(meanLuma.toFixed(4)),
  };
}

/**
 * Synthesizes a 5-axis `VisionRubricScores` struct from measured DOM layout invariants and pixel signals.
 */
export function evaluateGuiSurfaceRubric(
  dom: GuiDomAuditMetrics,
  pixelSignals: { edgeSharpness: number; luminanceContrast: number; noiseFloor: number },
): VisionRubricScores {
  // 1. technicalQuality: pixel sharpness, low noise floor, zero overflow, zero axe violations, zero broken icons
  const totalOverflowPx = dom.outerOverflowPx + (dom.scrollHostOverflowPx ?? 0);
  const overflowPenalty = Math.min(0.35, totalOverflowPx * 0.02);
  const axePenalty = Math.min(0.3, dom.axeViolations * 0.1);
  const clippedPenalty = Math.min(0.25, dom.clippedInteractiveCount * 0.05);
  const iconPenalty = Math.min(0.2, (dom.iconIssueCount ?? 0) * 0.06);
  const signalQuality = Math.min(
    1,
    0.4 +
      (pixelSignals.edgeSharpness / 100) * 0.35 +
      (pixelSignals.luminanceContrast / 100) * 0.25,
  );
  const technicalQuality = Number(
    Math.max(
      0.2,
      Math.min(0.99, signalQuality - overflowPenalty - axePenalty - clippedPenalty - iconPenalty),
    ).toFixed(4),
  );

  // 2. promptAdherence: dock panel completeness, unified knobs, landmark liveness
  const missingDockPenalty = dom.missingDockPanelIds.length * 0.12;
  const unlitLandmarkPenalty = dom.unlitLandmarkCount * 0.055;
  const promptAdherence = Number(
    Math.max(0.2, Math.min(0.99, 0.96 - missingDockPenalty - unlitLandmarkPenalty)).toFixed(4),
  );

  // 3. anatomicalAccuracy: zero raw emojis (`🧍`), zero overlapping controls, consistent control heights
  const emojiPenalty = dom.rawEmojiCount * 0.06;
  const overlapPenalty = dom.overlappingControlPairs * 0.08;
  const heightSpreadPenalty = Math.max(0, (dom.controlHeightSpreadPx - 4) * 0.015);
  const anatomicalAccuracy = Number(
    Math.max(0.2, Math.min(0.98, 0.96 - emojiPenalty - overlapPenalty - heightSpreadPenalty)).toFixed(4),
  );

  // 4. compositionQuality: balanced viewport coverage, zero unstyled native selects, clean toolbar rhythm
  const selectPenalty = dom.unstyledNativeSelectCount * 0.09;
  const coverageBonus =
    dom.viewportCoverageRatio >= 0.45 && dom.viewportCoverageRatio <= 0.92 ? 0.04 : -0.08;
  const compositionQuality = Number(
    Math.max(0.2, Math.min(0.98, 0.92 + coverageBonus - selectPenalty - overflowPenalty)).toFixed(4),
  );

  // 5. styleConsistency: VS Code theme cohesion, unified control chrome, zero light-theme contrast leaks, low noise floor
  const noisePenalty = Math.max(0, (pixelSignals.noiseFloor - 12) * 0.006);
  const contrastLeakPenalty = Math.min(0.3, (dom.lightThemeContrastLeakCount ?? 0) * 0.05);
  const styleConsistency = Number(
    Math.max(
      0.2,
      Math.min(
        0.98,
        0.95 - selectPenalty - emojiPenalty * 0.5 - noisePenalty - contrastLeakPenalty,
      ),
    ).toFixed(4),
  );

  return {
    technicalQuality,
    promptAdherence,
    anatomicalAccuracy,
    compositionQuality,
    styleConsistency,
  };
}

/**
 * Universal, zero-dependency Before/After Computer Vision Flywheel runner (`@ngi/platform-ssot`).
 */
export function runUniversalCvFlywheelPass(
  input: UniversalCvFlywheelInput,
): UniversalCvFlywheelVerdict {
  const beforeRubric =
    input.beforeMetrics.vlmRubric ?? makeRubricFromScalar(input.beforeMetrics.rubricScore);
  const afterRubric =
    input.afterMetrics.vlmRubric ?? makeRubricFromScalar(input.afterMetrics.rubricScore);

  const report = evaluateBeforeAfterCvFlywheel({
    beforeCv: {
      edgeSharpness: input.beforeMetrics.edgeSharpness,
      luminanceContrast: input.beforeMetrics.luminanceContrast,
      noiseFloor: input.beforeMetrics.noiseFloor,
      vlmRubric: beforeRubric,
    },
    afterCv: {
      edgeSharpness: input.afterMetrics.edgeSharpness,
      luminanceContrast: input.afterMetrics.luminanceContrast,
      noiseFloor: input.afterMetrics.noiseFloor,
      vlmRubric: afterRubric,
    },
    afterGrounded: input.afterGrounded ?? true,
  });

  const nextBootstrapDirectives: string[] = [];
  if (afterRubric.anatomicalAccuracy < 0.85) {
    nextBootstrapDirectives.push(
      'Eliminate raw emoji tokens and align toolbar control heights to uniform 28px/32px rhythm',
    );
  }
  if (afterRubric.compositionQuality < 0.85) {
    nextBootstrapDirectives.push(
      'Replace unstyled native <select> elements with unified custom popover selectors',
    );
  }
  if (afterRubric.promptAdherence < 0.88) {
    nextBootstrapDirectives.push(
      'Register missing dock panels (terminal, urbs) and unfreeze embedded Urbs landmark telemetry',
    );
  }
  if (input.afterMetrics.edgeSharpness < 75) {
    nextBootstrapDirectives.push(
      'Sharpen canvas/SVG stroke contrast and crisp 1px dockview border hairlines',
    );
  }
  if (nextBootstrapDirectives.length === 0) {
    nextBootstrapDirectives.push(
      'Flywheel converged: promote After frame & verified DOM layout as new golden baseline',
    );
  }

  return {
    surfaceId: input.surfaceId,
    passNumber: input.passNumber,
    visualBeforeScore: report.visualBeforeScore,
    visualAfterScore: report.visualAfterScore,
    visualDelta: report.visualDelta,
    edgeSharpnessDeltaPct: report.edgeSharpnessDeltaPct,
    contrastDeltaPct: report.contrastDeltaPct,
    noiseReductionPct: report.noiseReductionPct,
    estimatedPsnrDb: report.estimatedPsnrDb,
    flywheelConverged: report.flywheelConverged,
    promoteToGoldenBaseline: report.flywheelConverged && report.visualAfterScore >= 0.85,
    nextBootstrapDirectives,
  };
}
