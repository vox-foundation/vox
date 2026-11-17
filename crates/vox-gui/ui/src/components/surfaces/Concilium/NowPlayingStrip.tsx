import React from 'react';
import { Icon } from '../../ui/Icons';

export interface NowPlayingStripProps {
  activeWorkers: number;
  parkedWorkers: number;
  headSeq: number;
  activeLocks: number;
  tickerText: string;
  isLive: boolean;
  focusMode: 'swimlanes' | 'board';
  onToggleLive: () => void;
  onRefresh: () => void;
  onFocusModeChange: (mode: 'swimlanes' | 'board') => void;
}

export function NowPlayingStrip({
  activeWorkers,
  parkedWorkers,
  headSeq,
  activeLocks,
  tickerText,
  isLive,
  focusMode,
  onToggleLive,
  onRefresh,
  onFocusModeChange,
}: NowPlayingStripProps) {
  return (
    <header
      data-slot="concilium-now-playing"
      data-testid="concilium-now-playing"
      className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border-strong bg-bg-surface px-3.5 py-2.5 text-text-primary shadow-sm"
    >
      <div className="flex min-w-0 flex-wrap items-center gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-lg border border-brass/50 bg-brass/15 text-text-primary">
            <Icon.flow className="size-4 text-[var(--color-accent-default)]" aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="rounded border border-brass/45 bg-brass/15 px-1.5 py-0.5 font-display text-[9px] font-bold tracking-[0.14em] uppercase text-[var(--color-accent-default)]">
                VOX CONCILIUM
              </span>
              <h2 className="truncate font-display text-xs sm:text-sm font-bold tracking-[0.08em] text-text-primary">
                Agent topology &amp; W3C Causal Observatory
              </h2>
            </div>
            <p className="truncate font-mono text-[10px] text-text-muted">
              Perfetto traceparent swimlanes · #seq board · 0 tok/s parked hibernation
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2 font-mono text-[11px]">
          <div
            data-slot="concilium-stat-active"
            className="flex items-center gap-1.5 rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-1"
          >
            <span className="size-2 rounded-full bg-brass" aria-hidden="true" />
            <span className="text-[10px] uppercase tracking-wider text-text-muted">Active Workers</span>
            <span className="font-bold text-text-primary" data-testid="concilium-active-workers">
              {activeWorkers}
            </span>
          </div>

          <div
            data-slot="concilium-stat-parked"
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-status-pass)]/35 bg-[var(--color-status-pass)]/10 px-2.5 py-1"
          >
            <span className="size-2 rounded-full bg-[var(--color-status-pass)]" aria-hidden="true" />
            <span className="text-[10px] uppercase tracking-wider text-[var(--color-status-pass)]">
              Parked (0 tok/s)
            </span>
            <span className="font-bold text-[var(--color-status-pass)]" data-testid="concilium-parked-workers">
              {parkedWorkers}
            </span>
          </div>

          <div
            data-slot="concilium-stat-seq"
            className="flex items-center gap-1.5 rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-1"
          >
            <span className="text-[10px] uppercase tracking-wider text-text-muted">Head #seq</span>
            <span className="font-bold text-text-primary" data-testid="concilium-head-seq">
              #{headSeq}
            </span>
          </div>

          <div
            data-slot="concilium-stat-locks"
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-status-fail)]/35 bg-[var(--color-status-fail)]/10 px-2.5 py-1"
          >
            <Icon.lockKeyhole className="size-3 text-[var(--color-status-fail)]" aria-hidden="true" />
            <span className="text-[10px] uppercase tracking-wider text-[var(--color-status-fail)]">
              Active File Locks
            </span>
            <span className="font-bold text-[var(--color-status-fail)]" data-testid="concilium-active-locks">
              {activeLocks}
            </span>
          </div>
        </div>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-elevated px-3 py-1.5 font-mono text-[11px]">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={`size-2 shrink-0 rounded-full ${
              isLive ? 'bg-[var(--color-status-pass)] animate-pulse' : 'bg-[var(--color-text-muted)]'
            }`}
            aria-hidden="true"
          />
          <span className="shrink-0 font-display text-[9px] font-bold uppercase tracking-widest text-[var(--color-accent-default)]">
            NOW PLAYING
          </span>
          <span
            data-testid="concilium-now-playing-ticker"
            className="truncate text-text-secondary"
          >
            {tickerText}
          </span>
        </div>

        <div className="flex shrink-0 flex-wrap items-center gap-1.5">
          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            aria-pressed={focusMode === 'swimlanes'}
            onClick={() => onFocusModeChange('swimlanes')}
            className={`rounded border px-2 py-0.5 font-display text-[9px] font-bold uppercase tracking-wider transition-colors ${
              focusMode === 'swimlanes'
                ? 'border-brass bg-brass/20 text-text-primary'
                : 'border-border-subtle bg-bg-surface text-text-muted hover:text-text-primary'
            }`}
          >
            Swimlanes
          </button>
          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            aria-pressed={focusMode === 'board'}
            onClick={() => onFocusModeChange('board')}
            className={`rounded border px-2 py-0.5 font-display text-[9px] font-bold uppercase tracking-wider transition-colors ${
              focusMode === 'board'
                ? 'border-brass bg-brass/20 text-text-primary'
                : 'border-border-subtle bg-bg-surface text-text-muted hover:text-text-primary'
            }`}
          >
            #seq Board
          </button>
          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            aria-pressed={isLive}
            aria-label={isLive ? 'Pause live event stream' : 'Resume live event stream'}
            onClick={onToggleLive}
            className={`inline-flex items-center gap-1 rounded border px-2 py-0.5 font-display text-[9px] font-bold uppercase tracking-wider transition-colors ${
              isLive
                ? 'border-[var(--color-status-pass)]/40 bg-[var(--color-status-pass)]/12 text-[var(--color-status-pass)] hover:border-[var(--color-status-pass)]'
                : 'border-border-subtle bg-bg-surface text-text-secondary hover:border-border-strong'
            }`}
          >
            <span>{isLive ? 'Live' : 'Paused'}</span>
          </button>
          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            onClick={onRefresh}
            aria-label="Refresh Concilium snapshot"
            className="inline-flex items-center gap-1 rounded border border-border-strong bg-bg-surface px-2 py-0.5 font-display text-[9px] font-bold uppercase tracking-wider text-text-primary hover:border-brass"
          >
            <Icon.refresh className="size-3" aria-hidden="true" />
            <span>Sync</span>
          </button>
        </div>
      </div>
    </header>
  );
}
