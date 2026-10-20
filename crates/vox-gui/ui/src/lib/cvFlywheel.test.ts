import { describe, expect, it } from 'vitest';
import {
  PLATFORM_SSOT_SHA256,
  PLATFORM_SSOT_VERSION,
  computeRgbaFrameCvMetrics,
  evaluateGuiSurfaceRubric,
  evaluateWcagContrastRatio,
  runUniversalCvFlywheelPass,
} from './cvFlywheel';

describe('cvFlywheel (@ngi/platform-ssot adapter)', () => {
  it('exposes the pinned @ngi/platform-ssot version and SHA-256', () => {
    expect(PLATFORM_SSOT_VERSION).toBe('2026.10.19-v3.5');
    expect(PLATFORM_SSOT_SHA256).toHaveLength(64);
  });

  it('computes WCAG 2.2 relative luminance contrast ratio accurately', () => {
    const res = evaluateWcagContrastRatio([250, 250, 250], [12, 12, 16]);
    expect(res.contrastRatio).toBeGreaterThan(15);
    expect(res.passesAA).toBe(true);
    expect(res.passesAAA).toBe(true);
  });

  it('computes Sobel edge sharpness, luminance contrast, and Laplacian noise floor on RGBA buffers', () => {
    const w = 16;
    const h = 16;
    const rgba = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const idx = (y * w + x) * 4;
        const val = x >= 8 ? 220 : 18;
        rgba[idx] = val;
        rgba[idx + 1] = val;
        rgba[idx + 2] = val;
        rgba[idx + 3] = 255;
      }
    }
    const m = computeRgbaFrameCvMetrics(rgba, w, h);
    expect(m.edgeSharpness).toBeGreaterThan(20);
    expect(m.luminanceContrast).toBeGreaterThan(40);
    expect(m.noiseFloor).toBeLessThanOrEqual(5);
  });

  it('evaluates BEFORE -> AFTER convergence through runUniversalCvFlywheelPass', () => {
    const beforeRubric = evaluateGuiSurfaceRubric(
      {
        surfaceId: 'loquela-knobs',
        outerOverflowPx: 0,
        clippedInteractiveCount: 1,
        overlappingControlPairs: 0,
        unstyledNativeSelectCount: 2,
        controlHeightSpreadPx: 6,
        rawEmojiCount: 1,
        unlitLandmarkCount: 1,
        missingDockPanelIds: ['terminal', 'urbs'],
        axeViolations: 1,
        viewportCoverageRatio: 0.78,
      },
      { edgeSharpness: 88, luminanceContrast: 14.5, noiseFloor: 5.4 },
    );

    const afterRubric = evaluateGuiSurfaceRubric(
      {
        surfaceId: 'loquela-knobs',
        outerOverflowPx: 0,
        clippedInteractiveCount: 0,
        overlappingControlPairs: 0,
        unstyledNativeSelectCount: 0,
        controlHeightSpreadPx: 0,
        rawEmojiCount: 0,
        unlitLandmarkCount: 0,
        missingDockPanelIds: [],
        axeViolations: 0,
        viewportCoverageRatio: 0.84,
      },
      { edgeSharpness: 94, luminanceContrast: 15.8, noiseFloor: 5.1 },
    );

    const verdict = runUniversalCvFlywheelPass({
      surfaceId: 'loquela-knobs',
      passNumber: 1,
      rawPrompt: 'Unify Loquela knobs and dock panels',
      beforeMetrics: {
        edgeSharpness: 88,
        luminanceContrast: 14.5,
        noiseFloor: 5.4,
        rubricScore: 0.83,
        vlmRubric: beforeRubric,
      },
      afterMetrics: {
        edgeSharpness: 94,
        luminanceContrast: 15.8,
        noiseFloor: 5.1,
        rubricScore: 0.96,
        vlmRubric: afterRubric,
      },
      afterGrounded: true,
    });

    expect(verdict.flywheelConverged).toBe(true);
    expect(verdict.visualAfterScore).toBeGreaterThan(verdict.visualBeforeScore);
    expect(verdict.estimatedPsnrDb).toBeGreaterThan(35);
  });
});
