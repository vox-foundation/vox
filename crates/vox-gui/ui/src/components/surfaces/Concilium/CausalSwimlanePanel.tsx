import React from 'react';
import type {
  ComputeTier,
  ConciliumAgentLane,
  ConciliumSpanBar,
  ConciliumSpanKind,
  ConciliumWakeupArc,
} from '../../../types/concilium';
import { Icon } from '../../ui/Icons';

export interface CausalSwimlanePanelProps {
  lanes: ConciliumAgentLane[];
  wakeupArcs: ConciliumWakeupArc[];
  windowDurationMs: number;
  selectedAgentId: string | null;
  onSelectAgent: (agentId: string) => void;
}

export function formatComputeTierBadge(tier: ComputeTier): {
  label: string;
  className: string;
} {
  switch (tier) {
    case 'cloud_bandit':
      return {
        label: 'CLOUD BANDIT',
        className: 'border-[#D4AF37]/50 bg-[#D4AF37]/15 text-[#F3E5AB]',
      };
    case 'local_mens_gpu':
      return {
        label: 'LOCAL MENS GPU',
        className: 'border-emerald-500/50 bg-emerald-950/50 text-emerald-300',
      };
    case 'populi_quic_peer':
      return {
        label: 'POPULI QUIC PEER',
        className: 'border-amber-500/50 bg-amber-950/50 text-amber-300',
      };
    case 'inline_cpu':
      return {
        label: 'INLINE CPU',
        className: 'border-slate-500/50 bg-slate-900 text-slate-300',
      };
  }
}

function spanToneClasses(kind: ConciliumSpanKind): string {
  switch (kind) {
    case 'thinking':
      return 'border-[#D4AF37] bg-[#D4AF37]/25 text-[#F3E5AB]';
    case 'tool_call':
    case 'writing':
      return 'border-emerald-400/80 bg-emerald-500/25 text-emerald-100';
    case 'parked':
      return 'border-dashed border-slate-500/80 bg-slate-800/50 text-slate-300';
    case 'lock_wait':
    case 'trip':
      return 'border-rose-400/80 bg-rose-500/25 text-rose-100';
  }
}

function renderSpanLabel(span: ConciliumSpanBar, lane: ConciliumAgentLane): string {
  if (span.kind === 'parked') {
    const seqNum = span.woken_by_seq ?? lane.parked_since_seq ?? 0;
    return `0 tok/s · Asleep on #seq-${seqNum}`;
  }
  return `${span.label} (${span.duration_ms}ms)`;
}

