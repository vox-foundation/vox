import React, { useEffect, useState } from 'react';
import type {
  ComputeTier,
  ConciliumAgentLane,
  ConciliumPilotActionInput,
} from '../../../types/concilium';
import type { Agent } from '../../../types/dashboard';
import { VoxSelect, type VoxSelectOption } from '../../ui/Select';
import { Icon } from '../../ui/Icons';

export interface LiveAgentLensProps {
  lane: ConciliumAgentLane | null;
  legacyAgent?: Agent | null;
  lastActionReceipt?: string | null;
  onPilotAction: (input: ConciliumPilotActionInput) => Promise<void> | void;
  children?: React.ReactNode;
}

const COMPUTE_TIER_OPTIONS: readonly VoxSelectOption[] = [
  { value: 'cloud_bandit', label: 'Cloud Bandit (Frontier Router)' },
  { value: 'local_mens_gpu', label: 'Local MENS GPU (Warm KV)' },
  { value: 'populi_quic_peer', label: 'Populi QUIC Peer (iroh Mesh)' },
  { value: 'inline_cpu', label: 'Inline CPU Sandbox' },
];

export function LiveAgentLens({
  lane,
  legacyAgent,
  lastActionReceipt,
  onPilotAction,
  children,
}: LiveAgentLensProps) {
  const [steerDirective, setSteerDirective] = useState('');
  const laneAgentId = lane?.agent_id ?? null;

  useEffect(() => {
    setSteerDirective('');
  }, [laneAgentId]);

  if (!lane) {
    return (
      <aside
        data-slot="concilium-agent-lens"
        data-testid="concilium-agent-lens"
        aria-label="Live Agent Lens inspector"
        className="flex flex-col gap-3 rounded-xl border border-[#D4AF37]/30 bg-[#0B0D12] p-3.5 text-slate-200"
      >
        <p className="font-mono text-xs text-slate-400">
          Select a worker swimlane to inspect live W3C traceparent spans and pilot controls.
        </p>
      </aside>
    );
  }

  const tokenRatio =
    lane.max_tokens > 0 ? Math.min(100, Math.round((lane.used_tokens / lane.max_tokens) * 100)) : 0;

  const handleSteerClick = () => {
    const directive =
      steerDirective.trim() || `Prioritize current slice on ${lane.file_lock_prefix ?? 'workspace'}`;
    void onPilotAction({
      action: 'steer_agent',
      agent_id: lane.agent_id,
      channel: '#main',
      directive,
    });
    setSteerDirective('');
  };

  return (
    <aside
      data-slot="concilium-agent-lens"
      data-testid="concilium-agent-lens"
      aria-label="Live Agent Lens inspector"
      className="flex flex-col gap-3 rounded-xl border border-[#D4AF37]/35 bg-[#0B0D12] p-3.5 text-slate-100 shadow-lg"
    >
      <div className="flex flex-wrap items-start justify-between gap-2 border-b border-[#D4AF37]/20 pb-2.5">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="rounded border border-[#D4AF37]/45 bg-[#D4AF37]/15 px-1.5 py-0.5 font-display text-[9px] font-bold uppercase tracking-widest text-[#F3E5AB]">
              LIVE AGENT LENS
            </span>
            <h3 className="truncate font-mono text-sm font-bold text-[#F3E5AB]">
              {lane.roster_name}
            </h3>
            <span className="font-mono text-[10px] text-slate-400">({lane.agent_id})</span>
          </div>
          <p className="mt-1 font-mono text-[11px] text-slate-300">{lane.current_action}</p>
        </div>

        <div className="flex flex-wrap items-center gap-1.5 font-mono text-[10px]">
          <span className="rounded border border-[#D4AF37]/35 bg-slate-900 px-2 py-0.5 uppercase text-[#F3E5AB]">
            {lane.status}
          </span>
          <span className="rounded border border-emerald-500/35 bg-emerald-950/40 px-2 py-0.5 text-emerald-300">
            {lane.tok_per_sec} tok/s
          </span>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-2 font-mono text-[10px] sm:grid-cols-4">
        <div className="rounded-lg border border-slate-800 bg-slate-950/90 p-2">
          <div className="text-[9px] uppercase text-slate-400">Model &amp; Node</div>
          <div className="mt-0.5 truncate font-bold text-slate-100">{lane.model_id}</div>
          <div className="truncate text-[9px] text-slate-400">{lane.node_id}</div>
        </div>
        <div className="rounded-lg border border-slate-800 bg-slate-950/90 p-2">
          <div className="text-[9px] uppercase text-slate-400">Workspace &amp; Lock</div>
          <div className="mt-0.5 font-bold text-[#F3E5AB]">jj {lane.workspace_mode}</div>
          <div className="truncate text-[9px] text-rose-300">
            {lane.file_lock_prefix ?? 'no exclusive lock'}
          </div>
        </div>
        <div className="rounded-lg border border-slate-800 bg-slate-950/90 p-2">
          <div className="text-[9px] uppercase text-slate-400">Token Window</div>
          <div className="mt-0.5 font-bold text-slate-100">
            {lane.used_tokens.toLocaleString()} / {lane.max_tokens.toLocaleString()}
          </div>
          <div className="text-[9px] text-emerald-300">{tokenRatio}% utilized</div>
        </div>
        <div className="rounded-lg border border-slate-800 bg-slate-950/90 p-2">
          <div className="text-[9px] uppercase text-slate-400">KV-Cache Prefix</div>
          <div className="mt-0.5 font-bold text-emerald-300">
            {lane.kv_cache_hit_pct != null ? `${lane.kv_cache_hit_pct}% warm` : 'N/A'}
          </div>
          <div className="text-[9px] text-slate-400">
            {lane.parked_since_seq != null ? `Parked #seq-${lane.parked_since_seq}` : 'Active stream'}
          </div>
        </div>
      </div>

      {legacyAgent && (
        <div className="rounded-lg border border-[#D4AF37]/25 bg-slate-950/80 p-2.5 font-mono text-[10px]">
          <div className="grid grid-cols-4 gap-2">
            <div>
              <div className="text-[8.5px] uppercase text-slate-400">Cost</div>
              <div className="font-bold text-slate-100">${(legacyAgent.cost ?? 0).toFixed(2)}</div>
            </div>
            <div>
              <div className="text-[8.5px] uppercase text-slate-400">Budget</div>
              <div className="font-bold text-slate-100">
                {legacyAgent.budget != null ? `$${legacyAgent.budget.toFixed(2)}` : '—'}
              </div>
            </div>
            <div>
              <div className="text-[8.5px] uppercase text-slate-400">ETA</div>
              <div className="font-bold text-slate-100">{legacyAgent.eta}</div>
            </div>
            <div>
              <div className="text-[8.5px] uppercase text-slate-400">Skill</div>
              <div className="font-bold text-slate-100">{legacyAgent.skill ?? '—'}</div>
            </div>
          </div>
          <div className="mt-2">
            <div className="flex items-center justify-between text-[9px] uppercase text-slate-400">
              <span>Progress</span>
              <span data-testid="agent-inspector-progress" className="font-bold text-[#F3E5AB]">
                {legacyAgent.progress != null
                  ? `${Math.max(0, Math.min(100, Math.round(legacyAgent.progress * 100)))}%`
                  : '…'}
              </span>
            </div>
            {legacyAgent.progress != null && (() => {
              const pct = Math.max(0, Math.min(100, Math.round(legacyAgent.progress * 100)));
              return (
                <div
                  role="progressbar"
                  aria-label={`${lane.roster_name} progress`}
                  aria-valuenow={pct}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  className="mt-1 h-1.5 overflow-hidden rounded-full bg-slate-800"
                >
                  <div className="h-full bg-[#D4AF37]" style={{ width: `${pct}%` }} />
                </div>
              );
            })()}
          </div>
        </div>
      )}

      <div
        data-slot="concilium-pilot-controls"
        className="space-y-2.5 rounded-lg border border-[#D4AF37]/30 bg-slate-950/90 p-2.5"
      >
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-display text-[10px] font-bold uppercase tracking-widest text-[#D4AF37]">
            Operator Pilot Controls (4 Actions)
          </span>
          {lastActionReceipt && (
            <span
              data-testid="concilium-pilot-receipt"
              className="rounded border border-emerald-500/40 bg-emerald-950/50 px-2 py-0.5 font-mono text-[10px] text-emerald-300"
            >
              {lastActionReceipt}
            </span>
          )}
        </div>

        <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            data-testid="pilot-park-agent"
            onClick={() =>
              void onPilotAction({
                action: 'park_agent',
                agent_id: lane.agent_id,
              })
            }
            className="inline-flex items-center justify-center gap-1.5 rounded-md border border-slate-600 bg-slate-900 px-2.5 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-slate-200 hover:border-[#D4AF37] hover:text-[#F3E5AB]"
          >
            <Icon.pause className="size-3 text-slate-300" aria-hidden="true" />
            <span>Park (0 tok/s)</span>
          </button>

          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            data-testid="pilot-fork-agent"
            onClick={() =>
              void onPilotAction({
                action: 'fork_agent',
                agent_id: lane.agent_id,
                fork_name: `${lane.roster_name.replace(/^@/, '')}-fork`,
                workspace_mode: 'share',
              })
            }
            className="inline-flex items-center justify-center gap-1.5 rounded-md border border-emerald-500/50 bg-emerald-950/40 px-2.5 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-emerald-200 hover:border-emerald-400"
          >
            <Icon.branch className="size-3 text-emerald-300" aria-hidden="true" />
            <span>Fork Warm Context (jj share)</span>
          </button>

          <div className="flex items-center gap-1.5">
            <VoxSelect
              aria-label="Swap Compute Tier"
              data-testid="pilot-swap-compute-select"
              value={lane.compute_tier}
              options={COMPUTE_TIER_OPTIONS}
              size="xs"
              onValueChange={(nextTier) =>
                void onPilotAction({
                  action: 'swap_agent_compute',
                  agent_id: lane.agent_id,
                  target_tier: nextTier as ComputeTier,
                })
              }
              className="border-[#D4AF37]/40 bg-slate-900 font-mono text-[10px] text-[#F3E5AB]"
            />
          </div>
        </div>

        <div className="flex items-center gap-2">
          <input
            type="text"
            data-slot="input"
            data-testid="pilot-steer-input"
            aria-label={`Steer @${lane.roster_name.replace(/^@/, '')}`}
            value={steerDirective}
            onChange={(e) => setSteerDirective(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                handleSteerClick();
              }
            }}
            placeholder={`Steer @${lane.roster_name.replace(/^@/, '')} directive...`}
            className="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-900 px-2.5 py-1 font-mono text-[11px] text-slate-100 placeholder:text-slate-500 focus:border-[#D4AF37] focus:outline-hidden"
          />
          <button
            type="button"
            tabIndex={0}
            data-slot="button"
            data-testid="pilot-steer-agent"
            onClick={handleSteerClick}
            className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-[#D4AF37]/60 bg-[#D4AF37]/20 px-2.5 py-1 font-display text-[10px] font-bold uppercase tracking-wider text-[#F3E5AB] hover:bg-[#D4AF37]/30"
          >
            <Icon.send className="size-3" aria-hidden="true" />
            <span>Steer @agent</span>
          </button>
        </div>
      </div>

      {children && (
        <div className="border-t border-[#D4AF37]/20 pt-2.5 font-mono text-xs">{children}</div>
      )}
    </aside>
  );
}
