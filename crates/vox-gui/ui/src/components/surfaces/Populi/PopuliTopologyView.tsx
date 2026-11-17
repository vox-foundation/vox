import React, { useState } from 'react';
import type { Toast } from '../../../types/tauri';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { MasterDetailWorkbench } from '../../ui/BentoGrid';
import { Icon } from '../../ui/Icons';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { FileAffinityLockMatrix } from './FileAffinityLockMatrix';
import { HybridComputeNodeCard } from './HybridComputeNodeCard';
import { PeerAttestationInspector } from './PeerAttestationInspector';
import { PopuliDispatchWorkbench } from './PopuliDispatchWorkbench';
import { PopuliPeerMasterPanel } from './PopuliPeerMasterPanel';
import { PopuliQuicTopologyRing } from './PopuliQuicTopologyRing';
import { buildPopuliHeaderGauges } from './populiHeaderGauges';
import { usePopuliTopologyData } from './usePopuliTopologyData';

export interface PopuliTopologyViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
  entrySurface?: 'populi' | 'mesh';
}

export function PopuliTopologyView({
  pushToast,
  gamifyEnabled,
  entrySurface = 'populi',
}: PopuliTopologyViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [peerFilter, setPeerFilter] = useState<'all' | 'online' | 'degraded'>('all');
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedWorkerId, setSelectedWorkerId] = useState<string | null>(null);

  const data = usePopuliTopologyData({
    embedded,
    entrySurface,
    pushToast,
    selectedNodeId,
    selectedWorkerId,
    peerFilter,
  });

  const headerGauges = buildPopuliHeaderGauges({
    entrySurface,
    nodes: data.nodes,
    onlineNodes: data.onlineNodes,
    peerFilter,
    setPeerFilter,
    effectivePending: data.effectivePending,
    sourceName: data.meta.source ?? data.placement?.source,
    dispatchConfigured: data.dispatchConfigured,
    onlineVramGb: data.onlineVramGb,
    totalVramGb: data.totalVramGb,
    healthyProbes: data.healthyProbes,
    failedProbes: data.failedProbes,
    refreshAll: data.refreshAll,
  });

  return (
    <section
      data-slot="populi-topology-view"
      data-testid="populi-mesh-bento-view"
      data-peer-filter={peerFilter}
      data-entry-surface={entrySurface}
      className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5"
    >
      <RomanSurfaceHeader
        domainTag={entrySurface === 'mesh' ? 'AGENTS · MESH' : 'COMPUTE · QUIC MESH'}
        title={entrySurface === 'mesh' ? 'Vox Populi Mesh' : 'Vox Populi'}
        subtitle="Hybrid compute placement, QUIC overlay attestation, and FileAffinityMap lock matrix"
        waxSealBadge={data.probing ? 'PROBING' : data.failedProbes > 0 ? 'ERROR' : 'QUORUM OK'}
        icon={<Icon.users className="size-5 text-brass" aria-hidden="true" />}
        gauges={headerGauges}
        actions={
          <button
            type="button"
            data-slot="button"
            data-testid="populi-refresh-btn"
            onClick={data.refreshAll}
            className="inline-flex items-center gap-1.5 rounded-md border border-border-strong bg-bg-elevated px-2.5 py-1 font-display text-[10px] font-bold tracking-wider text-text-primary uppercase hover:border-brass"
          >
            <Icon.refresh className="size-3" aria-hidden="true" />
            <span>Refresh</span>
          </button>
        }
      />

      <div
        data-testid="populi-hybrid-compute-grid"
        className="grid grid-cols-1 gap-3 @3xl/surface:grid-cols-3"
      >
        {data.computeNodes.map((cNode) => (
          <HybridComputeNodeCard
            key={cNode.node_id}
            node={cNode}
            selectedWorkerId={data.activeWorker?.agent_id ?? null}
            selectedWorkerName={data.activeWorker?.roster_name ?? null}
            migratingTargetNodeId={data.migratingNodeId}
            isInspected={data.inspectedNode?.id === cNode.node_id}
            onSelectWorker={setSelectedWorkerId}
            onMigrateWorkerHere={(target) => void data.handleMigrateWorkerHere(target)}
            onInspectNode={setSelectedNodeId}
          />
        ))}
      </div>

      <FileAffinityLockMatrix
        locks={data.placement?.lock_matrix ?? []}
        headSeq={data.placement?.head_seq}
      />

      <PopuliQuicTopologyRing
        visibleNodes={data.visibleNodes}
        inspectedNodeId={data.inspectedNode?.id ?? null}
        sourceLabel={(data.meta.source ?? data.placement?.source ?? 'LOCAL').toUpperCase()}
        gossipCadenceMs={data.gossipCadenceMs}
        onSelectNode={setSelectedNodeId}
      />

      <MasterDetailWorkbench
        storageKey="vox.layout.populi.v4"
        detailTitle="Peer Attestation Inspector"
        detailDescription="Ed25519 peer identity, GPU capacity, advertised models, and local registry snapshot."
        hasActiveDetail={Boolean(selectedNodeId)}
        master={
          <PopuliPeerMasterPanel
            peerFilter={peerFilter}
            onPeerFilterChange={setPeerFilter}
            nodesCount={data.nodes.length}
            visibleNodes={data.visibleNodes}
            inspectedNodeId={data.inspectedNode?.id ?? null}
            effectivePending={data.effectivePending}
            loading={data.loading}
            onSelectNode={setSelectedNodeId}
          />
        }
        detail={<PeerAttestationInspector inspectedNode={data.inspectedNode} probes={data.probes} />}
      />

      <PopuliDispatchWorkbench
        nodes={data.nodes}
        dispatchConfigured={data.dispatchConfigured}
        gossipCadenceMs={data.gossipCadenceMs}
        probing={data.probing}
        probes={data.probes}
        gamifyEnabled={gamifyEnabled}
        pushToast={pushToast}
        onCadenceChange={data.setGossipCadenceMs}
        onCadenceCommit={data.commitCadence}
        onRunProbes={() => data.runOverlayProbes(true)}
        onAfterDispatch={data.refreshAll}
      />
    </section>
  );
}
