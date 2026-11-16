import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invokeCmd } from '../../../transport';
import { Glass } from '../../ui/Glass';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import { BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import type { Toast } from '../../../types/tauri';
import { useMeshNodesFull } from '../../../hooks/useMeshNodes';

interface MeshViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
}

/** One node row as summarized by the `vox_mesh_nodes` MCP tool. */
export interface MeshNode {
  id: string;
  status: string;
  vram_gb?: number | null;
  host_triple?: string | null;
  gpu_summary?: string | null;
  trust_tier?: string | null;
  advertised_models?: string[];
  last_seen_unix_ms?: number;
}

interface QueueStatsResult {
  source?: string;
  control_plane_error?: string;
  pending_count?: number | null;
  pending_by_kind?: Record<string, number>;
  pending_by_priority?: Record<string, number>;
}

interface McpEnvelope<T> {
  tool: string;
  is_error: boolean;
  result: T;
}

const REFRESH_MS = 5000;

function formatLastSeen(ms?: number): string {
  if (!ms || !Number.isFinite(ms) || ms <= 0) return '—';
  const deltaSec = Math.round((Date.now() - ms) / 1000);
  if (deltaSec < 5) return 'just now';
  if (deltaSec < 60) return `${deltaSec}s ago`;
  if (deltaSec < 3600) return `${Math.floor(deltaSec / 60)}m ago`;
  if (deltaSec < 86400) return `${Math.floor(deltaSec / 3600)}h ago`;
  return `${Math.floor(deltaSec / 86400)}d ago`;
}

function statusTone(status: string): string {
  switch (status) {
    case 'online':
      return 'border-emerald-400/30 bg-emerald-400/10 text-emerald-300';
    case 'maintenance':
      return 'border-amber-400/30 bg-amber-400/10 text-amber-300';
    case 'quarantined':
      return 'border-rose-400/30 bg-rose-400/10 text-rose-300';
    default:
      return 'border-border-subtle bg-overlay-subtle text-text-secondary';
  }
}

