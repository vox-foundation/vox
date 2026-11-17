import React from 'react';
import type {
  ComputeTier,
  DockedSwarmWorkerPill,
  HybridComputeNodeCardDto,
} from '../../../types/concilium';
import type { MeshNode } from '../Mesh/MeshView';
import { Icon } from '../../ui/Icons';

function toNodeStatus(raw: string): HybridComputeNodeCardDto['status'] {
  if (raw === 'maintenance' || raw === 'quarantined' || raw === 'degraded') return raw;
  return 'online';
}

export function synthesizeFallbackComputeNodes(meshNodes: MeshNode[]): HybridComputeNodeCardDto[] {
  const mapped: HybridComputeNodeCardDto[] = meshNodes.map((n) => ({
    node_id: n.id,
    display_name: n.id,
    tier: n.id.includes('local') ? 'local_mens_gpu' : 'populi_quic_peer',
    status: toNodeStatus(n.status),
    host_triple: n.host_triple ?? null,
    gpu_summary: n.gpu_summary ?? null,
    vram_used_mb: null,
    vram_total_mb: n.vram_gb != null ? n.vram_gb * 1024 : null,
    kv_cache_warm_prefixes: n.id.includes('local') ? 2 : null,
    rtt_ms: n.id.includes('local') ? 0 : 12,
    advertised_models: n.advertised_models ?? [],
    docked_workers: [],
    last_seen_unix_ms: n.last_seen_unix_ms ?? null,
  }));
  if (!mapped.some((n) => n.tier === 'local_mens_gpu')) {
    mapped.unshift({
      node_id: 'local-mens-gpu',
      display_name: 'Local Host MENS GPU',
      tier: 'local_mens_gpu',
      status: 'online',
      gpu_summary: 'Local MENS Runtime',
      vram_used_mb: 0,
      vram_total_mb: 24576,
      kv_cache_warm_prefixes: 0,
      advertised_models: [],
      docked_workers: [],
    });
  }
  if (!mapped.some((n) => n.tier === 'cloud_bandit')) {
    mapped.push({
      node_id: 'cloud-frontier-router',
      display_name: 'Cloud Frontier Router',
      tier: 'cloud_bandit',
      status: 'online',
      gpu_summary: 'Thompson Bandit Multi-Provider Router',
      bandit_weight: 0.5,
      advertised_models: [],
      docked_workers: [],
    });
  }
  return mapped;
}

export interface HybridComputeNodeCardProps {
  node: HybridComputeNodeCardDto;
  selectedWorkerId: string | null;
  selectedWorkerName?: string | null;
  migratingTargetNodeId?: string | null;
  isInspected?: boolean;
  onSelectWorker: (agentId: string) => void;
  onMigrateWorkerHere: (node: HybridComputeNodeCardDto) => void;
  onInspectNode?: (nodeId: string) => void;
}

function tierBadgeLabel(tier: ComputeTier): string {
  switch (tier) {
    case 'local_mens_gpu':
      return 'LOCAL HOST MENS GPU';
    case 'populi_quic_peer':
      return 'POPULI QUIC PEER (iroh)';
    case 'cloud_bandit':
      return 'CLOUD FRONTIER ROUTER';
    case 'inline_cpu':
      return 'INLINE CPU';
  }
}

function tierIcon(tier: ComputeTier): React.ReactNode {
  switch (tier) {
    case 'local_mens_gpu':
      return <Icon.cpu className="size-4 text-[#D4AF37]" aria-hidden="true" />;
    case 'populi_quic_peer':
      return <Icon.globe className="size-4 text-[#10B981]" aria-hidden="true" />;
    case 'cloud_bandit':
      return <Icon.spark className="size-4 text-[#F3E5AB]" aria-hidden="true" />;
    case 'inline_cpu':
      return <Icon.terminal className="size-4 text-slate-300" aria-hidden="true" />;
  }
}

function nodeStatusClasses(status: HybridComputeNodeCardDto['status']): string {
  switch (status) {
    case 'online':
      return 'border-[#10B981]/40 bg-[#10B981]/10 text-[#10B981]';
    case 'maintenance':
    case 'degraded':
      return 'border-[#D4AF37]/45 bg-[#D4AF37]/10 text-[#F3E5AB]';
    case 'quarantined':
      return 'border-[#F43F5E]/45 bg-[#F43F5E]/15 text-[#F43F5E]';
  }
}