export function CausalSwimlanePanel({
  lanes,
  wakeupArcs,
  windowDurationMs,
  selectedAgentId,
  onSelectAgent,
}: CausalSwimlanePanelProps) {
  const safeWindowMs = Math.max(
    1000,
    Number.isFinite(windowDurationMs) ? windowDurationMs : 10000,
  );
  const svgHeight = Math.max(72, lanes.length * 56);

  return (
    <section
      data-slot="concilium-swimlanes"
      data-testid="concilium-swimlanes"
      aria-label="W3C traceparent causal swimlanes"
      className="flex min-h-0 min-w-0 flex-col gap-2.5 rounded-xl border border-[#D4AF37]/30 bg-[#0B0D12] p-3 text-slate-100 shadow-inner"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#D4AF37]/20 pb-2">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-display text-xs font-bold uppercase tracking-[0.12em] text-[#F3E5AB]">
              Perfetto W3C Traceparent Causal Swimlanes
            </span>
            <span className="rounded border border-[#D4AF37]/35 bg-[#D4AF37]/10 px-1.5 py-0.5 font-mono text-[9px] text-[#F3E5AB]">
              window {safeWindowMs}ms
            </span>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3 font-mono text-[10px] text-slate-300">
          <span className="inline-flex items-center gap-1">
            <span className="size-2 rounded-full bg-[#D4AF37]" aria-hidden="true" />
            <span>Planning</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="size-2 rounded-full bg-emerald-400" aria-hidden="true" />
            <span>Executing</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="size-2 rounded-full bg-amber-400" aria-hidden="true" />
            <span>Verifying</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="size-2 rounded-full bg-slate-400" aria-hidden="true" />
            <span>Paused</span>
          </span>
          <span className="inline-flex items-center gap-1">
            <span className="size-2 rounded-full bg-rose-400" aria-hidden="true" />
            <span>Validated</span>
          </span>
        </div>
      </div>

      <div className="relative space-y-2">
        {wakeupArcs.length > 0 && lanes.length > 0 && (
          <svg
            data-testid="concilium-wakeup-arcs"
            viewBox={`0 0 800 ${svgHeight}`}
            preserveAspectRatio="none"
            className="pointer-events-none absolute inset-0 z-10 h-full w-full overflow-visible"
            aria-label="@mention doorbell wakeup arcs"
          >
            <defs>
              <linearGradient id="concilium-wakeup-gold" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stopColor="#F3E5AB" stopOpacity="0.95" />
                <stop offset="100%" stopColor="#D4AF37" stopOpacity="0.85" />
              </linearGradient>
            </defs>
            {wakeupArcs.map((arc, idx) => {
              const targetIdx = Math.max(
                0,
                lanes.findIndex(
                  (l) =>
                    l.agent_id === arc.target_agent ||
                    l.roster_name === arc.target_agent.replace(/^@/, ''),
                ),
              );
              const startX = 250 + ((idx * 110) % 260);
              const startY = 12;
              const endX = 420 + ((idx * 95) % 300);
              const endY = Math.min(svgHeight - 10, 28 + targetIdx * 56);
              const ctrlX = (startX + endX) / 2;
              const ctrlY = Math.max(4, Math.min(startY, endY) - 18);
              const d = `M ${startX} ${startY} Q ${ctrlX} ${ctrlY} ${endX} ${endY}`;
              return (
                <g key={`${arc.from_seq}-${arc.target_span_id}-${idx}`}>
                  <path
                    d={d}
                    fill="none"
                    stroke="url(#concilium-wakeup-gold)"
                    strokeWidth="2"
                    strokeLinecap="round"
                  />
                  <circle cx={endX} cy={endY} r="3.5" fill="#F3E5AB" stroke="#D4AF37" strokeWidth="1.2" />
                  <title>{`#seq-${arc.from_seq} -> ${arc.mention} (${arc.target_span_id})`}</title>
                </g>
              );
            })}
          </svg>
        )}

        {lanes.length === 0 ? (
          <div className="rounded-lg border border-dashed border-slate-700 bg-slate-950/60 p-4 text-center font-mono text-xs text-slate-400">
            No active agent swimlanes.
          </div>
        ) : (
          lanes.map((lane) => {
          const isSelected = selectedAgentId === lane.agent_id;
          const tierBadge = formatComputeTierBadge(lane.compute_tier);
          const isParked = lane.status === 'parked' || lane.status === 'waiting' || lane.tok_per_sec === 0;

          return (
            <div
              key={lane.agent_id}
              role="button"
              tabIndex={0}
              data-slot="concilium-swimlane-row"
              data-testid={`swimlane-row-${lane.agent_id}`}
              aria-label={`${lane.roster_name} — ${lane.role}`}
              aria-pressed={isSelected}
              onClick={() => onSelectAgent(lane.agent_id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  onSelectAgent(lane.agent_id);
                }
              }}
              className={`group relative flex flex-col gap-2 rounded-lg border p-2.5 transition-colors cursor-pointer lg:flex-row lg:items-center ${
                isSelected
                  ? 'border-[#D4AF37] bg-[#D4AF37]/10 shadow-[0_0_12px_rgba(212,175,55,0.18)]'
                  : 'border-[#D4AF37]/25 bg-slate-950/85 hover:border-[#D4AF37]/60'
              }`}
            >
              <div className="flex w-full shrink-0 flex-wrap items-center justify-between gap-2 lg:w-72">
                <div className="min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="font-mono text-xs font-bold text-[#F3E5AB]">
                      @{lane.roster_name.replace(/^@/, '')}
                    </span>
                    <span className="font-mono text-[10px] text-slate-300">
                      {lane.roster_name}
                    </span>
                    <span className="rounded border border-[#D4AF37]/30 bg-slate-900 px-1.5 py-0.5 font-mono text-[9px] uppercase text-slate-300">
                      {lane.role}
                    </span>
                  </div>
                  <div className="mt-1 flex flex-wrap items-center gap-1.5 font-mono text-[9px]">
                    <span className={`rounded border px-1.5 py-0.5 font-bold ${tierBadge.className}`}>
                      {tierBadge.label}
                    </span>
                    <span className="rounded border border-slate-700 bg-slate-900 px-1.5 py-0.5 text-slate-300">
                      jj:{lane.workspace_mode}
                    </span>
                    {lane.file_lock_prefix && (
                      <span
                        data-slot="concilium-file-lock-shield"
                        className="inline-flex items-center gap-1 rounded border border-rose-500/45 bg-rose-950/40 px-1.5 py-0.5 text-rose-200"
                      >
                        <Icon.lockKeyhole className="size-2.5 text-rose-300" aria-hidden="true" />
                        <span>[FileLock] {lane.file_lock_prefix}</span>
                      </span>
                    )}
                  </div>
                </div>

                <div className="text-right font-mono text-[10px]">
                  {isParked ? (
                    <span className="inline-block rounded border border-dashed border-slate-500/70 bg-slate-900/90 px-1.5 py-0.5 text-slate-300">
                      0 tok/s · #seq-{lane.parked_since_seq ?? 0}
                    </span>
                  ) : (
                    <span className="inline-block rounded border border-emerald-500/40 bg-emerald-950/40 px-1.5 py-0.5 font-bold text-emerald-300">
                      {lane.tok_per_sec} tok/s
                    </span>
                  )}
                </div>
              </div>

              <div className="relative h-9 min-w-0 flex-1 overflow-hidden rounded-md border border-slate-800 bg-slate-900/90 p-1">
                {lane.spans.map((span) => {
                  const leftPct = Math.min(
                    92,
                    Math.max(0, (span.start_offset_ms / safeWindowMs) * 100),
                  );
                  const widthPct = Math.min(
                    100 - leftPct,
                    Math.max(8, (span.duration_ms / safeWindowMs) * 100),
                  );
                  return (
                    <div
                      key={span.span_id}
                      data-slot="concilium-span-bar"
                      data-span-kind={span.kind}
                      title={`${span.label} · traceparent: ${span.traceparent}`}
                      style={{ left: `${leftPct}%`, width: `${widthPct}%` }}
                      className={`absolute top-1 bottom-1 flex items-center justify-between gap-1 overflow-hidden rounded border px-2 font-mono text-[10px] ${spanToneClasses(
                        span.kind,
                      )}`}
                    >
                      <span className="truncate font-medium">{renderSpanLabel(span, lane)}</span>
                      {span.file_lock_prefix && (
                        <span className="shrink-0 rounded bg-rose-950/80 px-1 text-[8px] text-rose-200">
                          [FileLock] {span.file_lock_prefix}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })
        )}
      </div>
    </section>
  );
}
