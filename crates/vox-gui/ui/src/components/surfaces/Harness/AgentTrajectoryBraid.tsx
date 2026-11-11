import React from 'react';
import type { AgentTrajectoryRow } from './riverTypes';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

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

export const LANE_COLORS = ['#D4AF37', '#8FA339', '#F59E0B', '#E07A5F', '#C89D54', '#B89B4C'];
const STROKE_PALETTE = LANE_COLORS;

export function AgentTrajectoryBraid({ trajectories }: AgentTrajectoryBraidProps) {
  const svgW = 520;
  const svgH = 72;

  return (
    <div
      data-testid="harness-agent-braid"
      className="rounded-lg border border-[#D4AF37]/30 bg-bg-surface shadow-[0_1px_0_rgba(243,229,171,0.1)_inset] p-2 font-mono text-[10px]"
    >
      <RomanTooltip
        title="Parallel Agent Execution Braid"
        purpose="Visualizes real-time and historical execution paths across roles (Intercept, Fan-out, Verify, Sandbox) for each active agent."
        actionHint="Inspect role transitions across active and completed tool trajectories"
      >
        <div className="mb-1.5 flex items-center justify-between border-b border-[#D4AF37]/20 pb-1 text-[9px] uppercase tracking-wider text-text-secondary">
          <div className="flex items-center gap-2">
            <RomanMedallion active size="sm" {...({ icon: 'romanArch', tone: 'gold' } as any)}>
              <span className="inline-block size-3.5 rounded-t-full border-2 border-current border-b-0" aria-hidden="true" />
            </RomanMedallion>
            <div>
              <div className="font-display text-[11px] font-semibold tracking-wider text-text-primary normal-case">
                Parallel Agent Execution Braid
              </div>
              <span className="text-[8.5px] tracking-widest text-text-muted">
                AGENT ROLE TRAJECTORY FLOW — LAST 8 SLICES
              </span>
            </div>
          </div>
          <span className="hidden sm:inline font-mono text-[9px] text-text-muted">
            INTERCEPT · FAN-OUT · VERIFY · SANDBOX
          </span>
        </div>
      </RomanTooltip>

      <div className="flex items-center gap-3">
        <div className="w-44 shrink-0 space-y-1">
          {trajectories.slice(0, 5).map((row, idx) => (
            <RomanTooltip
              key={row.agentId}
              title={`Agent: ${row.label}`}
              purpose={`Current role: ${row.activeLane}. The braided spline maps its last 8 tool execution slices across verification stages.`}
              actionHint={`Active role lane: ${row.activeLane}`}
              className="w-full block"
            >
              <div className="flex items-center justify-between gap-1 truncate rounded px-1 py-0.5 hover:bg-[#D4AF37]/10 transition-colors">
                <span className="flex items-center gap-1.5 truncate font-semibold text-text-primary">
                  <span
                    className="inline-block size-2 shrink-0 rounded-full"
                    style={{ backgroundColor: LANE_COLORS[idx % LANE_COLORS.length] }}
                    aria-hidden="true"
                  />
                  <span className="truncate">{row.label}</span>
                </span>
                <span className="rounded border border-[#D4AF37]/25 bg-[#D4AF37]/10 px-1 text-[8px] text-text-primary">
                  {row.activeLane}
                </span>
              </div>
            </RomanTooltip>
          ))}
        </div>

        <svg
          viewBox={`0 0 ${svgW} ${svgH}`}
          className="h-14 xl:h-18 flex-1 overflow-visible rounded border border-[#D4AF37]/20 bg-bg-elevated shadow-inner"
          role="img"
          aria-label="Agent role trajectory braid"
        >
          {trajectories.slice(0, 6).map((row, idx) => (
            <path
              key={row.agentId}
              d={buildSmoothBraidPath(row.slices, svgW, svgH)}
              fill="none"
              stroke={LANE_COLORS[idx % LANE_COLORS.length]}
              strokeWidth={idx === 0 ? 2 : 1.3}
              strokeOpacity={0.85}
            />
          ))}
        </svg>
      </div>
    </div>
  );
}
