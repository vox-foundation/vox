import React, { useEffect, useId, useRef, useState } from 'react';
import { CLUTCH_DETENTS, RISK_POSTURES, type ClutchId, type ControlState } from '../../../lib/driveConsole';
import { formatSpend } from '../../../config/budget';
import { Icon } from '../../ui/Icons';
import { RiskPopover } from './RiskPopover';

const TONE_BG: Record<string, string> = {
  rose: 'bg-(--color-status-fail)',
  amber: 'bg-(--color-status-warn)',
  emerald: 'bg-(--color-status-pass)',
};

const RISK_TONE_PILL: Record<string, string> = {
  rose: 'text-(--color-status-fail) bg-(--color-status-fail)/10 hover:bg-(--color-status-fail)/18',
  amber: 'text-(--color-status-warn) bg-(--color-status-warn)/10 hover:bg-(--color-status-warn)/18',
  emerald: 'text-(--color-status-pass) bg-(--color-status-pass)/10 hover:bg-(--color-status-pass)/18',
};

const CLUTCH_ACTIVE_STYLE: Record<ClutchId, string> = {
  free: 'bg-(--color-status-pass)/18 text-(--color-status-pass) ring-1 ring-(--color-status-pass)/40',
  efficiency: 'bg-(--color-status-info)/18 text-(--color-status-info) ring-1 ring-(--color-status-info)/40',
  balanced: 'bg-brass/18 text-brass ring-1 ring-brass/40',
  genius: 'bg-violet-400/20 text-violet-300 ring-1 ring-violet-400/45',
};

const CLUTCH_KEYBIND: Record<ClutchId, string> = {
  free: 'Alt+1',
  efficiency: 'Alt+2',
  balanced: 'Alt+3',
  genius: 'Alt+4',
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
    <div className="relative flex min-h-7 max-w-full flex-wrap items-center whitespace-nowrap rounded-md border border-border-subtle bg-overlay-subtle/60 text-[11px]">
      {/* ① Mode */}
      <div className="relative flex items-center gap-1 border-r border-border-subtle px-1.5">
        <Icon.settings className="size-3 text-text-muted shrink-0" aria-hidden="true" />
        <div role="radiogroup" aria-label="Mode — how much to spend" className="flex items-center gap-0.5">
          {CLUTCH_DETENTS.map(d => (
            <button
              key={d.id}
              type="button"
              role="radio"
              aria-checked={control.clutch === d.id}
              aria-keyshortcuts={CLUTCH_KEYBIND[d.id]}
              data-keybind={CLUTCH_KEYBIND[d.id]}
              aria-describedby={hintFor === d.id ? hintId : undefined}
              onClick={() => onControlChange({ clutch: d.id })}
              onMouseEnter={() => setHintFor(d.id)}
              onMouseLeave={() => setHintFor(null)}
              onFocus={() => setHintFor(d.id)}
              onBlur={() => setHintFor(null)}
              className={`h-6 my-auto whitespace-nowrap rounded-full px-2 font-medium transition-colors ${
                control.clutch === d.id
                  ? CLUTCH_ACTIVE_STYLE[d.id]
                  : 'text-text-muted hover:text-text-primary hover:bg-overlay-hover/60'
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
            className="pointer-events-none absolute bottom-full left-0 z-[99999] mb-1 whitespace-nowrap rounded border border-border-subtle bg-bg-base/95 px-2 py-0.5 text-[11px] text-text-secondary shadow-xl backdrop-blur-md"
          >
            {hint}
          </span>
        )}
      </div>

      {/* ② Spend — engine-wide; the status bar's Spend card has the breakdown */}
      <div
        data-testid="drive-console-spend"
        data-spend-gate={pct >= 95 ? 'halt' : pct >= 80 ? 'downgrade' : 'normal'}
        className="flex items-center gap-1 border-r border-border-subtle px-1.5 whitespace-nowrap"
        title="Engine spend across all sessions (80% soft tier downgrade · 95% hard halt gate)"
      >
        <span className="text-text-muted">Spend</span>
        <span className="font-mono text-brass">{formatSpend(spentUsd, budgetUsd > 0 ? budgetUsd : null)}</span>
        {budgetUsd > 0 && (
          <span className="relative h-[4px] w-8 rounded-sm bg-overlay-hover">
            <span
              data-testid="drive-console-budget-bar"
              className="absolute inset-y-0 left-0 rounded-sm bg-linear-to-r from-(--color-status-pass) to-brass"
              style={{ width: `${pct}%` }}
            />
            <span
              data-testid="drive-console-downgrade-marker"
              title="80% Soft Downgrade Gate"
              className="absolute inset-y-0 w-px bg-(--color-status-warn)/80"
              style={{ left: '80%' }}
            />
            <span
              data-testid="drive-console-halt-marker"
              title="95% Hard Approval Gate"
              className="absolute inset-y-0 w-px bg-(--color-status-fail)/90"
              style={{ left: '95%' }}
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
          data-keybind="Alt+R"
          aria-keyshortcuts="Alt+R"
          title="Risk posture (Alt+R to cycle)"
          aria-label={`Risk: ${risk.label} — click to configure`}
          aria-expanded={riskOpen}
          onClick={() => setRiskOpen(o => !o)}
          className={`flex h-7 items-center gap-1 whitespace-nowrap rounded-r-md px-2 font-medium transition-colors ${RISK_TONE_PILL[risk.tone] ?? 'hover:bg-overlay-hover'}`}
        >
          <span className={`size-2 rounded-full ${TONE_BG[risk.tone]}`} aria-hidden />
          <span>Risk: {risk.label}</span>
          <Icon.chevR className="size-2.5 rotate-90 text-text-muted" aria-hidden="true" />
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
