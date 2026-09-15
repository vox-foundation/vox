import React, { useState, useEffect } from 'react';
import { Icon } from '../../ui/Icons';
import { Glass } from '../../ui/Glass';

export interface EmpiricalVerificationBadgeProps {
  sessionId: number;
  stabilityScore: number;
  onNavigateToResearch?: (sessionId: number) => void;
}

export function EmpiricalVerificationBadge({
  sessionId,
  stabilityScore,
  onNavigateToResearch,
}: EmpiricalVerificationBadgeProps) {
  const [drawerOpen, setDrawerOpen] = useState(false);

  useEffect(() => {
    if (!drawerOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setDrawerOpen(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [drawerOpen]);

  const isHighStability = stabilityScore >= 0.85;
  const badgeColor = isHighStability
    ? 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30'
    : 'bg-amber-500/15 text-amber-300 border-amber-500/30';

  return (
    <>
      <button
        type="button"
        onClick={() => setDrawerOpen(true)}
        aria-haspopup="dialog"
        aria-expanded={drawerOpen}
        className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full border font-mono text-[11px] transition-all hover:scale-105 cursor-pointer ${badgeColor}`}
      >
        <Icon.shield className="size-3.5 shrink-0" />
        <span className="font-semibold">Verified by Deep Research</span>
        <span className="opacity-70">(S = {stabilityScore.toFixed(2)})</span>
      </button>

      {drawerOpen && (
        <div className="fixed inset-0 z-50 flex justify-end">
          <button
            type="button"
            aria-label="Close backdrop"
            className="fixed inset-0 bg-black/50 backdrop-blur-xs cursor-default w-full h-full border-0 p-0"
            onClick={() => setDrawerOpen(false)}
            tabIndex={-1}
          />
          <Glass
            role="dialog"
            aria-modal="true"
            aria-label="Empirical Verification Evidence"
            className="relative z-10 flex h-full w-full max-w-md flex-col rounded-none bg-bg-base/95 border-l border-border-subtle shadow-2xl p-6 overflow-y-auto"
            inset={false}
          >
            <div className="flex items-center justify-between border-b border-border-subtle pb-3">
              <div>
                <h3 className="font-display text-sm font-semibold text-text-primary uppercase tracking-wider">
                  Empirical Verification Ledger
                </h3>
                <p className="font-mono text-[11px] text-text-muted mt-0.5">
                  Research Session #{sessionId} · Stability S = {stabilityScore.toFixed(2)}
                </p>
              </div>
              <button
                type="button"
                onClick={() => setDrawerOpen(false)}
                aria-label="Close"
                className="size-7 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-overlay-hover"
              >
                <Icon.x className="size-4" />
              </button>
            </div>

            <div className="mt-4 space-y-4 text-xs font-mono text-text-secondary">
              <div className="p-3 rounded bg-black/30 border border-border-subtle">
                <div className="font-semibold text-text-primary mb-1">Empirical Reproducibility</div>
                <p>
                  This document was synthesized from empirical test runs verified by compiler sandbox
                  execution and epistemic contradiction resolution.
                </p>
              </div>

              {onNavigateToResearch && (
                <button
                  type="button"
                  onClick={() => {
                    setDrawerOpen(false);
                    onNavigateToResearch(sessionId);
                  }}
                  className="w-full py-2 rounded bg-brass/20 text-brass border border-brass/40 hover:bg-brass/30 transition-colors"
                >
                  Open Full Research Session #{sessionId} →
                </button>
              )}
            </div>
          </Glass>
        </div>
      )}
    </>
  );
}
