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
      className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-[#D4AF37]/35 bg-[#0B0D12] px-3.5 py-2.5 text-[#F3E5AB] shadow-[0_4px_16px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(243,229,171,0.14)]"
    >
      <div className="flex min-w-0 flex-wrap items-center gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <span className="inline-flex size-7 shrink-0 items-center justify-center rounded-lg border border-[#D4AF37]/50 bg-[#D4AF37]/15 text-[#F3E5AB]">
            <Icon.flow className="size-4" aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="rounded border border-[#D4AF37]/45 bg-[#D4AF37]/15 px-1.5 py-0.5 font-display text-[9px] font-bold tracking-[0.14em] uppercase text-[#F3E5AB]">
                VOX CONCILIUM
              </span>
              <h2 className="truncate font-display text-xs sm:text-sm font-bold tracking-[0.08em] text-[#F3E5AB]">
                Agent topology &amp; W3C Causal Observatory
              </h2>
            </div>
            <p className="truncate font-mono text-[10px] text-slate-400">
              Perfetto traceparent swimlanes · #seq board · 0 tok/s parked hibernation
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2 font-mono text-[11px]">
          <div
            data-slot="concilium-stat-active"
            className="flex items-center gap-1.5 rounded-lg border border-[#D4AF37]/30 bg-slate-900/80 px-2.5 py-1"
          >
            <span className="size-2 rounded-full bg-[#D4AF37]" aria-hidden="true" />
            <span className="text-[10px] uppercase tracking-wider text-slate-400">Active Workers</span>
            <span className="font-bold text-[#F3E5AB]" data-testid="concilium-active-workers">
              {activeWorkers}
            </span>
          </div>

          <div
            data-slot="concilium-stat-parked"
            className="flex items-center gap-1.5 rounded-lg border border-emerald-500/30 bg-emerald-950/30 px-2.5 py-1"
          >
            <span className="size-2 rounded-full bg-emerald-400" aria-hidden="true" />
            <span className="text-[10px] uppercase tracking-wider text-emerald-300/80">
              Parked (0 tok/s)
            </span>
            <span className="font-bold text-emerald-300" data-testid="concilium-parked-workers">
              {parkedWorkers}
            </span>
          </div>

          <div
            data-slot="concilium-stat-seq"
            className="flex items-center gap-1.5 rounded-lg border border-[#D4AF37]/30 bg-slate-900/80 px-2.5 py-1"
          >
            <span className="text-[10px] uppercase tracking-wider text-slate-400">Head #seq</span>
            <span className="font-bold text-[#F3E5AB]" data-testid="concilium-head-seq">
              #{headSeq}
            </span>
          </div>

          <div
            data-slot="concilium-stat-locks"
            className="flex items-center gap-1.5 rounded-lg border border-rose-500/30 bg-rose-950/25 px-2.5 py-1"
          >
            <Icon.lockKeyhole className="size-3 text-rose-400" aria-hidden="true" />
            <span className="text-[10px] uppercase tracking-wider text-rose-300/80">
              Active File Locks
            </span>
            <span className="font-bold text-rose-300" data-testid="concilium-active-locks">
              {activeLocks}
            </span>
          </div>
        </div>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-between gap-2 rounded-lg border border-[#D4AF37]/20 bg-slate-950/90 px-3 py-1.5 font-mono text-[11px]">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={`size-2 shrink-0 rounded-full ${
              isLive ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'
            }`}
            aria-hidden="true"
          />
          <span className="shrink-0 font-display text-[9px] font-bold uppercase tracking-widest text-[#D4AF37]">
            NOW PLAYING
          </span>
          <span
            data-testid="concilium-now-playing-ticker"
            className="truncate text-slate-200"
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
                ? 'border-[#D4AF37] bg-[#D4AF37]/20 text-[#F3E5AB]'
                : 'border-slate-700 bg-slate-900 text-slate-400 hover:text-slate-200'
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
                ? 'border-[#D4AF37] bg-[#D4AF37]/20 text-[#F3E5AB]'
                : 'border-slate-700 bg-slate-900 text-slate-400 hover:text-slate-200'
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
                ? 'border-emerald-500/40 bg-emerald-950/40 text-emerald-300 hover:border-emerald-400'
                : 'border-slate-600 bg-slate-900 text-slate-300 hover:border-slate-400'
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
            className="inline-flex items-center gap-1 rounded border border-[#D4AF37]/35 bg-slate-900 px-2 py-0.5 font-display text-[9px] font-bold uppercase tracking-wider text-[#F3E5AB] hover:border-[#D4AF37]"
          >
            <Icon.refresh className="size-3" aria-hidden="true" />
            <span>Sync</span>
          </button>
        </div>
      </div>
    </header>
  );
}
