import { describe, it, expect, vi } from 'vitest';
import { computeAlluvialControlPoints, renderRiverFrame } from './riverCanvasRenderer';
import { deriveRiverSnapshot } from './riverMath';

describe('riverCanvasRenderer', () => {
  it('pinches all alluvial strands through the central Singularity chokepoint', () => {
    const pts = computeAlluvialControlPoints(
      {
        id: 's-1',
        sourceSlot: 0.1,
        targetSlot: 0.9,
        intensity: 0.8,
        status: 'verified',
        hashPrefix: 'a1b2c3d',
      },
      600,
      400,
    );

    expect(pts.x0).toBeCloseTo(79.2, 1);
    expect(pts.y0).toBeLessThan(40);
    // Central chokepoint (sx, sy) must sit near the horizontal and vertical center
    expect(pts.sx).toBeGreaterThan(260);
    expect(pts.sx).toBeLessThan(340);
    expect(pts.sy).toBeCloseTo(200, 0);
    expect(pts.x1).toBeCloseTo(520.8, 1);
    expect(pts.y1).toBeGreaterThan(360);
  });

  it('renders strands, central singularity, and sweeping laser bar onto a 2D context without throwing', () => {
    const snap = deriveRiverSnapshot({
      tasks: [{ id: '1', title: 'Task A', status: 'in_progress' }],
      messages: [],
      clutch: 'efficiency',
      risk: 'low',
    });

    const mockCtx = {
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      bezierCurveTo: vi.fn(),
      stroke: vi.fn(),
      fill: vi.fn(),
      arc: vi.fn(),
      fillText: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
      createRadialGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      setLineDash: vi.fn(),
      measureText: vi.fn(() => ({ width: 120 })),
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      font: '',
      textAlign: 'left',
      textBaseline: 'alphabetic',
    } as unknown as CanvasRenderingContext2D;

    renderRiverFrame(mockCtx, 640, 360, snap, 1.25);

    expect(mockCtx.bezierCurveTo).toHaveBeenCalled();
    expect(mockCtx.fillText).toHaveBeenCalledWith(
      expect.stringContaining(snap.activePhaseBanner),
      expect.any(Number),
      expect.any(Number),
    );
  });
});
