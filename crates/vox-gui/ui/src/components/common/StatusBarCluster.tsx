import React, { useState, useEffect, useRef } from 'react';
import { getResearchEngineStatus, type ResearchEngineStatusDto } from '../surfaces/Research/researchActions';

export interface StatusBarClusterProps {
  onOpenDrawer?: () => void;
  className?: string;
}

export function StatusBarCluster({ onOpenDrawer, className = '' }: StatusBarClusterProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [status, setStatus] = useState<ResearchEngineStatusDto | null>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    getResearchEngineStatus().then(setStatus).catch(() => {});
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const handleOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (popoverRef.current?.contains(target) || triggerRef.current?.contains(target)) {
        return;
      }
      setIsOpen(false);
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', handleOutside);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleOutside);
      document.removeEventListener('keydown', handleKey);
    };
  }, [isOpen]);

  const activeLane = status?.active_lane ?? 'fast';
  const tavily = status?.providers.find((p) => p.id === 'tavily');
  const tavilyRemaining = tavily?.quota_usage
    ? Math.max(0, tavily.quota_usage.units_limit - tavily.quota_usage.units_spent)
    : null;

  return (
    <div className={`relative inline-flex items-center ${className}`}>
      <button
        ref={triggerRef}
        type="button"
        data-testid="status-bar-cluster-trigger"
        onClick={() => setIsOpen((o) => !o)}
        aria-expanded={isOpen}
        aria-label="Research & Engine Status"
        className="inline-flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
      >
        <span className="uppercase tracking-[0.14em] text-text-muted">Research</span>
        <span className="font-mono tabular-nums text-text-secondary">
          {activeLane === 'deep' ? '🔬 Deep' : '⚡ Fast'}
          {tavilyRemaining !== null ? ` · ${tavilyRemaining}/1000` : ''}
        </span>
      </button>

      {isOpen && (
        <div
          ref={popoverRef}
          data-testid="status-bar-cluster-popover"
          role="dialog"
          aria-label="Research System Health"
          className="absolute bottom-full right-0 z-50 mb-1 w-80 rounded-xl border border-border-subtle bg-bg-base p-4 shadow-2xl space-y-3 text-xs"
        >
          <div className="flex items-center justify-between border-b border-border-subtle pb-2">
            <span className="font-display font-semibold text-text-primary tracking-wide">
              System & Research Health
            </span>
            <span className="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-mono text-emerald-400">
              Online
            </span>
          </div>

          <div className="grid grid-cols-2 gap-2 text-[11px]">
            {/* Quadrant 1: Keyless Engines */}
            <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-2 space-y-1">
              <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">
                Keyless Engines
              </div>
              <div className="text-emerald-400 font-mono text-[10px] space-y-0.5">
                <div>✓ Wikipedia (Live)</div>
                <div>✓ OpenAlex (Academic)</div>
                <div>✓ arXiv (Preprints)</div>
                <div>✓ SearXNG (Meta)</div>
              </div>
            </div>

            {/* Quadrant 2: Free Quota Tracking */}
            <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-2 space-y-1">
              <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">
                Search Quotas
              </div>
              <div className="font-mono text-[10px]">
                {tavily ? (
                  <div>
                    Tavily: <span className="text-brass">{tavilyRemaining ?? '0'}/1000</span>
                  </div>
                ) : (
                  <div className="text-text-muted">Tavily: Free tier</div>
                )}
                <div className="text-text-muted">Resets monthly</div>
              </div>
            </div>

            {/* Quadrant 3: Dual-Lane Routing */}
            <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-2 space-y-1">
              <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">
                Lane Routing
              </div>
              <div className="font-mono text-[10px] space-y-0.5">
                <div className={activeLane === 'fast' ? 'text-brass font-semibold' : 'text-text-muted'}>
                  ⚡ Fast (&le;2.5s)
                </div>
                <div className={activeLane === 'deep' ? 'text-brass font-semibold' : 'text-text-muted'}>
                  🔬 Deep (&le;15s)
                </div>
              </div>
            </div>

            {/* Quadrant 4: Clavis Vault */}
            <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-2 space-y-1">
              <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">
                Clavis Vault
              </div>
              <div className="font-mono text-[10px] text-text-secondary space-y-0.5">
                <div>Encrypted Local</div>
                <div>0 Plaintext Keys</div>
              </div>
            </div>
          </div>

          {onOpenDrawer && (
            <button
              type="button"
              onClick={() => {
                setIsOpen(false);
                onOpenDrawer();
              }}
              className="w-full rounded border border-brass/40 bg-brass/10 hover:bg-brass/20 text-brass py-1 text-[11px] font-medium transition-colors text-center"
            >
              Configure Engines &amp; Keys ↗
            </button>
          )}
        </div>
      )}
    </div>
  );
}
