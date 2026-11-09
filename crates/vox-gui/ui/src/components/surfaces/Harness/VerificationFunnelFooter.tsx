import React from 'react';
import type { RiverSnapshot } from './riverTypes';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

export interface VerificationFunnelFooterProps {
  snapshot: RiverSnapshot;
  condensed?: boolean;
  gamifyEnabled?: boolean;
}

const FUNNEL_STAGE_META: Record<
  string,
  {
    plainName: string;
    icon: (p: React.SVGProps<SVGSVGElement>) => React.JSX.Element;
    purpose: string;
  }
> = {
  candidates: {
    plainName: 'Generated',
    icon: Icon.scrollTabula,
    purpose: 'Raw candidate code turns and diff patches emitted by LLM workers prior to AST parsing.',
  },
  ast_lint: {
    plainName: 'Compiled',
    icon: Icon.anvilForge,
    purpose: 'AST parsing, syntax compilation, type checking, and import validation.',
  },
  cold_tests: {
    plainName: 'Tested',
    icon: Icon.gladiusShield,
    purpose: 'Automated test execution, unit test runner, and integration verification pass.',
  },
  adversarial_pass: {
    plainName: 'Gated & Verified',
    icon: Icon.laurelWreath,
    purpose: 'Adversarial doubt evaluation, cryptographic BLAKE3 receipt generation, and sandbox quarantine clearance.',
  },
};

