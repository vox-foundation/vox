import React, { useEffect, useRef, useState } from 'react';
import type { AgentTrajectoryRow } from './riverTypes';

export interface AgentTrajectoryBraidProps {
  trajectories: AgentTrajectoryRow[];
}

export const LANE_COLORS = ['#D4AF37', '#10B981', '#F59E0B', '#F43F5E', '#C89D54', '#64748B'];

const SLICE_ROLES = ['INTERCEPT', 'FAN_OUT', 'VERIFY', 'SANDBOX'] as const;

function buildSmoothBraidPoints(
  slices: number[],
  width: number,
  height: number,
): Array<{ x: number; y: number }> {
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

function buildSmoothBraidPathFromPoints(points: Array<{ x: number; y: number }>): string {
  if (points.length === 0) return '';
  let d = `M ${points[0]!.x.toFixed(1)} ${points[0]!.y.toFixed(1)}`;
  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[i]!;
    const p1 = points[i + 1]!;
    const midX = ((p0.x + p1.x) / 2).toFixed(1);
    d += ` C ${midX} ${p0.y.toFixed(1)}, ${midX} ${p1.y.toFixed(1)}, ${p1.x.toFixed(1)} ${p1.y.toFixed(1)}`;
  }
  return d;
}

/**
 * Compact Mode 1 Concilium causal swimlane embed for `HarnessRiverPanel`.
 * Renders W3C `traceparent` slice span bars, `0 tok/s` parked hibernation indicators,
 * and causal spline paths for each active agent trajectory.
 */
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
      data-slot="concilium-trajectory-braid"
      data-testid="harness-agent-braid"
      className="rounded-lg border border-[#D4AF37]/30 bg-[#0B0D12] p-2.5 font-mono text-[10px] text-slate-100 shadow-inner"
    >
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2 border-b border-[#D4AF37]/20 pb-1.5 text-[9px] uppercase tracking-wider text-slate-400">
        <div className="flex items-center gap-2">
          <span
            className="inline-block size-3.5 rounded-t-full border-2 border-[#D4AF37] border-b-0"
            aria-hidden="true"
          />
          <div>
            <div className="font-display text-[11px] font-semibold tracking-wider text-[#F3E5AB] normal-case">
              Concilium Causal Trajectory Swimlanes
            </div>
            <span className="text-[8.5px] tracking-widest text-slate-400">
              AGENT ROLE TRAJECTORY FLOW — LAST 8 SLICES · W3C TRACEPARENT
            </span>
          </div>
        </div>
        <span className="hidden sm:inline font-mono text-[9px] text-[#F3E5AB]">
          INTERCEPT · FAN-OUT · VERIFY · SANDBOX (0 tok/s)
        </span>
      </div>

      <div className="flex flex-col gap-2.5 lg:flex-row lg:items-center">
        <div className="w-full shrink-0 space-y-1.5 lg:w-64">
          {trajectories.slice(0, 6).map((row, idx) => {
            const isIdleSandbox = row.activeLane === 'SANDBOX';
            const traceparent = `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba9020${idx}-01`;
            const rawLast = row.slices[row.slices.length - 1] ?? 0;
            const lastRoleIdx = Math.max(
              0,
              Math.min(SLICE_ROLES.length - 1, Number.isFinite(rawLast) ? rawLast : 0),
            );
            return (
              <div
                key={row.agentId}
                data-slot="concilium-braid-lane"
                title={`traceparent: ${traceparent}`}
                className="flex items-center justify-between gap-1.5 rounded border border-[#D4AF37]/25 bg-slate-950/90 px-2 py-1"
              >
                <span className="flex min-w-0 items-center gap-1.5 truncate font-semibold text-[#F3E5AB]">
                  <span
                    className="inline-block size-2 shrink-0 rounded-full"
                    style={{ backgroundColor: LANE_COLORS[idx % LANE_COLORS.length] }}
                    aria-hidden="true"
                  />
                  <span className="truncate">{row.label}</span>
                </span>
                <div className="flex shrink-0 items-center gap-1">
                  <span className="rounded border border-[#D4AF37]/35 bg-[#D4AF37]/15 px-1 text-[8px] text-[#F3E5AB]">
                    {row.activeLane}
                  </span>
                  <span className="rounded border border-slate-700 bg-slate-900 px-1 text-[8px] text-slate-300">
                    {isIdleSandbox ? '0 tok/s' : SLICE_ROLES[lastRoleIdx]}
                  </span>
                </div>
              </div>
            );
          })}
        </div>

        <div ref={wrapRef} className="min-w-0 flex-1">
          <svg
            data-cv-chart="true"
            viewBox={`0 0 ${svgW} ${svgH}`}
            preserveAspectRatio="none"
            className="block h-16 w-full overflow-hidden rounded border border-[#D4AF37]/30 bg-slate-950 xl:h-20"
            role="img"
            aria-label="Agent role trajectory braid"
          >
            {[0, 1, 2, 3].map((lane) => {
              const y = 10 + (lane / 3) * (svgH - 20);
              return (
                <line
                  key={`lane-${lane}`}
                  x1={6}
                  y1={y}
                  x2={svgW - 6}
                  y2={y}
                  stroke="rgba(212, 175, 55, 0.16)"
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
                    d={buildSmoothBraidPathFromPoints(pts)}
                    fill="none"
                    stroke={color}
                    strokeWidth={idx === 0 ? 2.4 : 1.75}
                    strokeOpacity={0.92}
                    vectorEffect="non-scaling-stroke"
                  />
                  {endPt && (
                    <circle
                      cx={endPt.x}
                      cy={endPt.y}
                      r={idx === 0 ? 3.5 : 2.6}
                      fill={color}
                      stroke="#0B0D12"
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
