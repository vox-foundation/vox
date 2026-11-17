import React from 'react';
import { DiegeticFxStage } from '../../fx/DiegeticFxStage';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import type { MeshNode } from '../Mesh/MeshView';

export interface PopuliQuicTopologyRingProps {
  visibleNodes: MeshNode[];
  inspectedNodeId: string | null;
  sourceLabel: string;
  gossipCadenceMs: number;
  onSelectNode: (nodeId: string) => void;
}

function computePeerPosition(index: number, total: number): { x: number; y: number } {
  if (total === 1) return { x: 615, y: 82 };
  if (total === 2) return index === 0 ? { x: 145, y: 82 } : { x: 615, y: 82 };
  const angle = -Math.PI / 2 + (index / total) * Math.PI * 2;
  return {
    x: Math.round(380 + Math.cos(angle) * 245),
    y: Math.round(82 + Math.sin(angle) * 56),
  };
}

export function PopuliQuicTopologyRing({
  visibleNodes,
  inspectedNodeId,
  sourceLabel,
  gossipCadenceMs,
  onSelectNode,
}: PopuliQuicTopologyRingProps) {
  return (
    <div
      data-testid="populi-topology-stage"
      className="relative w-full overflow-hidden rounded-xl border-2 border-border-strong bg-bg-elevated p-3 shadow-md"
    >
      <div
        className="pointer-events-none absolute inset-0 bg-cover bg-center opacity-75"
        style={{ backgroundImage: "url('/assets/romanesque/aqueduct-sluice-cascade-bg.jpg')" }}
      />
      <DiegeticFxStage
        preset="sunlit-colonnade"
        activeBoost={1.25}
        intensity={1.15}
        testId="populi-fx-sunlit-colonnade"
      />
      <div className="relative z-10 mb-1.5 flex flex-wrap items-center justify-between gap-2">
        <span className="rounded border border-border-strong bg-bg-surface/90 px-2.5 py-0.5 font-display text-[9.5px] font-bold tracking-[0.14em] text-text-primary uppercase">
          QUIC + TLS 1.3 OVERLAY RING - SOURCE {sourceLabel}
        </span>
        <RomanWaxSealBadge label={`GOSSIP ${gossipCadenceMs}MS`} tone="emerald" />
      </div>
      <div className="relative z-10 w-full">
        <svg
          data-cv-chart="true"
          data-testid="populi-quic-topology-svg"
          viewBox="0 0 760 164"
          className="block h-40 w-full select-none"
          role="group"
          aria-label="Populi QUIC mesh topology"
        >
          <ellipse
            cx="380"
            cy="82"
            rx="245"
            ry="56"
            fill="none"
            stroke="var(--color-border-strong)"
            strokeOpacity="0.55"
            strokeWidth="1.4"
            strokeDasharray="5 4"
          />
          {visibleNodes.map((peer, idx) => {
            const pos = computePeerPosition(idx, visibleNodes.length);
            const isSelected = inspectedNodeId === peer.id;
            const isOnline = peer.status === 'online';
            return (
              <g key={peer.id}>
                <line
                  x1="380"
                  y1="82"
                  x2={pos.x}
                  y2={pos.y}
                  stroke={isOnline ? 'var(--color-primary)' : 'var(--color-status-fail)'}
                  strokeWidth={isSelected ? 2.6 : 1.7}
                  strokeDasharray={isOnline ? undefined : '4 3'}
                />
                <g
                  role="button"
                  tabIndex={0}
                  data-testid={`populi-topology-node-${peer.id}`}
                  aria-label={`Inspect mesh peer ${peer.id}`}
                  onClick={() => onSelectNode(peer.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      onSelectNode(peer.id);
                    }
                  }}
                  className="cursor-pointer"
                >
                  <rect
                    x={pos.x - 58}
                    y={pos.y - 20}
                    width="116"
                    height="40"
                    rx="8"
                    fill="var(--color-bg-surface)"
                    stroke={
                      isSelected
                        ? 'var(--color-text-primary)'
                        : isOnline
                          ? 'var(--color-primary)'
                          : 'var(--color-status-fail)'
                    }
                    strokeWidth={isSelected ? 2.4 : 1.6}
                  />
                  <text
                    x={pos.x}
                    y={pos.y - 3}
                    textAnchor="middle"
                    fill="var(--color-text-primary)"
                    fontSize="11"
                    fontWeight="700"
                  >
                    {peer.id}
                  </text>
                  <text
                    x={pos.x}
                    y={pos.y + 12}
                    textAnchor="middle"
                    fill="var(--color-text-secondary)"
                    fontSize="9.5"
                  >
                    {peer.vram_gb ? `${peer.vram_gb}GB` : peer.status} -{' '}
                    {peer.trust_tier ?? 'peer'}
                  </text>
                </g>
              </g>
            );
          })}
          <g>
            <circle
              cx="380"
              cy="82"
              r="30"
              fill="var(--color-bg-surface)"
              stroke="var(--color-primary)"
              strokeWidth="2.4"
            />
            <circle
              cx="380"
              cy="82"
              r="24"
              fill="var(--color-bg-elevated)"
              stroke="var(--color-border-strong)"
              strokeWidth="1.2"
            />
            <text
              x="380"
              y="79"
              textAnchor="middle"
              fill="var(--color-text-primary)"
              fontSize="9.5"
              fontWeight="700"
            >
              LOCAL HUB
            </text>
            <text
              x="380"
              y="92"
              textAnchor="middle"
              fill="var(--color-text-secondary)"
              fontSize="8.5"
            >
              {gossipCadenceMs}ms
            </text>
          </g>
        </svg>
      </div>
    </div>
  );
}
