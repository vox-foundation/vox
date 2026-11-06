import { describe, expect, it, vi } from 'vitest';
import {
  buildActiveHydraulicConduits,
  computeIsoConduitWaypoints,
  drawAnimatedHydraulicActivityFlows,
  drawChevronWaveParticle,
  drawDiamondIngotParticle,
  drawLaurelSealParticle,
  drawRomanHydraulicInfrastructure,
  drawScreenSpaceHydraulicOverlay,
  drawWarningTriangleParticle,
  samplePolylineAt,
  traceRoundedPolylinePath,
  strokeRoundedPolyline,
  computeConduitActivityLoadScale,
} from '../hydraulicNetwork';
import type { TownLayout } from '../types';
import type { WorldState } from '../worldRenderer';

function makeMockCtx(): CanvasRenderingContext2D {
  return {
    save: vi.fn(),
    restore: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    quadraticCurveTo: vi.fn(),
    closePath: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    arc: vi.fn(),
    ellipse: vi.fn(),
    fillRect: vi.fn(),
    strokeRect: vi.fn(),
    roundRect: vi.fn(),
    fillText: vi.fn(),
    translate: vi.fn(),
    rotate: vi.fn(),
    scale: vi.fn(),
    createRadialGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
    font: '',
    textAlign: 'left',
    textBaseline: 'middle',
  } as unknown as CanvasRenderingContext2D;
}

describe('hydraulicNetwork (Phases 292–294 URBS-HYDRAULIC-01..04, URBS-CONDUIT-ANIM-01..04, URBS-SEMIOTICS-CV-01..03)', () => {
  const sampleLayout: TownLayout = {
    grid: { w: 24, h: 24 },
    roads: [],
    districts: [
      {
        name: 'vox-compiler',
        x0: 2,
        y0: 2,
        x1: 8,
        y1: 8,
        landmark: true,
        layer: 2,
        deps: ['vox-db'],
        buildings: [
          { path: 'crates/vox-compiler/src/parser.rs', x: 4, y: 4, tier: 2, isTouched: true },
        ],
      },
      {
        name: 'vox-db',
        x0: 10,
        y0: 10,
        x1: 16,
        y1: 16,
        landmark: false,
        layer: 0,
        buildings: [
          { path: 'crates/vox-db/src/lib.rs', x: 12, y: 12, tier: 1 },
        ],
      },
      {
        name: 'vox-gui',
        x0: 10,
        y0: 2,
        x1: 16,
        y1: 8,
        landmark: false,
        layer: 3,
        deps: ['vox-compiler'],
        buildings: [
          { path: 'crates/vox-gui/src/lib.rs', x: 12, y: 4, tier: 3, isTouched: true },
        ],
      },
    ],
    byPath: {},
    landmarks: {
      aqvae: { x: 1, y: 1 },
      castrum: { x: 20, y: 2 },
      portus: { x: 2, y: 20 },
      gate: { x: 12, y: 20 },
    },
  };

  const sampleState: WorldState = {
    layout: sampleLayout,
    buildings: {
      'crates/vox-compiler/src/parser.rs': { warnings: 1, errors: 2 },
    },
    agentTasks: {
      'AEDILIS-01': { filePath: 'crates/vox-gui/src/lib.rs', action: 'editing' },
    },
    harness: {
      ci: { runners: [{ name: 'legio-1', busy: true }], queued: 0 },
      vcs: { branches: [{ name: 'main', track: 'origin/main', isHead: true }], prs: [] },
      queueLen: 3,
      mcp: [{ name: 'vox-mcp', ok: true }],
    },
    selectedDistrict: 'vox-compiler',
  };

  const worldPxFn = (_l: TownLayout, x: number, y: number) => ({ px: x * 32, py: y * 16 });
  const elevFn = () => 24;

  it('computes isometric-aligned 3-point conduit waypoints and interpolates polyline samples accurately', () => {
    const wps = computeIsoConduitWaypoints([100, 100], [300, 220]);
    expect(wps.length).toBe(3);
    expect(wps[0]).toEqual([100, 100]);
    expect(wps[2]).toEqual([300, 220]);

    const start = samplePolylineAt(wps, 0);
    const mid = samplePolylineAt(wps, 0.5);
    const end = samplePolylineAt(wps, 1);
    expect(start.x).toBeCloseTo(100, 5);
    expect(end.x).toBeCloseTo(300, 5);
    expect(mid.x).toBeGreaterThan(100);
    expect(mid.x).toBeLessThan(300);
  });

  it('builds all 4 hydraulic/sewer conduit classes (supply, work_in_progress, error_fault, verified_ci)', () => {
    const conduits = buildActiveHydraulicConduits(sampleState, worldPxFn, elevFn);
    const kinds = new Set(conduits.map((c) => c.kind));
    expect(kinds.has('supply')).toBe(true);
    expect(kinds.has('work_in_progress')).toBe(true);
    expect(kinds.has('error_fault')).toBe(true);
    expect(kinds.has('verified_ci')).toBe(true);
  });

  it('renders physical Roman hydraulic infrastructure (Aquaeductus, Castellum Divisorium, Fistulae & Cloaca Maxima)', () => {
    const ctx = makeMockCtx();
    drawRomanHydraulicInfrastructure(
      ctx,
      sampleLayout,
      sampleState.buildings,
      sampleState.agentTasks,
      worldPxFn,
      elevFn,
    );
    expect(ctx.stroke).toHaveBeenCalled();
    expect(ctx.ellipse).toHaveBeenCalled();
    expect(ctx.arc).toHaveBeenCalled();
  });

  it('animates all 4 shape+color activity particles and renders the screen-space hydraulic legend', () => {
    const ctx = makeMockCtx();
    drawChevronWaveParticle(ctx, 50, 50, 0.2);
    drawDiamondIngotParticle(ctx, 60, 60, -0.3);
    drawWarningTriangleParticle(ctx, 70, 70, 0.5);
    drawLaurelSealParticle(ctx, 80, 80);

    drawAnimatedHydraulicActivityFlows(ctx, sampleState, 2400, worldPxFn, elevFn);
    drawScreenSpaceHydraulicOverlay(
      ctx,
      sampleState,
      (wx, wy) => [wx, wy],
      worldPxFn,
      elevFn,
      'vox-compiler',
    );

    const legendTexts = (ctx.fillText as ReturnType<typeof vi.fn>).mock.calls.map((c) => String(c[0]));
    expect(legendTexts.some((t) => t.includes('AQUA') && t.includes('Supply'))).toBe(true);
    expect(legendTexts.some((t) => t.includes('IGNIS') && t.includes('WIP'))).toBe(true);
    expect(legendTexts.some((t) => t.includes('CLOACA') && t.includes('Fault'))).toBe(true);
    expect(legendTexts.some((t) => t.includes('VICTORIA') && t.includes('CI'))).toBe(true);
  });
});


