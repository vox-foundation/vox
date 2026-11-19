import React, { useMemo } from 'react';
import {
  evaluateNoveltyUtilityGate,
  type WorthinessAxisScore,
} from '../../../api/scientia';
import { Icon } from '../../ui/Icons';

export interface WorthinessRadarPanelProps {
  axes: WorthinessAxisScore[];
  noveltyVerdict?: string | null;
  title?: string;
  compact?: boolean;
  actions?: React.ReactNode;
}

const CENTER = 110;
const MAX_RADIUS = 70;
const RINGS = [0.33, 0.66, 1.0];

function polarPoint(index: number, total: number, ratio: number): [number, number] {
  const angleRad = ((-90 + (index * 360) / total) * Math.PI) / 180;
  const r = MAX_RADIUS * Math.max(0.05, Math.min(1, ratio));
  return [
    Number((CENTER + r * Math.cos(angleRad)).toFixed(1)),
    Number((CENTER + r * Math.sin(angleRad)).toFixed(1)),
  ];
}

function polygonPoints(total: number, ratios: number[]): string {
  return ratios
    .map((ratio, idx) => {
      const [x, y] = polarPoint(idx, total, ratio);
      return `${x},${y}`;
    })
    .join(' ');
}

/**
 * 6-Axis Scientia Worthiness Radar & Two-Stage Novelty x Utility Gate panel.
 * Renders a deterministic SVG spider chart alongside the two-stage discovery gate
 * status pill and per-axis breakdown cards.
 */
export function WorthinessRadarPanel({
  axes,
  noveltyVerdict,
  title = '6-Axis Scientia Worthiness Profile',
  compact = false,
  actions,
}: WorthinessRadarPanelProps) {
  const gate = useMemo(
    () => evaluateNoveltyUtilityGate(axes, noveltyVerdict),
    [axes, noveltyVerdict],
  );

  const totalAxes = Math.max(1, axes.length);
  const dataPolygon = useMemo(
    () => polygonPoints(totalAxes, axes.map((a) => a.score)),
    [axes, totalAxes],
  );

  return (
    <div
      data-testid="scientia-worthiness-meter"
      data-slot="worthiness-radar-panel"
      className="rounded-xl border border-border-subtle bg-bg-surface p-4 space-y-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2.5">
          <span className="font-display text-xs uppercase tracking-[0.16em] text-text-muted">
            {title}
          </span>
          <div
            data-testid="novelty-utility-gate-pill"
            title="Two-stage autonomous promotion gate: Novelty (prior-art distance N) x Utility (empirical claim support & replayability U) = Composite Score (>= 65% required for promotion)"
            className={`inline-flex items-center gap-1.5 rounded-full border bg-bg-elevated px-2.5 py-0.5 font-mono text-[11px] ${
              gate.passed
                ? 'border-(--color-status-pass)/40 text-(--color-status-pass)'
                : 'border-(--color-status-warn)/40 text-(--color-status-warn)'
            }`}
          >
            {gate.passed ? (
              <Icon.check className="size-3 shrink-0" aria-hidden="true" />
            ) : (
              <Icon.alert className="size-3 shrink-0" aria-hidden="true" />
            )}
            <span>{gate.verdictLabel}</span>
            <span className="text-text-muted">·</span>
            <span>
              N {Math.round(gate.noveltyScore * 100)}% x U {Math.round(gate.utilityScore * 100)}% ={' '}
              {Math.round(gate.compositeScore * 100)}%
            </span>
          </div>
        </div>
        {actions && <div className="flex flex-wrap items-center gap-1.5">{actions}</div>}
      </div>
      <p data-testid="worthiness-formula-legend" className="font-serif text-[11px] text-text-secondary">
        Gate formula: Novelty (prior-art distance) x Utility (corroborated claims &amp; replayability). Composite scores ≥ 65% clear autonomous Scientia promotion.
      </p>

      <div
        className={
          compact
            ? 'grid grid-cols-1 gap-3 lg:grid-cols-[190px_1fr]'
            : 'grid grid-cols-1 gap-4 lg:grid-cols-[220px_1fr]'
        }
      >
        <div className="flex items-center justify-center rounded-lg border border-border-subtle bg-bg-elevated p-2">
          <svg
            data-testid="worthiness-radar-svg"
            viewBox="-14 -8 248 236"
            role="img"
            aria-label="6-Axis Scientia Worthiness Radar"
            className={compact ? 'h-40 w-40' : 'h-44 w-44'}
          >
            {RINGS.map((ring) => (
              <polygon
                key={ring}
                points={polygonPoints(
                  totalAxes,
                  axes.map(() => ring),
                )}
                fill="none"
                stroke="rgba(212, 175, 55, 0.22)"
                strokeWidth="1"
              />
            ))}
            {axes.map((axis, idx) => {
              const [x, y] = polarPoint(idx, totalAxes, 1);
              const [lx, ly] = polarPoint(idx, totalAxes, 1.24);
              return (
                <g key={axis.axis}>
                  <line
                    x1={CENTER}
                    y1={CENTER}
                    x2={x}
                    y2={y}
                    stroke="rgba(212, 175, 55, 0.25)"
                    strokeWidth="1"
                  />
                  <text
                    x={lx}
                    y={ly}
                    textAnchor="middle"
                    dominantBaseline="middle"
                    className="fill-text-secondary font-sans text-[11px] font-medium"
                  >
                    {axis.label.slice(0, 10)}
                  </text>
                </g>
              );
            })}
            <polygon
              data-testid="worthiness-radar-polygon"
              points={dataPolygon}
              fill="rgba(197, 155, 39, 0.24)"
              stroke="var(--color-accent-default)"
              strokeWidth="2"
            />
            {axes.map((axis, idx) => {
              const [px, py] = polarPoint(idx, totalAxes, axis.score);
              return (
                <circle
                  key={axis.axis}
                  cx={px}
                  cy={py}
                  r="3.2"
                  fill="var(--color-bg-elevated)"
                  stroke="var(--color-accent-default)"
                  strokeWidth="1.5"
                />
              );
            })}
          </svg>
        </div>

        <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3">
          {axes.map((axis) => {
            const pct = Math.round(axis.score * 100);
            return (
              <div
                key={axis.axis}
                className="rounded-lg border border-border-subtle bg-bg-elevated p-2.5"
              >
                <div className="flex items-center justify-between font-mono text-[11px]">
                  <span className="truncate font-sans font-medium text-text-primary">{axis.label}</span>
                  <span
                    className={
                      pct >= 70
                        ? 'text-(--color-status-pass) font-semibold'
                        : pct >= 45
                          ? 'text-brass font-semibold'
                          : 'text-(--color-status-warn) font-semibold'
                    }
                  >
                    {pct}%
                  </span>
                </div>
                <div className="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-bg-panel border border-border-subtle">
                  <div
                    className={`h-full rounded-full ${
                      pct >= 70
                        ? 'bg-(--color-status-pass)'
                        : pct >= 45
                          ? 'bg-brass'
                          : 'bg-(--color-status-warn)'
                    }`}
                    style={{ width: `${pct}%` }}
                  />
                </div>
                <div className="mt-1 truncate font-sans text-[11px] text-text-muted">
                  {axis.description}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
