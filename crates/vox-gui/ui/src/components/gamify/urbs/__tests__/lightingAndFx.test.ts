import { describe, expect, it, vi } from 'vitest';
import {
  drawAdditiveLightPools,
  drawCinematicPostProcess,
  drawElevationAtmosphericMist,
  drawForgeAndFireEmbers,
  drawMasonChiselSparks,
  drawProjectedIsoCastShadow,
  drawVolumetricGodRays,
  drawVolumetricSmokePlumes,
  drawWaterCausticsAndSpray,
} from '../lightingAndFx';
import type { TownLayout } from '../types';

function makeMockCtx(): CanvasRenderingContext2D {
  return {
    save: vi.fn(),
    restore: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    closePath: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    arc: vi.fn(),
    ellipse: vi.fn(),
    fillRect: vi.fn(),
    translate: vi.fn(),
    rotate: vi.fn(),
    scale: vi.fn(),
    createRadialGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    fillStyle: '',
    strokeStyle: '',
  } as unknown as CanvasRenderingContext2D;
}

describe('lightingAndFx (Phases 280 & 281 URBS-LIGHT-01..04, URBS-FX-01..04)', () => {
  const sampleLayout: TownLayout = {
    grid: { w: 20, h: 20 },
    districts: [
      {
        name: 'vox-compiler',
        x0: 2,
        y0: 2,
        x1: 8,
        y1: 8,
        landmark: true,
        layer: 2,
        buildings: [{ path: 'crates/vox-compiler/src/parser.rs', x: 4, y: 4, tier: 2, isTouched: true }],
      },
    ],
    byPath: {},
    landmarks: {
      aqvae: { x: 1, y: 1 },
      castrum: { x: 18, y: 2 },
      portus: { x: 2, y: 18 },
      gate: { x: 10, y: 18 },
    },
  };

  it('projects multi-layer directional twilight cast shadows', () => {
    const ctx = makeMockCtx();
    drawProjectedIsoCastShadow(
      ctx,
      (px, py, u, v, z = 0) => [px + (u - v) * 32, py + (u + v) * 16 - z],
      200,
      200,
      -0.4,
      -0.4,
      0,
      0.4,
      0.4,
      24,
    );
    expect(ctx.fill).toHaveBeenCalledTimes(3);
  });

  it('renders elevation mist, volumetric god-rays, additive GI pools, and cinematic vignette', () => {
    const ctx = makeMockCtx();
    const worldPxFn = (_l: TownLayout, x: number, y: number) => ({ px: x * 32, py: y * 16 });
    const elevFn = () => 36;

    drawElevationAtmosphericMist(ctx, sampleLayout, worldPxFn, elevFn);
    drawVolumetricGodRays(ctx, sampleLayout, worldPxFn, elevFn);
    drawAdditiveLightPools(
      ctx,
      sampleLayout,
      { 'crates/vox-compiler/src/parser.rs': { warnings: 0, errors: 2 } },
      {},
      worldPxFn,
      elevFn,
    );
    drawCinematicPostProcess(ctx, 1280, 720);

    expect(ctx.createRadialGradient).toHaveBeenCalled();
    expect(ctx.translate).toHaveBeenCalled();
    expect(ctx.rotate).toHaveBeenCalled();
  });

  it('renders deterministic particle FX (embers, volumetric smoke, waterfall spray, chisel sparks)', () => {
    const ctx = makeMockCtx();
    drawForgeAndFireEmbers(ctx, 300, 200, 1500, 1, 8);
    drawVolumetricSmokePlumes(ctx, 300, 200, 1500, 1, true);
    drawWaterCausticsAndSpray(ctx, 150, 400, 1500, 2);
    drawMasonChiselSparks(ctx, 250, 220, 1500, 3);
    expect(ctx.arc).toHaveBeenCalled();
  });
});
