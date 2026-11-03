import React from 'react';
import type { RiverSnapshot } from './riverTypes';

export interface VerificationFunnelFooterProps {
  snapshot: RiverSnapshot;
  condensed?: boolean;
  gamifyEnabled?: boolean;
}

export function VerificationFunnelFooter({
  snapshot,
  condensed = false,
  gamifyEnabled = true,
}: VerificationFunnelFooterProps) {
  return (
    <div
      data-testid="harness-verification-footer"
      className={`grid gap-2 font-mono text-[10px] ${condensed ? 'grid-cols-1' : 'grid-cols-3'}`}
    >
      {/* Column 1: Failure / Doubt Attribution Ledger */}
      {!condensed && (
        <div className="rounded-lg border border-white/10 bg-[#09090b] p-2.5">
          <div className="mb-2 text-[9px] uppercase tracking-wider text-text-muted">
            FAILURE / DOUBT ATTRIBUTION
          </div>
          <div className="space-y-2">
            {snapshot.failures.map((f) => (
              <div key={f.code} className="space-y-0.5">
                <div className="flex items-center justify-between text-zinc-200">
                  <span className="truncate">{f.code}</span>
                  <span className="text-text-muted">
                    {f.interceptedCount} ({f.sharePct}%)
                  </span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-sm bg-white/10">
                  <div className="h-full bg-zinc-300" style={{ width: `${f.sharePct}%` }} />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Column 2: 4-Stage Inverted Trapezoid Verification Retention Funnel */}
      <div className="rounded-lg border border-white/10 bg-[#09090b] p-2.5">
        <div className="mb-2 flex items-center justify-between text-[9px] uppercase tracking-wider text-text-muted">
          <span>VERIFICATION RETENTION FUNNEL</span>
          <span className="text-emerald-400">DETERMINISTIC GATE</span>
        </div>
        <div className="space-y-1.5">
          {snapshot.funnel.map((stage) => (
            <div key={stage.stageId} className="flex flex-col items-center">
              <div
                className="flex items-center justify-between rounded-sm border border-white/15 bg-white/[0.06] px-2 py-1 text-[9px] text-zinc-100 transition-all"
                style={{ width: `${Math.max(42, stage.retentionPct)}%` }}
              >
                <span className="truncate">{stage.label}</span>
                <span className="font-semibold text-cyan">
                  {stage.count} ({stage.retentionPct}%)
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Column 3: Zero-Cascade SLO & Ludus Gamification Thrift Yield */}
      <div className="rounded-lg border border-white/10 bg-[#09090b] p-2.5 flex flex-col justify-between">
        <div>
          <div className="mb-2 text-[9px] uppercase tracking-wider text-text-muted">
            ZERO-CASCADE SLO & RECEIPTS
          </div>
          <div className="space-y-1 text-[10px]">
            <div className="flex items-center justify-between">
              <span className="text-text-muted">BLAKE3 TOOL RECEIPTS</span>
              <span className="text-emerald-400">HMAC VALID</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-text-muted">SANDBOX ISOLATION</span>
              <span className="text-zinc-200">ENFORCED</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-text-muted">ACTIVE LAYER</span>
              <span className="truncate max-w-[150px] text-cyan">{snapshot.activePhaseSubtext}</span>
            </div>
          </div>
        </div>

        {gamifyEnabled && (
          <div
            data-testid="ludus-crystal-yield"
            className="mt-2 flex items-center justify-between rounded border border-emerald-500/25 bg-emerald-500/10 px-2 py-1"
          >
            <span className="text-[9px] uppercase tracking-wider text-emerald-300">
              LUDUS {snapshot.ludusYield.thriftRankLabel}
            </span>
            <span className="font-semibold text-emerald-200">
              +{snapshot.ludusYield.xpEarned} XP · +{snapshot.ludusYield.crystalsEarned} ◆
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
