import type React from 'react';
import type { MeshNode } from '../Mesh/MeshView';

export interface PopuliGaugeItem {
  label: string;
  valueText: string;
  ratio: number;
  tone: 'gold' | 'emerald' | 'crimson';
  active?: boolean;
  onClick?: () => void;
}

export function buildPopuliHeaderGauges(params: {
  entrySurface: 'populi' | 'mesh';
  nodes: MeshNode[];
  onlineNodes: MeshNode[];
  peerFilter: 'all' | 'online' | 'degraded';
  setPeerFilter: React.Dispatch<React.SetStateAction<'all' | 'online' | 'degraded'>>;
  effectivePending: number | null;
  sourceName: string | undefined;
  dispatchConfigured: boolean;
  onlineVramGb: number;
  totalVramGb: number;
  healthyProbes: number;
  failedProbes: number;
  refreshAll: () => void;
}): PopuliGaugeItem[] {
  const {
    entrySurface,
    nodes,
    onlineNodes,
    peerFilter,
    setPeerFilter,
    effectivePending,
    sourceName,
    dispatchConfigured,
    onlineVramGb,
    totalVramGb,
    healthyProbes,
    failedProbes,
    refreshAll,
  } = params;

  if (entrySurface === 'mesh') {
    return [
      {
        label: 'Nodes',
        valueText: `${nodes.length}`,
        ratio: nodes.length > 0 ? onlineNodes.length / nodes.length : 0,
        tone: 'gold',
        active: peerFilter === 'online',
        onClick: () => setPeerFilter((f) => (f === 'online' ? 'all' : 'online')),
      },
      {
        label: 'Pending',
        valueText: `${effectivePending ?? 0}`,
        ratio: Math.min(1, (effectivePending ?? 0) / 10),
        tone: 'emerald',
        onClick: refreshAll,
      },
      {
        label: 'Source',
        valueText: (sourceName ?? 'LOCAL').toUpperCase(),
        ratio: dispatchConfigured ? 1 : 0,
        tone: dispatchConfigured ? 'emerald' : 'gold',
        active: dispatchConfigured,
        onClick: () => document.getElementById('mesh-source')?.focus(),
      },
    ];
  }

  return [
    {
      label: 'Peers',
      valueText: `${onlineNodes.length}/${nodes.length} UP`,
      ratio: nodes.length > 0 ? onlineNodes.length / nodes.length : 0,
      tone: nodes.length > 0 && onlineNodes.length === nodes.length ? 'emerald' : 'gold',
      active: peerFilter === 'online',
      onClick: () => setPeerFilter((f) => (f === 'online' ? 'all' : 'online')),
    },
    {
      label: 'VRAM Pool',
      valueText: `${onlineVramGb} GB`,
      ratio: totalVramGb > 0 ? onlineVramGb / totalVramGb : 0,
      tone: 'gold',
      active: peerFilter === 'all',
      onClick: () => setPeerFilter('all'),
    },
    {
      label: 'Probes',
      valueText: `${healthyProbes}/2 OK`,
      ratio: healthyProbes / 2,
      tone: failedProbes > 0 ? 'crimson' : 'emerald',
      active: peerFilter === 'degraded',
      onClick: () => setPeerFilter((f) => (f === 'degraded' ? 'all' : 'degraded')),
    },
  ];
}