describe('hydraulicNetwork (Phases 298–300 URBS-PLUMBING-3D & URBS-FLOW-SEMIOTICS)', () => {
  it('traces rounded elbow fillets via strokeRoundedPolyline / quadraticCurveTo', () => {
    const ctx = makeMockCtx();
    const wps: [number, number][] = [
      [10, 10],
      [80, 10],
      [80, 90],
      [140, 90],
    ];
    strokeRoundedPolyline(ctx, wps, 14);
    expect(ctx.quadraticCurveTo).toHaveBeenCalled();
    expect(ctx.moveTo).toHaveBeenCalledWith(10, 10);
  });

  it('scales conduit gauge by live activity intensity', () => {
    const quiet = computeConduitActivityLoadScale({});
    const busy = computeConduitActivityLoadScale({
      mcpOk: 3,
      mcpTotal: 3,
      churnLines: 100,
      errorCount: 4,
      runnerCount: 3,
      activeTaskCount: 4,
    });
    expect(quiet).toBeGreaterThanOrEqual(0.72);
    expect(busy).toBeGreaterThan(quiet);
    expect(busy).toBeLessThanOrEqual(1.85);
  });

  it('renders halo-backed particles with radial glow gradients', () => {
    const ctx = makeMockCtx();
    drawChevronWaveParticle(ctx, 40, 40, 0.1, 1);
    drawDiamondIngotParticle(ctx, 50, 50, 0.2, 1);
    drawWarningTriangleParticle(ctx, 60, 60, 0.3, 1);
    drawLaurelSealParticle(ctx, 70, 70, 1);
    expect(ctx.createRadialGradient).toHaveBeenCalled();
  });
});
