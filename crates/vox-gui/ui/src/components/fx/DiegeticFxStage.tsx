import React, { useEffect, useRef } from 'react';
import {
  renderAtmosphericTelemetryPulse,
  renderDiegeticTorchAndBrazierPools,
  renderPerspectiveIntegratedProcession,
  renderPhotorealWaterCausticsAndMist,
  renderSunlitGodRays,
  type FxMistPoolSpec,
  type FxProcessionRouteSpec,
  type FxWaterChannelSpec,
} from './PhotorealisticCanvasFx';

export type DiegeticFxPreset =
  | 'aqueduct-dag'
  | 'voice-harmonics'
  | 'river-cascade'
  | 'sunlit-colonnade';

export interface DiegeticFxStageProps {
  preset: DiegeticFxPreset;
  activeBoost?: number;
  intensity?: number;
  className?: string;
  testId?: string;
}

const AQUEDUCT_DAG_CHANNELS: readonly FxWaterChannelSpec[] = [
  {
    uvPoints: [
      [0.04, 0.46],
      [0.32, 0.43],
      [0.64, 0.39],
      [0.95, 0.36],
    ],
    widthPx: 9,
    flowSpeed: 0.42,
    tint: 'turquoise',
  },
  {
    uvPoints: [
      [0.18, 0.68],
      [0.48, 0.64],
      [0.82, 0.59],
    ],
    widthPx: 11,
    flowSpeed: 0.34,
    tint: 'gold',
  },
];

const AQUEDUCT_DAG_POOLS: readonly FxMistPoolSpec[] = [
  { uv: [0.31, 0.56], radiusPx: 26, plumeHeightPx: 34 },
  { uv: [0.76, 0.58], radiusPx: 32, plumeHeightPx: 40 },
];

const AQUEDUCT_DAG_WALKERS: readonly FxProcessionRouteSpec[] = [
  {
    uvPoints: [
      [0.08, 0.38],
      [0.45, 0.35],
      [0.88, 0.32],
    ],
    count: 5,
    caste: 'legionary',
    speed: 0.048,
  },
];

const RIVER_CASCADE_CHANNELS: readonly FxWaterChannelSpec[] = [
  {
    uvPoints: [
      [0.03, 0.54],
      [0.36, 0.53],
      [0.72, 0.52],
      [0.97, 0.52],
    ],
    widthPx: 14,
    flowSpeed: 0.55,
    tint: 'turquoise',
  },
  {
    uvPoints: [
      [0.08, 0.72],
      [0.48, 0.71],
      [0.92, 0.70],
    ],
    widthPx: 12,
    flowSpeed: 0.44,
    tint: 'gold',
  },
];

const RIVER_CASCADE_POOLS: readonly FxMistPoolSpec[] = [
  { uv: [0.26, 0.63], radiusPx: 28, plumeHeightPx: 36 },
  { uv: [0.58, 0.65], radiusPx: 32, plumeHeightPx: 42 },
  { uv: [0.84, 0.64], radiusPx: 26, plumeHeightPx: 34 },
];

export function DiegeticFxStage({
  preset,
  activeBoost = 1,
  intensity = 1,
  className = '',
  testId = 'diegetic-fx-stage',
}: DiegeticFxStageProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const reducedMotion =
      typeof window !== 'undefined' &&
      typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    let width = 0;
    let height = 0;
    const syncSize = () => {
      const parent = canvas.parentElement;
      if (!parent) return;
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      width = Math.max(1, parent.clientWidth);
      height = Math.max(1, parent.clientHeight);
      if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) {
        canvas.width = Math.round(width * dpr);
        canvas.height = Math.round(height * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    syncSize();
    const ro =
      typeof ResizeObserver !== 'undefined' && canvas.parentElement
        ? new ResizeObserver(() => syncSize())
        : null;
    if (ro && canvas.parentElement) {
      ro.observe(canvas.parentElement);
    }

    let rafId = 0;
    const drawFrame = (nowMs: number) => {
      if (width < 8 || height < 8) {
        syncSize();
      }
      ctx.clearRect(0, 0, width, height);
      const bounds = { x: 0, y: 0, width, height };

      if (preset === 'aqueduct-dag') {
        renderSunlitGodRays(ctx, bounds, nowMs, 0.95 * intensity);
        renderPhotorealWaterCausticsAndMist(
          ctx,
          bounds,
          AQUEDUCT_DAG_CHANNELS,
          AQUEDUCT_DAG_POOLS,
          nowMs,
          intensity,
        );
        renderPerspectiveIntegratedProcession(
          ctx,
          bounds,
          AQUEDUCT_DAG_WALKERS,
          nowMs,
          activeBoost,
        );
        renderDiegeticTorchAndBrazierPools(
          ctx,
          bounds,
          [
            [0.22, 0.34],
            [0.52, 0.31],
            [0.79, 0.33],
          ],
          nowMs,
          activeBoost,
        );
      } else if (preset === 'river-cascade') {
        renderSunlitGodRays(ctx, bounds, nowMs, 0.9 * intensity);
        renderPhotorealWaterCausticsAndMist(
          ctx,
          bounds,
          RIVER_CASCADE_CHANNELS,
          RIVER_CASCADE_POOLS,
          nowMs,
          intensity,
        );
        renderDiegeticTorchAndBrazierPools(
          ctx,
          bounds,
          [
            [0.16, 0.42],
            [0.5, 0.4],
            [0.84, 0.42],
          ],
          nowMs,
          activeBoost,
        );
      } else if (preset === 'voice-harmonics') {
        renderSunlitGodRays(ctx, bounds, nowMs, 0.75 * intensity);
        const t = nowMs * 0.001;
        for (let band = 0; band < 3; band++) {
          const amp = (10 + band * 6) * Math.min(1.4, activeBoost);
          const baseY = height * (0.42 + band * 0.14);
          ctx.beginPath();
          for (let x = 0; x <= width; x += 8) {
            const u = x / Math.max(1, width);
            const env = Math.sin(u * Math.PI);
            const y =
              baseY +
              Math.sin(u * 10 + t * (2.6 + band * 0.7) + band) * amp * env +
              Math.cos(u * 21 - t * 1.8) * (amp * 0.35) * env;
            if (x === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
          }
          ctx.strokeStyle =
            band === 0
              ? 'rgba(197, 155, 39, 0.52)'
              : band === 1
                ? 'rgba(27, 107, 58, 0.38)'
                : 'rgba(180, 120, 40, 0.34)';
          ctx.lineWidth = band === 0 ? 2.2 : 1.5;
          ctx.stroke();
        }
        renderAtmosphericTelemetryPulse(
          ctx,
          bounds,
          [
            [
              [0.05, 0.48],
              [0.5, 0.44],
              [0.95, 0.48],
            ],
            [
              [0.05, 0.64],
              [0.5, 0.68],
              [0.95, 0.64],
            ],
          ],
          nowMs,
          '#D4AF37',
        );
      } else {
        renderSunlitGodRays(ctx, bounds, nowMs, 0.85 * intensity);
      }

      if (!reducedMotion) {
        rafId = requestAnimationFrame(drawFrame);
      }
    };

    if (reducedMotion) {
      drawFrame(1200);
    } else {
      rafId = requestAnimationFrame(drawFrame);
    }

    return () => {
      if (rafId) cancelAnimationFrame(rafId);
      ro?.disconnect();
    };
  }, [preset, activeBoost, intensity]);

  return (
    <canvas
      ref={canvasRef}
      data-testid={testId}
      aria-hidden="true"
      className={`pointer-events-none absolute inset-0 h-full w-full ${className}`}
    />
  );
}
