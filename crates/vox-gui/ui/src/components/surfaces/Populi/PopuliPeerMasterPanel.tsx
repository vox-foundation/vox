import React from 'react';
import { BentoScrollRegion } from '../../ui/BentoGrid';
import { Icon } from '../../ui/Icons';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { RomanMedallion } from '../../ui/RomanTooltip';
import type { MeshNode } from '../Mesh/MeshView';
import { formatPeerLastSeen } from './PeerAttestationInspector';

export interface PopuliPeerMasterPanelProps {
  peerFilter: 'all' | 'online' | 'degraded';
  onPeerFilterChange: (next: 'all' | 'online' | 'degraded') => void;
  nodesCount: number;
  visibleNodes: MeshNode[];
  inspectedNodeId: string | null;
  effectivePending: number | null;
  loading: boolean;
  onSelectNode: (nodeId: string) => void;
}

export function PopuliPeerMasterPanel({
  peerFilter,
  onPeerFilterChange,
  nodesCount,
  visibleNodes,
  inspectedNodeId,
  effectivePending,
  loading,
  onSelectNode,
}: PopuliPeerMasterPanelProps) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-elevated/55 p-2">
        <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label="Peer status filter">
          {(['all', 'online', 'degraded'] as const).map((f) => (
            <button
              key={f}
              type="button"
              data-slot="button"
              data-testid={`populi-filter-${f}`}
              aria-pressed={peerFilter === f}
              onClick={() => onPeerFilterChange(f)}
              className={`rounded-md border px-2.5 py-1 font-display text-[10px] tracking-wider uppercase transition-colors ${
                peerFilter === f
                  ? 'border-brass bg-brass/20 text-text-primary'
                  : 'border-border-subtle bg-bg-surface text-text-secondary hover:border-brass/45'
              }`}
            >
              {f}
            </button>
          ))}
        </div>
        <span className="font-mono text-[10px] text-text-muted">
          {visibleNodes.length} visible | pending: {effectivePending ?? 'n/a'}
        </span>
      </div>
      <BentoScrollRegion ariaLabel="Populi mesh peer matrix" className="max-h-56 flex-1 pr-1">
        {loading && nodesCount === 0 ? (
          <div className="rounded-lg border border-border-subtle bg-bg-elevated/40 p-4 text-xs text-text-muted">
            Discovering QUIC mesh peers...
          </div>
        ) : visibleNodes.length === 0 ? (
          <div
            data-testid="populi-empty-peers"
            className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border-subtle bg-bg-elevated/35 px-4 py-6 text-center"
          >
            <RomanMedallion size="sm">
              <Icon.users className="size-3.5 text-text-primary" aria-hidden="true" />
            </RomanMedallion>
            <div className="font-display text-xs font-bold tracking-wider text-text-primary uppercase">
              No Matching Mesh Peers
            </div>
            <p className="max-w-xs text-[11px] text-text-muted">
              Adjust the status filter or enroll a peer with vox populi join.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {visibleNodes.map((node) => (
              <button
                key={node.id}
                type="button"
                data-slot="button"
                data-testid={`populi-peer-row-${node.id}`}
                aria-pressed={inspectedNodeId === node.id}
                onClick={() => onSelectNode(node.id)}
                className={`flex w-full flex-wrap items-center justify-between gap-2 rounded-lg border p-2.5 text-left transition-colors ${
                  inspectedNodeId === node.id
                    ? 'border-brass bg-brass/12'
                    : 'border-border-subtle bg-bg-elevated/45 hover:border-brass/45'
                }`}
              >
                <div className="min-w-0">
                  <div className="font-mono text-xs font-bold text-text-primary">{node.id}</div>
                  <div className="font-mono text-[10.5px] text-text-muted">
                    {node.gpu_summary ?? node.host_triple ?? 'host unreported'}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {node.vram_gb != null && (
                    <span className="rounded border border-border-subtle bg-bg-surface px-2 py-0.5 font-mono text-[10px] text-text-secondary">
                      {node.vram_gb} GB
                    </span>
                  )}
                  <RomanWaxSealBadge
                    label={node.status}
                    tone={node.status === 'online' ? 'emerald' : 'crimson'}
                  />
                </div>
              </button>
            ))}
          </div>
        )}
      </BentoScrollRegion>
      <BentoScrollRegion
        ariaLabel="Mesh nodes"
        aria-live="polite"
        aria-busy={loading}
        className="max-h-40"
      >
        <table className="w-full text-left text-xs">
          <thead>
            <tr className="text-[10px] tracking-wider text-text-muted uppercase">
              <th scope="col" className="px-2 py-1 font-display">
                Node
              </th>
              <th scope="col" className="px-2 py-1 font-display">
                Status
              </th>
              <th scope="col" className="px-2 py-1 font-display">
                Host
              </th>
              <th scope="col" className="px-2 py-1 font-display">
                GPU
              </th>
              <th scope="col" className="px-2 py-1 font-display">
                Last seen
              </th>
            </tr>
          </thead>
          <tbody>
            {visibleNodes.map((n) => (
              <tr key={n.id} className="border-t border-border-subtle">
                <td className="px-2 py-1 font-mono text-brass">{n.id}</td>
                <td className="px-2 py-1 font-mono text-[11px]">{n.status}</td>
                <td className="px-2 py-1 font-mono text-[11px] text-text-muted">
                  {n.host_triple ?? 'n/a'}
                </td>
                <td className="px-2 py-1 font-mono text-[11px] text-text-secondary">
                  {n.gpu_summary ?? 'n/a'}
                </td>
                <td className="px-2 py-1 font-mono text-[11px] text-text-muted">
                  {formatPeerLastSeen(n.last_seen_unix_ms)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </BentoScrollRegion>
    </div>
  );
}