export function VerificationFunnelFooter({
  snapshot,
  condensed = false,
  gamifyEnabled = true,
}: VerificationFunnelFooterProps) {
  return (
    <div
      data-testid="harness-verification-footer"
      className={`grid gap-2 font-mono text-[10px] text-[#1C140C] ${
        condensed ? 'grid-cols-1' : 'grid-cols-3'
      }`}
    >
      {/* Column 1: Failure / Doubt Attribution Ledger */}
      {!condensed && (
        <div
          style={{
            backgroundImage:
              "linear-gradient(180deg, rgba(250, 246, 238, 0.96) 0%, rgba(239, 230, 213, 0.95) 100%), url('/assets/romanesque/carrara-seamless-stone.jpg')",
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
          className="vox-marble-tablet rounded-xl border-2 border-[#C59B27]/85 p-2 shadow-[0_6px_16px_rgba(28,20,12,0.18),inset_0_1px_0_rgba(255,255,255,0.92)] space-y-1.5"
        >
          <div className="flex items-center gap-1.5 border-b border-[#C59B27]/45 pb-1 text-[9px] font-serif font-bold uppercase tracking-wider text-[#5C441E]">
            <Icon.alert className="size-3 text-[#9A5812]" />
            <span>FAILURE / DOUBT ATTRIBUTION</span>
          </div>
          <div className="space-y-1.5">
            {snapshot.failures.map((f) => (
              <div key={f.code} className="space-y-0.5">
                <div className="flex items-center justify-between text-[#1C140C]">
                  <span className="truncate font-bold text-[#1C140C]">{f.code}</span>
                  <span className="font-semibold text-[#5C441E]">
                    {f.interceptedCount} ({f.sharePct}%)
                  </span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-xs bg-[#D8CBB0] border border-[#A67C1E]/50">
                  <div
                    className="h-full bg-gradient-to-r from-[#8C6825] to-[#C59B27]"
                    style={{ width: `${f.sharePct}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Column 2: 4-Stage Stepped Travertine Settling Basins (Verification Retention Funnel) */}
      <div
        style={{
          backgroundImage:
            "linear-gradient(180deg, rgba(250, 246, 238, 0.96) 0%, rgba(239, 230, 213, 0.95) 100%), url('/assets/romanesque/carrara-seamless-stone.jpg')",
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="vox-marble-tablet rounded-xl border-2 border-[#C59B27]/85 p-2 shadow-[0_6px_16px_rgba(28,20,12,0.18),inset_0_1px_0_rgba(255,255,255,0.92)] space-y-1.5"
      >
        <div className="flex items-center justify-between border-b border-[#C59B27]/45 pb-1 text-[9px] font-serif font-bold uppercase tracking-wider text-[#5C441E]">
          <div className="flex items-center gap-1.5">
            <Icon.aqueductArch className="size-3 text-[#785312]" />
            <span>VERIFICATION RETENTION FUNNEL</span>
          </div>
          <span className="font-bold text-[#4D5D1E]">DETERMINISTIC GATE</span>
        </div>
        <div className="space-y-1">
          {snapshot.funnel.map((stage) => {
            const meta = FUNNEL_STAGE_META[stage.stageId] ?? {
              plainName: stage.label,
              icon: Icon.waxSeal,
              purpose: 'Verification gate in the retention pipeline.',
            };
            const StageIcon = meta.icon;

            return (
              <RomanTooltip
                key={stage.stageId}
                title={meta.plainName}
                purpose={meta.purpose}
                statusBadge={`${stage.count} (${stage.retentionPct}%)`}
              >
                <div className="flex flex-col items-center w-full cursor-default">
                  <div
                    className="flex items-center justify-between rounded-md border border-[#8C6825] bg-bg-elevated px-2 py-1 text-[9.5px] text-text-primary shadow-xs transition-[width] hover:border-[#D4AF37]"
                    style={{ width: `${Math.max(48, stage.retentionPct)}%` }}
                  >
                    <div className="flex items-center gap-1.5 min-w-0">
                      <StageIcon className="size-3 shrink-0 text-brass" />
                      <span className="font-serif font-bold tracking-wide text-text-primary truncate">
                        {meta.plainName}
                      </span>
                      <span className="text-[8px] font-mono uppercase tracking-wider text-text-muted truncate hidden sm:inline">
                        · {stage.label}
                      </span>
                    </div>
                    <span className="font-mono font-semibold text-text-primary shrink-0 ml-1">
                      {stage.count} ({stage.retentionPct}%)
                    </span>
                  </div>
                </div>
              </RomanTooltip>
            );
          })}
        </div>
      </div>

      {/* Column 3: Zero-Cascade SLO & Ludus Gamification Thrift Yield */}
      <div
        style={{
          backgroundImage:
            "linear-gradient(180deg, rgba(248, 243, 232, 0.96) 0%, rgba(235, 226, 208, 0.94) 100%), url('/assets/romanesque/travertine-parchment-tablet.jpg')",
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
        className="vox-marble-tablet rounded-xl border-2 border-[#C59B27]/85 p-2 shadow-[0_6px_16px_rgba(0,0,0,0.45),inset_0_1px_0_rgba(255,255,255,0.92)] flex flex-col justify-between space-y-1.5"
      >
        <div>
          <div className="flex items-center gap-1.5 border-b border-[#C59B27]/45 pb-1 text-[9px] font-serif font-bold uppercase tracking-wider text-[#5C441E]">
            <Icon.waxSeal className="size-3 text-[#4D5D1E]" />
            <span>ZERO-CASCADE SLO &amp; RECEIPTS</span>
          </div>
          <div className="mt-1.5 space-y-1 text-[9.5px]">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-[#5C441E]">BLAKE3 TOOL RECEIPTS</span>
              <span className="font-bold text-[#4D5D1E]">HMAC VALID</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="font-semibold text-[#5C441E]">SANDBOX ISOLATION</span>
              <span className="font-bold text-[#1C140C]">ENFORCED</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="font-semibold text-[#5C441E]">ACTIVE LAYER</span>
              <span className="truncate max-w-[150px] font-bold text-[#785312]">
                {snapshot.activePhaseSubtext}
              </span>
            </div>
          </div>
        </div>

        {gamifyEnabled && (
          <RomanTooltip
            title="Ludus Gamification Thrift Yield"
            purpose="Tokens and computation saved by deterministic caching and local MENS execution converted to civilization yield."
            statusBadge={`+${snapshot.ludusYield.xpEarned} XP`}
          >
            <div
              data-testid="ludus-crystal-yield"
              className="mt-2 flex items-center justify-between rounded-lg border border-[#D4AF37]/40 bg-bg-elevated px-2.5 py-1.5 cursor-default hover:border-[#D4AF37] transition"
            >
              <div className="flex items-center gap-1.5">
                <RomanMedallion size="sm" active>
                  <Icon.denariusCoin className="size-3 text-text-primary" />
                </RomanMedallion>
                <span className="text-[9px] font-serif uppercase tracking-wider text-text-primary font-bold">
                  LUDUS {snapshot.ludusYield.thriftRankLabel}
                </span>
              </div>
              <span className="font-mono font-semibold text-emerald-700">
                +{snapshot.ludusYield.xpEarned} XP · +{snapshot.ludusYield.crystalsEarned} ◆
              </span>
            </div>
          </RomanTooltip>
        )}
      </div>
    </div>
  );
}