export function MeshView({ pushToast, gamifyEnabled }: MeshViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [queue, setQueue] = useState<QueueStatsResult>({});

  // Dispatch form state.
  const [targetNode, setTargetNode] = useState<string>('');
  const [source, setSource] = useState<string>('');
  const [taskKind, setTaskKind] = useState<string>('');
  const [dispatching, setDispatching] = useState(false);
  const [dispatchResult, setDispatchResult] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'online'>('all');

  const onMeshNodesError = useCallback(
    (err: unknown) => {
      pushToast({ tone: 'warn', title: 'Mesh refresh failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    },
    [pushToast],
  );
  const {
    nodes,
    meta: nodesMeta,
    loading,
    refresh: refreshNodes,
  } = useMeshNodesFull(REFRESH_MS, { embedded, onError: onMeshNodesError });

  const refreshQueue = useCallback(async () => {
    try {
      const queueRes = await invokeCmd<McpEnvelope<QueueStatsResult>>('invoke_mcp_tool', {
        tool: 'vox_mesh_queue_stats',
        args: {},
      });
      setQueue(queueRes?.result ?? {});
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Mesh refresh failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  const refresh = useCallback(async () => {
    await Promise.all([refreshNodes(), refreshQueue()]);
  }, [refreshNodes, refreshQueue]);

  useEffect(() => {
    refreshQueue();
    // Embedded mini-render: one initial fetch only, no repeating poll.
    if (embedded) return;
    const id = setInterval(refreshQueue, REFRESH_MS);
    return () => clearInterval(id);
  }, [refreshQueue, embedded]);

  // Dispatch availability: the local-registry source means no control plane is
  // reachable, so dispatch (a write) cannot succeed and should be disabled.
  const dispatchConfigured = nodesMeta.source === 'control_plane';

  const pendingCount = useMemo(() => {
    if (typeof queue.pending_count === 'number') return queue.pending_count;
    if (typeof nodesMeta.queue_depth === 'number') return nodesMeta.queue_depth;
    return null;
  }, [queue.pending_count, nodesMeta.queue_depth]);

  const dispatch = useCallback(async () => {
    if (!source.trim()) {
      pushToast({ tone: 'warn', title: 'Dispatch needs source', body: 'Enter .vox source to run.', cause: 'validation' });
      return;
    }
    setDispatching(true);
    setDispatchResult('');
    try {
      const args: Record<string, unknown> = { source };
      if (targetNode) args.node_id = targetNode;
      if (taskKind.trim()) args.task_kind = taskKind.trim();
      const res = await invokeCmd<McpEnvelope<any>>('invoke_mcp_tool', {
        tool: 'vox_mesh_dispatch',
        args,
      });
      if (res?.is_error) {
        const msg = res?.result?.error ?? JSON.stringify(res?.result);
        setDispatchResult(sanitizeErrorForToast(msg));
        pushToast({ tone: 'warn', title: 'Dispatch failed', body: sanitizeErrorForToast(msg), cause: 'backend-error' });
      } else {
        const r = res?.result ?? {};
        const id = r.node_id ?? '(unknown node)';
        setDispatchResult(
          `node=${id} success=${r.success} exit=${r.exit_code ?? '—'} (${r.duration_ms ?? 0}ms)\n${r.output ?? ''}`,
        );
        pushToast({
          tone: r.success ? 'ok' : 'warn',
          title: r.success ? 'Dispatched' : 'Dispatch returned failure',
          body: `node ${id}`,
          cause: 'backend-ok',
        });
        if (r.success) {
          void recordGamifyGuiEvent(
            'mesh_dispatch_success',
            { node_id: id, task_kind: taskKind.trim() || null },
            { enabled: gamifyEnabled },
          );
        }
      }
      await refresh();
    } catch (err) {
      setDispatchResult(sanitizeErrorForToast(err));
      pushToast({ tone: 'warn', title: 'Dispatch failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setDispatching(false);
    }
  }, [source, targetNode, taskKind, pushToast, refresh, gamifyEnabled]);

  const onlineNodeCount = useMemo(
    () => nodes.filter((n) => n.status === 'online').length,
    [nodes],
  );
  const visibleNodes = useMemo(
    () => (statusFilter === 'online' ? nodes.filter((n) => n.status === 'online') : nodes),
    [nodes, statusFilter],
  );

  return (
    <div className="@container/surface flex min-w-0 flex-col gap-4">
      <RomanSurfaceHeader
        domainTag="AGENTS · MESH"
        title="Vox Populi Distributed Compute Mesh & Queue Dispatch"
        subtitle="PEER DISCOVERY · VRAM TELEMETRY · REMOTE WORKLOAD DISPATCH"
        waxSealBadge="MESH ACTIVE"
        icon={<Icon.cpu className="size-4 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'Nodes',
            valueText: `${nodes.length}`,
            ratio: nodes.length > 0 ? Math.max(0, Math.min(1, onlineNodeCount / nodes.length)) : 0,
            tone: 'gold',
            purpose: 'Registered mesh compute peers (online ratio)',
            actionHint: statusFilter === 'online' ? 'Show all mesh nodes' : 'Filter to online mesh nodes',
            active: statusFilter === 'online',
            onClick: () => setStatusFilter((prev) => (prev === 'online' ? 'all' : 'online')),
          },
          {
            label: 'Pending',
            valueText: `${pendingCount ?? 0}`,
            ratio: Math.max(0, Math.min(1, (pendingCount ?? 0) / Math.max(1, nodes.length * 4 || 10))),
            tone: 'emerald',
            purpose: 'Pending remote workloads in mesh queue',
            actionHint: 'Refresh mesh queue stats',
            onClick: () => {
              void refresh();
            },
          },
          {
            label: 'Source',
            valueText: (nodesMeta.source ?? 'LOCAL').toUpperCase(),
            ratio: dispatchConfigured ? 1 : 0,
            tone: dispatchConfigured ? 'emerald' : 'gold',
            purpose: 'Mesh discovery source and control-plane reachability',
            actionHint: 'Focus remote workload dispatch form',
            active: dispatchConfigured,
            onClick: () => {
              const el = document.getElementById('mesh-source');
              el?.scrollIntoView?.({ behavior: 'smooth', block: 'center' });
              el?.focus();
            },
          },
        ]}
      />
      {/* Summary / queue header */}
      <Glass className="p-4">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <span className="flex size-7 items-center justify-center rounded-lg bg-brass/10 text-brass ring-1 ring-brass/30">
            <Icon.cpu className="size-4" />
          </span>
          <div className="font-display text-sm tracking-widest uppercase text-text-secondary">
            Vox Populi Mesh
          </div>
          <span className="ml-1 rounded-full bg-overlay-subtle px-2 py-0.5 font-mono text-[10px] text-text-muted">
            {nodes.length} node{nodes.length === 1 ? '' : 's'}
          </span>
          <span className="rounded-full bg-overlay-subtle px-2 py-0.5 font-mono text-[10px] text-text-muted">
            source: {nodesMeta.source ?? '—'}
          </span>
          <span className="rounded-full bg-overlay-subtle px-2 py-0.5 font-mono text-[10px] text-text-muted">
            pending: {pendingCount ?? '—'}
          </span>
          <button
            type="button"
            onClick={refresh}
            className="ml-auto flex items-center gap-1.5 rounded-md border border-border-subtle bg-overlay-subtle px-3 py-1.5 font-display text-[11px] tracking-wider uppercase text-text-secondary transition-colors hover:bg-overlay-subtle"
          >
            <Icon.refresh aria-hidden="true" className="size-3.5" /> Refresh
          </button>
        </div>

        {nodesMeta.control_plane_error && (
          <div className="mb-2 flex items-start gap-2 rounded-md border border-amber-400/20 bg-amber-400/5 px-3 py-2 text-[11px] text-amber-300">
            <Icon.alert className="mt-0.5 size-3.5 shrink-0" />
            <span>
              Control plane unreachable ({nodesMeta.control_url}) — showing local registry.{' '}
              <span className="font-mono text-amber-200/80">{nodesMeta.control_plane_error}</span>
            </span>
          </div>
        )}

        {queue.pending_by_kind && Object.keys(queue.pending_by_kind).length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {Object.entries(queue.pending_by_kind).map(([kind, n]) => (
              <span
                key={kind}
                className="rounded-full bg-overlay-subtle px-2 py-0.5 font-mono text-[10px] text-text-muted"
              >
                {kind}: {n}
              </span>
            ))}
          </div>
        )}
      </Glass>

      <BentoGrid preset="hero" className="items-start">
        {/* Node table */}
        <Glass className="flex min-h-0 min-w-0 flex-col p-4 @3xl/surface:col-span-7 @5xl/surface:col-span-8">
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div className="font-display text-xs tracking-widest uppercase text-text-muted">Nodes</div>
            <span className="font-mono text-[10px] text-text-muted">{visibleNodes.length} visible</span>
          </div>
          <BentoScrollRegion
            ariaLabel="Mesh nodes"
            aria-live="polite"
            aria-busy={loading}
            className="max-h-[30rem]"
          >
            {loading && nodes.length === 0 ? (
              <div className="text-sm text-text-muted">Loading mesh nodes…</div>
            ) : nodes.length === 0 ? (
              <div className="flex flex-col items-center justify-center gap-3 py-12 text-center">
                <span className="flex size-12 items-center justify-center rounded-2xl bg-overlay-subtle text-text-muted ring-1 ring-white/10">
                  <Icon.cpu className="size-6" />
                </span>
                <div className="font-display text-sm tracking-wider text-text-secondary">No mesh nodes</div>
                <div className="text-[11px] leading-relaxed text-text-muted">
                  Join one with <code className="font-mono text-text-muted">vox populi join</code>, or
                  configure a control plane via{' '}
                  <code className="font-mono text-text-muted">VOX_ORCHESTRATOR_DAEMON_SOCKET</code> /{' '}
                  <code className="font-mono text-text-muted">VOX_ORCHESTRATOR_MESH_CONTROL_URL</code>.
                </div>
              </div>
            ) : (
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="text-[10px] uppercase tracking-wider text-text-muted">
                    <th scope="col" className="px-2 py-1.5 font-display">Node</th>
                    <th scope="col" className="px-2 py-1.5 font-display">Status</th>
                    <th scope="col" className="px-2 py-1.5 font-display">Host</th>
                    <th scope="col" className="px-2 py-1.5 font-display">GPU</th>
                    <th scope="col" className="px-2 py-1.5 font-display">Trust</th>
                    <th scope="col" className="px-2 py-1.5 font-display">Models</th>
                    <th scope="col" className="px-2 py-1.5 font-display">Last seen</th>
                  </tr>
                </thead>
                <tbody>
                  {visibleNodes.map((n) => (
                    <tr key={n.id} className="border-t border-border-subtle">
                      <td className="px-2 py-1.5 font-mono text-brass break-all">{n.id}</td>
                      <td className="px-2 py-1.5">
                        <span
                          className={`rounded-full border px-2 py-0.5 font-display text-[10px] tracking-wider uppercase ${statusTone(
                            n.status,
                          )}`}
                        >
                          {n.status}
                        </span>
                      </td>
                      <td className="px-2 py-1.5 font-mono text-[11px] text-text-muted">
                        {n.host_triple ?? '—'}
                      </td>
                      <td className="px-2 py-1.5 font-mono text-[11px] text-text-secondary">
                        {n.gpu_summary ?? (n.vram_gb ? `${n.vram_gb} GB VRAM` : '—')}
                      </td>
                      <td className="px-2 py-1.5 text-[11px] text-text-secondary">
                        {n.trust_tier ? (
                          <span className="rounded border border-brass/25 bg-brass/10 px-1.5 py-0.5 font-mono text-[10px] uppercase text-brass">
                            {n.trust_tier}
                          </span>
                        ) : (
                          '—'
                        )}
                      </td>
                      <td className="px-2 py-1.5 text-[11px] text-text-muted">
                        {n.advertised_models && n.advertised_models.length > 0 ? (
                          <div className="flex flex-wrap gap-1">
                            {n.advertised_models.map((m) => (
                              <span
                                key={m}
                                className="rounded border border-border-subtle bg-bg-base/70 px-1.5 py-0.5 font-mono text-[10px] text-text-secondary"
                              >
                                {m}
                              </span>
                            ))}
                          </div>
                        ) : (
                          '—'
                        )}
                      </td>
                      <td className="px-2 py-1.5 font-mono text-[11px] text-text-muted">
                        {formatLastSeen(n.last_seen_unix_ms)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </BentoScrollRegion>
        </Glass>

        {/* Dispatch form */}
        <Glass className="flex min-h-0 min-w-0 flex-col p-4 @3xl/surface:col-span-5 @5xl/surface:col-span-4">
          <div className="mb-3 flex items-center gap-2">
            <span className="flex size-7 items-center justify-center rounded-lg bg-brass/10 text-brass ring-1 ring-brass/30">
              <Icon.send className="size-4" />
            </span>
            <div className="font-display text-sm tracking-widest uppercase text-text-secondary">
              Dispatch Job
            </div>
          </div>

          {!dispatchConfigured && (
            <div className="mb-3 flex items-start gap-2 rounded-md border border-amber-400/20 bg-amber-400/5 px-3 py-2 text-[11px] text-amber-300">
              <Icon.alert className="mt-0.5 size-3.5 shrink-0" />
              <span>
                Dispatch is disabled: no populi control plane is reachable. Configure{' '}
                <code className="font-mono text-amber-200/80">VOX_ORCHESTRATOR_MESH_CONTROL_URL</code>{' '}
                (or <code className="font-mono text-amber-200/80">populi_control_url</code>) and rebuild
                with the <code className="font-mono text-amber-200/80">populi-transport</code> feature.
              </span>
            </div>
          )}

          <div className="grid grid-cols-12 gap-3">
            <div className="col-span-12 @xl/surface:col-span-6">
              <label htmlFor="mesh-target-node" className="mb-1 block font-display text-[10px] uppercase tracking-wider text-text-muted">
                Target node (optional)
              </label>
              <VoxSelect
                id="mesh-target-node"
                aria-label="Target node (optional)"
                value={targetNode}
                onValueChange={setTargetNode}
                disabled={!dispatchConfigured}
                options={[
                  { value: '', label: 'Auto (control plane picks)' },
                  ...nodes.map((n) => ({ value: n.id, label: n.id })),
                ]}
                className="w-full border-border-subtle bg-black/40 text-xs text-text-secondary"
              />
            </div>
            <div className="col-span-12 @xl/surface:col-span-6">
              <label htmlFor="mesh-task-kind" className="mb-1 block font-display text-[10px] uppercase tracking-wider text-text-muted">
                Task kind (optional)
              </label>
              <input id="mesh-task-kind"
                value={taskKind}
                onChange={(e) => setTaskKind(e.target.value)}
                disabled={!dispatchConfigured}
                placeholder="e.g. text_infer"
                className="w-full rounded-md border border-border-subtle bg-black/40 px-2 py-1.5 font-mono text-xs text-text-secondary disabled:opacity-40"
              />
            </div>
            <div className="col-span-12">
              <label htmlFor="mesh-source" className="mb-1 block font-display text-[10px] uppercase tracking-wider text-text-muted">
                Source (.vox)
              </label>
              <textarea id="mesh-source"
                value={source}
                onChange={(e) => setSource(e.target.value)}
                disabled={!dispatchConfigured}
                rows={5}
                placeholder={'fn main() {\n  print("hello from the mesh")\n}'}
                className="w-full rounded-md border border-border-subtle bg-black/40 p-2 font-mono text-xs text-text-secondary disabled:opacity-40"
              />
            </div>
          </div>

          <div className="mt-3 flex items-center gap-3">
            <button
              type="button"
              onClick={dispatch}
              disabled={!dispatchConfigured || dispatching || !source.trim()}
              className="flex items-center gap-1.5 rounded-md border border-brass/30 bg-brass/10 px-4 py-1.5 font-display text-[11px] tracking-wider uppercase text-brass transition-colors hover:bg-brass/20 disabled:cursor-not-allowed disabled:opacity-40"
            >
              <Icon.send aria-hidden="true" className="size-3.5" /> {dispatching ? 'Dispatching…' : 'Dispatch'}
            </button>
          </div>

          {dispatchResult && (
            <pre
              role="region"
              tabIndex={0}
              aria-label="Dispatch result"
              aria-live="polite"
              className="mt-3 max-h-64 overflow-auto rounded-md border border-border-subtle bg-black/40 p-3 text-[11px] text-text-secondary"
            >
              {dispatchResult}
            </pre>
          )}
        </Glass>
      </BentoGrid>
    </div>
  );
}
