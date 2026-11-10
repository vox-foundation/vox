import React from 'react';
import type { HarnessTierId, RiverSnapshot } from './riverTypes';
import { RomanTooltip, RomanBrassGauge, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

export interface WaveformTierStripProps {
  snapshot: RiverSnapshot;
  condensed?: boolean;
  onSelectTier?: (tierId: HarnessTierId) => void;
}

const TIER_METADATA: Record<
  HarnessTierId,
  {
    plainTitle: string;
    purpose: string;
    accentColor: string;
    borderColor: string;
    bgGradient: string;
  }
> = {
  intercept: {
    plainTitle: 'Tier 1 · Fast Local MENS',
    purpose:
      'Low-latency on-device model for quick edits, syntax fixes, and instant AST transforms.',
    accentColor: '#D4AF37',
    borderColor: 'border-[#D4AF37]/50',
    bgGradient: 'from-[#F8F3E8] via-[#EFE6D2] to-[#E6DBC4]',
  },
  fanout: {
    plainTitle: 'Tier 2 · Balanced Hybrid',
    purpose:
      'Balanced routing between local MENS and cloud models for multi-file refactoring and graph fan-out.',
    accentColor: '#D97706',
    borderColor: 'border-[#D97706]/50',
    bgGradient: 'from-[#F8F3E8] via-[#EFE6D2] to-[#E6DBC4]',
  },
  verifier: {
    plainTitle: 'Tier 3 · Deep Architecture',
    purpose:
      'Heavy reasoning tier for full-workspace causal analysis, security audits, and formal verification.',
    accentColor: '#5B6B24',
    borderColor: 'border-[#C59B27]/85',
    bgGradient: 'from-[#F8F3E8] via-[#EFE6D2] to-[#E6DBC4]',
  },
};

export function WaveformTierStrip({
  snapshot,
  condensed = false,
  onSelectTier,
}: WaveformTierStripProps) {
  return (
    <div
      data-testid="harness-waveform-tier-strip"
      className="space-y-1.5 rounded-xl border border-[#C59B27]/40 bg-bg-surface p-2 font-mono text-[11px] text-text-primary shadow-sm"
    >
      {/* Top Row: 3 Symmetrical Roman Brass Dial Gauges (Concept Art Option B) + Live Waveform */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-1.5">
        <div className="flex flex-wrap items-center gap-3">
          <RomanBrassGauge
            label="THROUGHPUT"
            valueText={`${snapshot.throughputTokPerSec} tok/s`}
            ratio={Math.min(1, Math.max(0.18, snapshot.throughputTokPerSec / 320))}
            tone="gold"
            purpose="Live token generation velocity across local MENS and cloud tiers."
          />

          <RomanBrassGauge
            label="BLAKE3 CACHE"
            valueText={`${snapshot.cacheHitPct}%`}
            ratio={snapshot.cacheHitPct / 100}
            tone="gold"
            purpose="BLAKE3 content-addressed cache hit rate for prompt tokens and AST digests."
          />

          <RomanBrassGauge
            label="THRIFT"
            valueText={`-${snapshot.tokenSavingsPct}% ($${snapshot.tokenSavingsPct > 0 ? snapshot.savedUsd.toFixed(2) : '0.00'})`}
            ratio={snapshot.tokenSavingsPct / 100}
            tone="emerald"
            purpose="Token and dollar savings from compression, prompt caching, and local MENS execution."
          />
        </div>

        <div className="flex items-center gap-2.5">
          <div
            role="img"
            aria-label="Token throughput waveform"
            className="flex h-7 items-end gap-[2px] rounded-md bg-bg-elevated px-2 py-1 border border-border-subtle"
          >
            {snapshot.waveformBars.slice(0, condensed ? 12 : 20).map((val, idx) => (
              <span
                key={idx}
                className="w-[2px] rounded-xs bg-gradient-to-t from-[#B4881A] via-[#D4AF37] to-[#F3E5AB] transition-[height] duration-300"
                style={{ height: `${Math.max(15, Math.round(val * 100))}%` }}
              />
            ))}
          </div>

          <div className="flex items-center gap-2 rounded-lg border border-border-subtle bg-bg-elevated px-2.5 py-1">
            <RomanMedallion size="sm">
              <Icon.romanColumn className="size-3 text-brass" />
            </RomanMedallion>
            <div>
              <div className="text-[9px] font-serif uppercase tracking-wider text-text-secondary">
                AGENTS
              </div>
              <div className="text-xs font-semibold text-text-primary">
                {snapshot.activeAgents}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Middle Row: 3-Tier Model Cascade Sluice Cards */}
      <div className={`grid gap-2 ${condensed ? 'grid-cols-1' : 'grid-cols-3'}`}>
        {snapshot.tiers.map((tier) => {
          const isInteractive = Boolean(onSelectTier);
          const meta = TIER_METADATA[tier.tierId] ?? {
            plainTitle: tier.title,
            purpose: 'Execution tier in the autonomous cascade.',
            accentColor: '#D4AF37',
            borderColor: 'border-[#D4AF37]/40',
            bgGradient: 'from-[#F8F3E8] to-[#E6DBC4]',
          };

          const card = (
            <button
              type="button"
              disabled={!isInteractive}
              data-testid={`tier-card-${tier.tierId}`}
              onClick={isInteractive ? () => onSelectTier!(tier.tierId) : undefined}
              style={{
                backgroundImage:
                  "linear-gradient(180deg, rgba(248, 243, 232, 0.96) 0%, rgba(235, 226, 208, 0.94) 100%), url('/assets/romanesque/travertine-parchment-tablet.jpg')",
                backgroundSize: 'cover',
                backgroundPosition: 'center',
              }}
              className={`vox-marble-tablet flex w-full flex-col items-start justify-between rounded-lg border-2 border-[#C59B27]/85 p-2 text-left text-[#1C140C] transition shadow-[0_6px_16px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)] ${
                isInteractive
                  ? 'cursor-pointer hover:border-[#8C6825] hover:shadow-[0_4px_16px_rgba(212,175,55,0.35)]'
                  : 'cursor-default'
              }`}
            >
              <div className="flex w-full items-center justify-between gap-1">
                <span className="font-serif text-[11px] font-bold tracking-wide text-[#1C140C]">
                  {meta.plainTitle}
                </span>
                <span className="rounded border border-[#8C6825] bg-bg-elevated px-1.5 py-0.5 text-[9px] font-mono text-text-primary">
                  {tier.latencySlo}
                </span>
              </div>
              <div className="mt-0.5 text-[8.5px] font-mono font-semibold uppercase tracking-wider text-[#5C441E]">
                {tier.title}
              </div>
              <div className="mt-0.5 truncate w-full text-[11px] font-bold text-[#1C140C]">
                {tier.modelLabel}
              </div>
              <div className="mt-0.5 flex w-full items-center justify-between text-[9px]">
                <span className="truncate font-semibold text-[#6B5125]">{tier.badge}</span>
                <span className="shrink-0 font-bold text-[#4D5D1E]">{tier.tokenDeltaLabel}</span>
              </div>
            </button>
          );

          return (
            <RomanTooltip
              key={tier.tierId}
              title={meta.plainTitle}
              purpose={meta.purpose}
              actionHint={isInteractive ? `Click to activate ${meta.plainTitle}` : undefined}
              statusBadge={tier.latencySlo}
            >
              {card}
            </RomanTooltip>
          );
        })}
      </div>

      {/* Bottom Row: Progressive-Disclosure 5 Stage Progress Sluices */}
      {!condensed && (
        <details className="group pt-0.5">
          <summary className="flex cursor-pointer list-none items-center justify-between rounded-md border border-border-subtle bg-bg-elevated px-2.5 py-1 text-[9.5px] text-text-secondary hover:border-[#D4AF37]/40 hover:text-text-primary">
            <span className="font-display uppercase tracking-wider">
              ▸ Pipeline Stage Gates (5-Stage Sluice Breakdown)
            </span>
            <span className="text-brass">100% Verified</span>
          </summary>
          <div className="mt-2 grid grid-cols-5 gap-1.5">
            {snapshot.stages.map((st) => (
              <div key={st.id} className="space-y-1 min-w-0">
                <div className="flex items-center justify-between gap-1 text-[9px] text-text-secondary">
                  <span
                    className={`truncate font-mono ${
                      st.active ? 'font-bold text-text-primary' : 'text-text-muted'
                    }`}
                  >
                    {st.label}
                  </span>
                  <span className="shrink-0 font-mono">{st.pct}%</span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-sm bg-bg-elevated border border-border-subtle">
                  <div
                    className={`h-full transition-all duration-300 ${
                      st.active
                        ? 'bg-gradient-to-r from-[#B4881A] to-[#F3E5AB] shadow-[0_0_6px_rgba(212,175,55,0.4)]'
                        : 'bg-[#5C4B2B]'
                    }`}
                    style={{ width: `${st.pct}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
