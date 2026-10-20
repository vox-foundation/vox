import React from 'react';
import type { HarnessTierId, RiverSnapshot } from './riverTypes';

export interface WaveformTierStripProps {
  snapshot: RiverSnapshot;
  condensed?: boolean;
  onSelectTier?: (tierId: HarnessTierId) => void;
}

export function WaveformTierStrip({
  snapshot,
  condensed = false,
  onSelectTier,
}: WaveformTierStripProps) {
  return (
    <div
      data-testid="harness-waveform-tier-strip"
      className="space-y-2 rounded-lg border border-white/10 bg-[#09090b] p-2.5 font-mono text-[11px]"
    >
      {/* Top Row: Live Audio-Style Throughput Waveform + Key Metrics */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-white/10 pb-2">
        <div className="flex items-center gap-2">
          <div
            role="img"
            aria-label="Token throughput waveform"
            className="flex h-6 items-end gap-[2px] rounded bg-black/60 px-1.5 py-1 border border-white/5"
          >
            {snapshot.waveformBars.slice(0, condensed ? 12 : 24).map((val, idx) => (
              <span
                key={idx}
                className="w-[2px] bg-zinc-200 transition-all duration-300"
                style={{ height: `${Math.max(15, Math.round(val * 100))}%` }}
              />
            ))}
          </div>
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">THROUGHPUT</div>
            <div className="text-xs font-semibold text-zinc-100">{snapshot.throughputTokPerSec} tok/s</div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">BLAKE3 CACHE</div>
            <div className="text-xs font-semibold text-cyan">{snapshot.cacheHitPct}%</div>
          </div>
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">THRIFT</div>
            <div className="text-xs font-semibold text-emerald-400">
              -{snapshot.tokenSavingsPct}% (${snapshot.savedUsd.toFixed(2)})
            </div>
          </div>
          <div>
            <div className="text-[9px] uppercase tracking-wider text-text-muted">AGENTS</div>
            <div className="text-xs font-semibold text-zinc-100">{snapshot.activeAgents}</div>
          </div>
        </div>
      </div>

      {/* Middle Row: 3-Tier Model Cascade Selector Cards */}
      <div className={`grid gap-1.5 ${condensed ? 'grid-cols-1' : 'grid-cols-3'}`}>
        {snapshot.tiers.map((tier) => {
          const isInteractive = Boolean(onSelectTier);
          return (
            <button
              key={tier.tierId}
              type="button"
              disabled={!isInteractive}
              data-testid={`tier-card-${tier.tierId}`}
              onClick={isInteractive ? () => onSelectTier!(tier.tierId) : undefined}
              className={`flex flex-col items-start justify-between rounded border border-white/10 bg-white/[0.02] px-2.5 py-1.5 text-left transition-colors ${
                isInteractive
                  ? 'cursor-pointer hover:border-cyan/50 hover:bg-white/[0.05]'
                  : 'cursor-default'
              }`}
            >
            <div className="flex w-full items-center justify-between gap-1">
              <span className="text-[9px] font-semibold uppercase tracking-wider text-text-muted">
                {tier.title}
              </span>
              <span className="rounded bg-white/10 px-1 py-0.5 text-[9px] text-zinc-300">
                {tier.latencySlo}
              </span>
            </div>
            <div className="mt-0.5 truncate w-full text-[11px] font-semibold text-zinc-100">
              {tier.modelLabel}
            </div>
            <div className="mt-1 flex w-full items-center justify-between text-[9px] text-cyan">
              <span className="truncate">{tier.badge}</span>
              <span className="shrink-0 text-emerald-400">{tier.tokenDeltaLabel}</span>
            </div>
          </button>
          );
        })}
      </div>

      {/* Bottom Row: 5 Stage Progress Bars (hidden in ultra-condensed dock view) */}
      {!condensed && (
        <div className="grid grid-cols-5 gap-1.5 pt-1">
          {snapshot.stages.map((st) => (
            <div key={st.id} className="space-y-0.5">
              <div className="flex items-center justify-between text-[9px] text-text-muted">
                <span className={st.active ? 'font-semibold text-zinc-100' : ''}>{st.label}</span>
                <span>{st.pct}%</span>
              </div>
              <div className="h-1 w-full overflow-hidden rounded-sm bg-white/10">
                <div
                  className={`h-full ${st.active ? 'bg-cyan' : 'bg-zinc-300'}`}
                  style={{ width: `${st.pct}%` }}
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
