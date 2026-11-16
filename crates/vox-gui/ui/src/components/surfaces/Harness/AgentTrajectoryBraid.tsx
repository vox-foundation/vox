import React, { useEffect, useRef, useState } from 'react';
import type { AgentTrajectoryRow } from './riverTypes';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';

export interface AgentTrajectoryBraidProps {
  trajectories: AgentTrajectoryRow[];
}

function buildSmoothBraidPoints(slices: number[], width: number, height: number): Array<{ x: number; y: number }> {
  const padX = 6;
  const padY = 10;
  const usableW = Math.max(40, width - padX * 2);
  const usableH = Math.max(16, height - padY * 2);
  const stepX = slices.length > 1 ? usableW / (slices.length - 1) : usableW;

  return slices.map((laneIdx, i) => ({
    x: padX + i * stepX,
    y: padY + (Math.max(0, Math.min(3, laneIdx)) / 3) * usableH,
  }));
}

function buildSmoothBraidPath(slices: number[], width: number, height: number): string {
  const points = buildSmoothBraidPoints(slices, width, height);
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

export function AgentTrajectoryBraid({ trajectories }: AgentTrajectoryBraidProps) {
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const [svgW, setSvgW] = useState(720);
  const svgH = 76;

  useEffect(() => {
    const el = wrapRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const sync = () => {
      const nextW = Math.max(240, Math.round(el.clientWidth));
      setSvgW(nextW);
    };
    sync();
    const ro = new ResizeObserver(sync);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

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
        <div className="w-48 shrink-0 space-y-1">
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

        <div ref={wrapRef} className="min-w-0 flex-1">
          <svg
            data-cv-chart="true"
            viewBox={`0 0 ${svgW} ${svgH}`}
            preserveAspectRatio="none"
            className="h-16 xl:h-20 w-full block overflow-hidden rounded border border-[#D4AF37]/30 bg-bg-elevated shadow-inner"
            role="img"
            aria-label="Agent role trajectory braid"
          >
            <defs>
              <filter id="braid-spline-glow" x="-5%" y="-20%" width="110%" height="140%">
                <feDropShadow dx="0" dy="1" stdDeviation="1.8" floodColor="#D4AF37" floodOpacity="0.32" />
              </filter>
            </defs>
            {/* Horizontal role lane guides + 8 vertical slice hairlines */}
            {[0, 1, 2, 3].map((lane) => {
              const y = 10 + (lane / 3) * (svgH - 20);
              return (
                <line
                  key={`lane-${lane}`}
                  x1={6}
                  y1={y}
                  x2={svgW - 6}
                  y2={y}
                  stroke="rgba(197, 155, 39, 0.14)"
                  strokeWidth={1}
                />
              );
            })}
            {[0, 1, 2, 3, 4, 5, 6, 7].map((slice) => {
              const x = 6 + (slice / 7) * Math.max(40, svgW - 12);
              return (
                <line
                  key={`slice-${slice}`}
                  x1={x}
                  y1={6}
                  x2={x}
                  y2={svgH - 6}
                  stroke="rgba(197, 155, 39, 0.12)"
                  strokeWidth={1}
                />
              );
            })}
            {trajectories.slice(0, 6).map((row, idx) => {
              const color = LANE_COLORS[idx % LANE_COLORS.length]!;
              const pts = buildSmoothBraidPoints(row.slices, svgW, svgH);
              const endPt = pts[pts.length - 1];
              return (
                <g key={row.agentId}>
                  <path
                    d={buildSmoothBraidPath(row.slices, svgW, svgH)}
                    fill="none"
                    stroke={color}
                    strokeWidth={idx === 0 ? 2.4 : 1.75}
                    strokeOpacity={0.92}
                    vectorEffect="non-scaling-stroke"
                    filter={idx === 0 ? 'url(#braid-spline-glow)' : undefined}
                  />
                  {endPt && (
                    <circle
                      cx={endPt.x}
                      cy={endPt.y}
                      r={idx === 0 ? 3.5 : 2.6}
                      fill={color}
                      stroke="#FAF6EE"
                      strokeWidth={1}
                    />
                  )}
                </g>
              );
            })}
          </svg>
        </div>
      </div>
    </div>
  );
}
