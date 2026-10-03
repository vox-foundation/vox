import React, { useEffect, useId, useRef, useState } from 'react';
import { CLUTCH_DETENTS, RISK_POSTURES, type ClutchId, type ControlState } from '../../../lib/driveConsole';
import { formatSpend } from '../../../config/budget';
import { RiskPopover } from './RiskPopover';

const TONE_BG: Record<string, string> = {
  rose: 'bg-(--color-status-fail)',
  amber: 'bg-(--color-status-warn)',
  emerald: 'bg-(--color-status-pass)',
};

interface DriveConsoleProps {
  control: ControlState;
  onControlChange: (next: Partial<ControlState>) => void;
  spentUsd: number;
  /** Positive only when the daemon reported a cap; 0 hides the cap and the bar. */
  budgetUsd: number;
  burnPerMin?: number;
  /** Extra controls shown inside the Risk popover (App passes Check replies). */
  riskExtra?: React.ReactNode;
}

export function DriveConsole({
  control,
  onControlChange,
  spentUsd,
  budgetUsd,
  burnPerMin,
  riskExtra,
}: DriveConsoleProps) {
  const [riskOpen, setRiskOpen] = useState(false);
  const [hintFor, setHintFor] = useState<ClutchId | null>(null);
  const hintId = useId();
  const riskAnchorRef = useRef<HTMLSpanElement>(null);
  const risk = RISK_POSTURES.find(r => r.id === control.risk)!;
  const pct = budgetUsd > 0 ? Math.min(100, (spentUsd / budgetUsd) * 100) : 0;
  const hint = hintFor ? CLUTCH_DETENTS.find(d => d.id === hintFor)?.hint ?? null : null;

  // Dismiss the risk popover on any interaction outside the trigger+popover
  // (same pattern as ChatSessionRail's row-menu dismiss) — otherwise only
  // Escape or a selection closes it.
  useEffect(() => {
    if (!riskOpen) return;
    const onPointerDown = (e: PointerEvent) => {
      if (!riskAnchorRef.current?.contains(e.target as Node)) setRiskOpen(false);
    };
    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  }, [riskOpen]);

  return (
    <div className="relative flex flex-wrap items-stretch rounded-lg border border-white/10 text-[11px]">
      {/* ① Mode */}
      <div className="relative flex items-center gap-1 border-r border-white/[0.07] px-2.5 py-1.5">
        <span className="text-text-muted" aria-hidden>⚙</span>
        <div role="radiogroup" aria-label="Mode — how much to spend" className="flex gap-0.5">
          {CLUTCH_DETENTS.map(d => (
            <button
              key={d.id}
              type="button"
              role="radio"
              aria-checked={control.clutch === d.id}
              aria-describedby={hintFor === d.id ? hintId : undefined}
              onClick={() => onControlChange({ clutch: d.id })}
              onMouseEnter={() => setHintFor(d.id)}
              onMouseLeave={() => setHintFor(null)}
              onFocus={() => setHintFor(d.id)}
              onBlur={() => setHintFor(null)}
              className={`min-h-[24px] rounded px-1.5 font-medium ${
                control.clutch === d.id
                  ? 'bg-brass/16 text-brass'
                  : 'text-zinc-400 hover:text-zinc-200'
              }`}
            >
              {d.label}
            </button>
          ))}
        </div>
        {hint && (
          <span
            id={hintId}
            role="tooltip"
            data-testid="drive-mode-hint"
            className="pointer-events-none absolute bottom-full left-0 z-40 mb-1 whitespace-nowrap rounded border border-white/10 bg-bg-base px-2 py-0.5 text-[11px] text-text-secondary"
          >
            {hint}
          </span>
        )}
      </div>

      {/* ② Spend — engine-wide; the status bar's Spend card has the breakdown */}
      <div
        data-testid="drive-console-spend"
        className="flex items-center gap-2 border-r border-white/[0.07] px-2.5 py-1.5"
        title="Engine spend across all sessions"
      >
        <span className="text-text-muted">Spend</span>
        <span className="font-mono text-brass">{formatSpend(spentUsd, budgetUsd > 0 ? budgetUsd : null)}</span>
        {budgetUsd > 0 && (
          <span className="relative h-[3px] w-12 rounded-sm bg-white/8">
            <span
              data-testid="drive-console-budget-bar" className="absolute inset-y-0 left-0 rounded-sm bg-brass"
              style={{ width: `${pct}%` }}
            />
          </span>
        )}
        {burnPerMin != null && (
          <span className="text-text-muted">↑${burnPerMin.toFixed(2)}/m</span>
        )}
      </div>

      {/* ③ Risk — trigger + upward-anchored popover share a relative anchor so
          the popover escapes the strip's flow instead of rendering clipped at
          its far edge. */}
      <span ref={riskAnchorRef} className="relative flex items-stretch">
        <button
          type="button"
          aria-label={`Risk: ${risk.label} — click to configure`}
          aria-expanded={riskOpen}
          onClick={() => setRiskOpen(o => !o)}
          className="flex items-center gap-1.5 px-2.5 py-1.5 hover:bg-white/3"
        >
          <span className={`h-3.5 w-[3px] rounded-sm ${TONE_BG[risk.tone]}`} aria-hidden />
          <span>Risk: {risk.label}</span>
          <span className="text-text-muted">▾</span>
        </button>
        <RiskPopover
          open={riskOpen}
          risk={control.risk}
          onChange={(n) => { onControlChange(n); setRiskOpen(false); }}
          onClose={() => setRiskOpen(false)}
        >
          {riskExtra}
        </RiskPopover>
      </span>
    </div>
  );
}