function workerPillClasses(status: DockedSwarmWorkerPill['status'], selected: boolean): string {
  const ring = selected ? 'ring-1 ring-[#F3E5AB]' : '';
  switch (status) {
    case 'working':
      return `border-[#10B981]/45 bg-[#10B981]/12 text-emerald-200 ${ring}`;
    case 'parked':
      return `border-dashed border-[#64748B]/60 bg-[#64748B]/15 text-slate-300 ${ring}`;
    case 'tripped':
      return `border-[#F43F5E]/50 bg-[#F43F5E]/15 text-rose-200 ${ring}`;
    case 'waiting':
    case 'idle':
      return `border-[#D4AF37]/30 bg-[#D4AF37]/10 text-[#F3E5AB] ${ring}`;
  }
}

function formatVramGb(mb?: number | null): string {
  if (mb == null || !Number.isFinite(mb)) return 'n/a';
  return `${(mb / 1024).toFixed(1)} GB`;
}

export function HybridComputeNodeCard({
  node,
  selectedWorkerId,
  selectedWorkerName,
  migratingTargetNodeId,
  isInspected = false,
  onSelectWorker,
  onMigrateWorkerHere,
  onInspectNode,
}: HybridComputeNodeCardProps) {
  const vramPct =
    node.vram_total_mb && node.vram_total_mb > 0 && node.vram_used_mb != null
      ? Math.max(0, Math.min(100, Math.round((node.vram_used_mb / node.vram_total_mb) * 100)))
      : null;

  const isWorkerDockedHere = Boolean(
    selectedWorkerId && node.docked_workers.some((w) => w.agent_id === selectedWorkerId),
  );
  const isMigratingHere = migratingTargetNodeId === node.node_id;
  const canMigrate = Boolean(selectedWorkerId) && !isWorkerDockedHere && !migratingTargetNodeId;

  return (
    <article
      data-slot="populi-compute-node"
      data-testid={`populi-compute-node-${node.node_id}`}
      data-compute-tier={node.tier}
      className={`flex min-w-0 flex-col justify-between gap-3 rounded-xl border bg-[#0B0D12]/95 p-3.5 text-slate-100 shadow-sm transition-colors ${
        isInspected ? 'border-[#D4AF37] ring-1 ring-[#D4AF37]/40' : 'border-[#D4AF37]/25 hover:border-[#D4AF37]/50'
      }`}
    >
      <div className="space-y-2.5">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="flex items-center gap-1.5 font-display text-[10px] font-bold tracking-[0.14em] text-[#D4AF37] uppercase">
              {tierIcon(node.tier)}
              <span className="truncate">{tierBadgeLabel(node.tier)}</span>
            </div>
            <button
              type="button"
              data-slot="button"
              aria-pressed={isInspected}
              onClick={() => onInspectNode?.(node.node_id)}
              className="mt-0.5 text-left font-mono text-xs font-bold text-[#F3E5AB] hover:underline"
            >
              {node.display_name || node.node_id}
            </button>
          </div>
          <span
            className={`shrink-0 rounded-full border px-2 py-0.5 font-mono text-[10px] uppercase ${nodeStatusClasses(
              node.status,
            )}`}
          >
            {node.status}
          </span>
        </div>

        {/* Tier-specific telemetry */}
        <div className="rounded-lg border border-[#D4AF37]/15 bg-black/35 p-2.5 font-mono text-[11px]">
          {node.tier === 'local_mens_gpu' && (
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-slate-300">
                <span>{node.gpu_summary ?? 'Local Accelerator'}</span>
                <span className="text-[#F3E5AB]">
                  {formatVramGb(node.vram_used_mb)} / {formatVramGb(node.vram_total_mb)}
                </span>
              </div>
              {vramPct != null && (
                <div
                  role="progressbar"
                  aria-label={`${node.display_name} VRAM usage`}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={vramPct}
                  className="h-1.5 w-full overflow-hidden rounded-full bg-slate-800"
                >
                  <div
                    className="h-full rounded-full bg-gradient-to-r from-[#D4AF37] to-[#10B981]"
                    style={{ width: `${vramPct}%` }}
                  />
                </div>
              )}
              <div className="flex items-center justify-between text-[10px] text-slate-400">
                <span>Warm KV-Cache Prefixes</span>
                <span className="text-[#10B981]">{node.kv_cache_warm_prefixes ?? 0} warm</span>
              </div>
            </div>
          )}

          {node.tier === 'populi_quic_peer' && (
            <div className="space-y-1">
              <div className="flex items-center justify-between text-slate-300">
                <span className="truncate">{node.host_triple ?? 'triple unreported'}</span>
                <span className="shrink-0 text-[#10B981]">
                  {node.rtt_ms != null ? `${node.rtt_ms}ms RTT` : 'RTT n/a'}
                </span>
              </div>
              <div className="flex items-center justify-between text-[10px] text-slate-400">
                <span className="truncate">{node.gpu_summary ?? 'Peer compute'}</span>
                {node.vram_total_mb != null && (
                  <span className="shrink-0 text-[#F3E5AB]">{formatVramGb(node.vram_total_mb)}</span>
                )}
              </div>
            </div>
          )}

          {node.tier === 'cloud_bandit' && (
            <div className="space-y-1">
              <div className="flex items-center justify-between text-slate-300">
                <span>Thompson Bandit Arm</span>
                <span className="text-[#D4AF37]">
                  {node.bandit_weight != null
                    ? `${Math.round(node.bandit_weight * 100)}% weight`
                    : 'adaptive'}
                </span>
              </div>
              <div className="text-[10px] text-slate-400">
                {node.gpu_summary ?? 'Frontier router with fallback quorum'}
              </div>
            </div>
          )}

          {node.tier === 'inline_cpu' && (
            <div className="text-slate-300">
              {node.gpu_summary ?? node.host_triple ?? 'Inline orchestrator host'}
            </div>
          )}

          {node.advertised_models.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1 border-t border-white/5 pt-1.5">
              {node.advertised_models.map((model, idx) => (
                <span
                  key={`${model}-${idx}`}
                  className="rounded border border-[#D4AF37]/20 bg-[#D4AF37]/5 px-1.5 py-0.5 text-[10px] text-[#F3E5AB]"
                >
                  {model}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Docked @agent worker pills */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between font-display text-[10px] tracking-wider text-slate-400 uppercase">
            <span>Docked Swarm Workers</span>
            <span className="font-mono text-[#F3E5AB]">{node.docked_workers.length}</span>
          </div>
          {node.docked_workers.length === 0 ? (
            <div className="rounded-md border border-dashed border-slate-700/80 px-2.5 py-2 font-mono text-[10.5px] text-slate-500">
              No active @agent pills docked on this node
            </div>
          ) : (
            <div className="flex flex-col gap-1.5">
              {node.docked_workers.map((worker) => {
                const isSelected = selectedWorkerId === worker.agent_id;
                const throughputLabel =
                  worker.status === 'parked'
                    ? 'parked 0 tok/s'
                    : `${worker.status} ${worker.tok_per_sec.toFixed(1)} tok/s`;
                return (
                  <button
                    key={worker.agent_id}
                    type="button"
                    data-slot="docked-worker-pill"
                    data-testid={`docked-worker-pill-${worker.agent_id}`}
                    data-selected={isSelected ? 'true' : 'false'}
                    aria-pressed={isSelected}
                    onClick={() => onSelectWorker(worker.agent_id)}
                    className={`flex w-full flex-wrap items-center justify-between gap-1.5 rounded-lg border px-2.5 py-1.5 text-left font-mono text-[11px] transition-colors ${workerPillClasses(
                      worker.status,
                      isSelected,
                    )}`}
                  >
                    <span className="font-bold">@{worker.roster_name}</span>
                    <div className="flex flex-wrap items-center gap-1.5 text-[10px]">
                      <span>{throughputLabel}</span>
                      <span className="rounded border border-current/25 bg-black/30 px-1.5 py-0.5 uppercase">
                        {worker.workspace_mode}
                      </span>
                      {worker.file_lock_prefix && (
                        <span className="rounded border border-[#D4AF37]/30 bg-black/30 px-1.5 py-0.5 text-[#F3E5AB]">
                          {worker.file_lock_prefix}
                        </span>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Hot-swap target action */}
      <div className="flex items-center justify-between gap-2 border-t border-[#D4AF37]/15 pt-2.5">
        <span className="truncate font-mono text-[10px] text-slate-400">
          {selectedWorkerName ? `Target: @${selectedWorkerName}` : 'Select a worker pill to migrate'}
        </span>
        <button
          type="button"
          data-slot="button"
          data-testid={`migrate-worker-btn-${node.node_id}`}
          disabled={!canMigrate}
          onClick={() => onMigrateWorkerHere(node)}
          className="inline-flex shrink-0 items-center gap-1 rounded-md border border-[#D4AF37]/45 bg-[#D4AF37]/15 px-2.5 py-1 font-display text-[10px] font-bold tracking-wider text-[#F3E5AB] uppercase transition-colors hover:border-[#D4AF37] hover:bg-[#D4AF37]/25 disabled:cursor-not-allowed disabled:opacity-45"
        >
          <Icon.arrowLeftRight className="size-3" aria-hidden="true" />
          <span>
            {isMigratingHere
              ? 'Migrating...'
              : isWorkerDockedHere
                ? 'Docked Here'
                : 'Migrate Worker Here'}
          </span>
        </button>
      </div>
    </article>
  );
}
