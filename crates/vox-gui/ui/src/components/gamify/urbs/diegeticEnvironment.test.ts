import { describe, expect, it, vi } from 'vitest';
import {
  computeSafePanoramaRect,
  drawDiegeticCrowdsAndGalleys,
  drawDiegeticWatercourses,
} from './diegeticEnvironment';
import { renderInteractiveMonumentBeaconsAndHalo } from '../../fx/PhotorealisticCanvasFx';

describe('diegeticEnvironment', () => {
  it('computes a safe panorama rectangle that preserves full vertical monument & harbor boundaries', () => {
    const rectWide = computeSafePanoramaRect(1440, 800, 1920, 1080);
    expect(rectWide.drawH).toBeLessThanOrEqual(800 * 1.24);
    expect(Math.abs(rectWide.drawY)).toBeLessThanOrEqual(24);
    expect(rectWide.drawW).toBeGreaterThan(1400);

    const rectNarrow = computeSafePanoramaRect(640, 480, 1920, 1080);
    expect(rectNarrow.drawH).toBe(480);
    expect(rectNarrow.drawY).toBe(0);
    expect(rectNarrow.drawW).toBeGreaterThanOrEqual(640);
  });

  it('draws diegetic watercourses and moving street crowds onto a 2D canvas without throwing', () => {
    const mockCtx = {
      save: vi.fn(),
      restore: vi.fn(),
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      quadraticCurveTo: vi.fn(),
      arc: vi.fn(),
      ellipse: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      fillRect: vi.fn(),
      setLineDash: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      lineCap: 'round',
      lineJoin: 'round',
      lineDashOffset: 0,
      globalAlpha: 1,
    } as unknown as CanvasRenderingContext2D;

    const rect = { drawX: 0, drawY: 0, drawW: 1280, drawH: 720 };
    drawDiegeticWatercourses(mockCtx, rect, 1500, 3, 0);
    drawDiegeticCrowdsAndGalleys(mockCtx, rect, 1500, 3);

    expect(mockCtx.quadraticCurveTo).toHaveBeenCalled();
    expect(mockCtx.arc).toHaveBeenCalled();
    expect(mockCtx.ellipse).toHaveBeenCalled();
    // Strict invariant: zero dashed/dotted lines on painted watercourses or streets
    expect(mockCtx.setLineDash).not.toHaveBeenCalled();
  });

  it('renders interactive monument affordance beacons and smooth mouseover architectural halos without dashed lines', () => {
    const mockCtx = {
      save: vi.fn(),
      restore: vi.fn(),
      beginPath: vi.fn(),
      arc: vi.fn(),
      ellipse: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      setLineDash: vi.fn(),
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      globalCompositeOperation: 'source-over',
    } as unknown as CanvasRenderingContext2D;

    renderInteractiveMonumentBeaconsAndHalo(
      mockCtx,
      { x: 0, y: 0, width: 1280, height: 720 },
      [
        {
          id: 'monument-compiler',
          footprintUv: [0.288, 0.138],
          plinthUv: [0.292, 0.218],
          radiusUv: [0.095, 0.075],
          activityState: 'active',
          isHovered: true,
        },
        {
          id: 'monument-mens',
          footprintUv: [0.865, 0.605],
          plinthUv: [0.862, 0.692],
          radiusUv: [0.088, 0.072],
          activityState: 'ready',
          isHovered: false,
        },
      ],
      2400,
    );

    expect(mockCtx.ellipse).toHaveBeenCalled();
    expect(mockCtx.stroke).toHaveBeenCalled();
    expect(mockCtx.setLineDash).not.toHaveBeenCalled();
  });
});
