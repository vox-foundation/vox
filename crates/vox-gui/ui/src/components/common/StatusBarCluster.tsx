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

  // A failed fetch clears the reading: the popover never shows health it did not just receive.
  useEffect(() => {
    let cancelled = false;
    getResearchEngineStatus()
      .then((next) => {
        if (!cancelled) setStatus(next);
      })
      .catch(() => {
        if (!cancelled) setStatus(null);
      });
    return () => {
      cancelled = true;
    };
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

  const activeLane = status?.active_lane ?? null;
  const tavily = status?.providers.find((p) => p.id === 'tavily');
  const tavilyQuota = tavily?.quota_usage ?? null;
  const tavilyRemaining = tavilyQuota
    ? Math.max(0, tavilyQuota.units_limit - tavilyQuota.units_spent)
    : null;
  // Seconds from the engine's own timeouts, never a hardcoded figure.
  const seconds = (ms: number) => `${Number((ms / 1000).toFixed(1))}s`;

  return (
    <div className={`relative inline-flex items-center ${className}`}>
      <button
        ref={triggerRef}
        type="button"
        data-testid="status-bar-cluster-trigger"
        onClick={() => setIsOpen((o) => !o)}
        aria-expanded={isOpen}
        aria-label="Research & Engine Status"
        className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-sm px-2 py-0.5 text-[10px] text-text-muted hover:bg-overlay-subtle hover:text-text-secondary transition"
      >
        <span className="uppercase tracking-[0.14em] text-text-muted">Research</span>
        <span className="font-mono tabular-nums text-text-secondary">
          {activeLane === 'deep' ? '🔬 Deep' : activeLane === 'fast' ? '⚡ Fast' : 'unknown'}
          {tavilyQuota && tavilyRemaining !== null ? ` · ${tavilyRemaining}/${tavilyQuota.units_limit}` : ''}
        </span>
      </button>

      {isOpen && (
        <div
          ref={popoverRef}
          data-testid="status-bar-cluster-popover"
          role="dialog"
          aria-label="Research engine status"
          className="absolute bottom-full right-0 z-50 mb-1 w-80 rounded-xl border border-border-subtle bg-bg-base p-4 shadow-2xl space-y-3 text-xs"
        >
          <div className="flex items-center justify-between border-b border-border-subtle pb-2">
            <span className="font-display font-semibold text-text-primary tracking-wide">Research engine</span>
          </div>

          {status === null ? (
            <p data-testid="status-bar-cluster-unknown" className="text-[11px] text-text-muted">
              Research status unknown: the engine did not report.
            </p>
          ) : (
            <div className="space-y-2 text-[11px]">
              <div>
                <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">Providers</div>
                <ul className="mt-1 space-y-0.5 font-mono text-[10px]">
                  {status.providers.map((p) => (
                    <li
                      key={p.id}
                      data-testid={`status-bar-cluster-provider-${p.id}`}
                      className="flex justify-between gap-2"
                    >
                      <span className="text-text-secondary">{p.name}</span>
                      <span className="text-text-muted">
                        {p.is_enabled ? 'on' : 'off'}
                        {!p.is_keyless && !p.has_key ? ' · no key' : ''}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
              {tavilyQuota && (
                <div className="font-mono text-[10px] text-text-secondary">
                  Tavily quota: {tavilyRemaining}/{tavilyQuota.units_limit} left
                </div>
              )}
              <div>
                <div className="font-medium text-text-muted uppercase text-[9px] tracking-wider">Lane</div>
                <div className="font-mono text-[10px] space-y-0.5">
                  <div className={activeLane === 'fast' ? 'text-brass font-semibold' : 'text-text-muted'}>
                    ⚡ Fast (≤{seconds(status.fast_timeout_ms)})
                  </div>
                  <div className={activeLane === 'deep' ? 'text-brass font-semibold' : 'text-text-muted'}>
                    🔬 Deep (≤{seconds(status.deep_timeout_ms)})
                  </div>
                </div>
              </div>
            </div>
          )}

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
