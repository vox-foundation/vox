import React from 'react';
import type { AgentTrajectoryRow } from './riverTypes';

export interface AgentTrajectoryBraidProps {
  trajectories: AgentTrajectoryRow[];
}

function buildSmoothBraidPath(slices: number[], width: number, height: number): string {
  const padX = 16;
  const padY = 12;
  const usableW = width - padX * 2;
  const usableH = height - padY * 2;
  const stepX = slices.length > 1 ? usableW / (slices.length - 1) : usableW;

  const points = slices.map((laneIdx, i) => ({
    x: padX + i * stepX,
    y: padY + (Math.max(0, Math.min(3, laneIdx)) / 3) * usableH,
  }));

  if (points.length === 0) return '';
  let d = `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[i];
    const p1 = points[i + 1];
    const midX = ((p0.x + p1.x) / 2).toFixed(1);
    d += ` C ${midX} ${p0.y.toFixed(1)}, ${midX} ${p1.y.toFixed(1)}, ${p1.x.toFixed(1)} ${p1.y.toFixed(1)}`;
  }
  return d;
}

const STROKE_PALETTE = ['#e4e4e7', '#22d3ee', '#34d399', '#fbbf24', '#a1a1aa'];

export function AgentTrajectoryBraid({ trajectories }: AgentTrajectoryBraidProps) {
  const svgW = 520;
  const svgH = 88;

  return (
    <div
      data-testid="harness-agent-braid"
      className="rounded-lg border border-white/10 bg-[#09090b] p-2.5 font-mono text-[10px]"
    >
      <div className="mb-1.5 flex items-center justify-between text-[9px] uppercase tracking-wider text-text-muted">
        <span>AGENT ROLE TRAJECTORY FLOW — LAST 8 SLICES</span>
        <span>INTERCEPT · FAN-OUT · VERIFY · SANDBOX</span>
      </div>

      <div className="flex items-center gap-3">
        <div className="w-32 shrink-0 space-y-1">
          {trajectories.slice(0, 5).map((row, idx) => (
            <div key={row.agentId} className="flex items-center justify-between gap-1 truncate">
              <span className="truncate text-zinc-200" style={{ color: STROKE_PALETTE[idx % STROKE_PALETTE.length] }}>
                {row.label}
              </span>
              <span className="rounded bg-white/5 px-1 text-[8px] text-text-muted">{row.activeLane}</span>
            </div>
          ))}
        </div>

        <svg
          viewBox={`0 0 ${svgW} ${svgH}`}
          className="h-20 flex-1 overflow-visible rounded bg-black/50 border border-white/5"
          role="img"
          aria-label="Agent role trajectory braid"
        >
          {trajectories.slice(0, 6).map((row, idx) => (
            <path
              key={row.agentId}
              d={buildSmoothBraidPath(row.slices, svgW, svgH)}
              fill="none"
              stroke={STROKE_PALETTE[idx % STROKE_PALETTE.length]}
              strokeWidth={idx === 0 ? 2 : 1.3}
              strokeOpacity={0.85}
            />
          ))}
        </svg>
      </div>
    </div>
  );
}
