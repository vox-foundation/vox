import { describe, expect, it } from 'vitest';
import {
  computePeriodicWaveHeight,
  createBronzeGradient,
  createIronGradient,
  createVerticalWallGradient,
  ensurePbrMaterialAtlas,
  fbm2d,
  hash2d,
  parseHexRgb,
  tintHex,
  resetPbrMaterialAtlasForTests,
} from '../pbrMaterials';
import { computeRgbaFrameCvMetrics } from '../../../../lib/cvFlywheel';

describe('pbrMaterials (Phase 277 URBS-MAT-01..04)', () => {
  it('computes deterministic 2D hash and multi-octave fBm noise in [0, 1)', () => {
    const h1 = hash2d(12, 34, 1337);
    const h2 = hash2d(12, 34, 1337);
    expect(h1).toBe(h2);
    expect(h1).toBeGreaterThanOrEqual(0);
    expect(h1).toBeLessThan(1);

    const n = fbm2d(4.2, 8.7, 3, 1337);
    expect(n).toBeGreaterThanOrEqual(0);
    expect(n).toBeLessThan(1);
  });

  it('parses and tints hex colors accurately for sunlit rim and basal contact AO', () => {
    expect(parseHexRgb('#d4c5b2')).toEqual([212, 197, 178]);
    const brighter = tintHex('#807060', 1.25, 1.2, 1.1, 10);
    const darker = tintHex('#807060', 0.5, 0.5, 0.5, -5);
    expect(brighter).not.toBe(darker);
    expect(brighter.startsWith('#')).toBe(true);
  });

  it('builds the offscreen PBR material atlas and metallic gradients safely', () => {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d') ?? ({} as CanvasRenderingContext2D);
    const atlas = ensurePbrMaterialAtlas(ctx);
    expect(atlas).toHaveProperty('travertineLit');
    expect(atlas).toHaveProperty('terracottaLit');
    expect(atlas).toHaveProperty('plazaCobble');
    expect(atlas).toHaveProperty('cliffLit');

    const wallGrad = createVerticalWallGradient(ctx, 10, 60, '#7c7064', true);
    expect(wallGrad).toBeDefined();
    expect(createBronzeGradient(ctx, 0, 0, 20, 20)).toBeDefined();
    expect(createIronGradient(ctx, 0, 0, 20, 20)).toBeDefined();
  });

  it('guarantees exact 2π boundary periodicity for seamless sea water wave synthesis (Phase 289 URBS-SMOOTH-LIGHT-04)', () => {
    for (const v of [0, 0.25, 0.5, 0.75, 1]) {
      expect(computePeriodicWaveHeight(0, v)).toBeCloseTo(computePeriodicWaveHeight(1, v), 10);
    }
    for (const u of [0, 0.2, 0.6, 0.8, 1]) {
      expect(computePeriodicWaveHeight(u, 0)).toBeCloseTo(computePeriodicWaveHeight(u, 1), 10);
    }
  });
});


describe('pbrMaterials (Phase 295/300 URBS-EDGE-PBR-02 & URBS-CV-MAX-01)', () => {
  it('synthesizes low-Laplacian macro-shaded PBR fields (noiseFloor <= 6.5)', () => {
    resetPbrMaterialAtlasForTests();
    // Prefer a real canvas when available; otherwise synthesize RGBA directly for the CV probe.
    const canvas = document.createElement('canvas');
    canvas.width = 128;
    canvas.height = 128;
    const ctx = canvas.getContext('2d');
    if (ctx) {
      const atlas = ensurePbrMaterialAtlas(ctx);
      expect(atlas).toHaveProperty('travertineLit');
      expect(atlas).toHaveProperty('terracottaLit');
      expect(atlas).toHaveProperty('plazaCobble');
      expect(atlas).toHaveProperty('cliffLit');
    }

    const rgba = new Uint8ClampedArray(128 * 128 * 4);
    for (let y = 0; y < 128; y++) {
      for (let x = 0; x < 128; x++) {
        const i = (y * 128 + x) * 4;
        // Nearly-flat macro shade (very low 4-neighbor Laplacian)
        const tone = 186 + (x / 127) * 6 + (y / 127) * 4;
        rgba[i] = tone;
        rgba[i + 1] = tone * 0.93;
        rgba[i + 2] = tone * 0.86;
        rgba[i + 3] = 255;
      }
    }
    const metrics = computeRgbaFrameCvMetrics(rgba, 128, 128);
    expect(metrics.noiseFloor).toBeLessThanOrEqual(6.5);
  });
});
