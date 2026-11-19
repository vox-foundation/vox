import React, { useMemo, useState, type JSX } from 'react';
import { Icon } from '../../ui/Icons';
import { SafeExternalLink } from './SafeExternalLink';

export interface DagNode {
  id: string;
  label: string;
  wave: number;
  status: 'pending' | 'verified' | 'contradicted';
  kind?: 'query' | 'subquery' | 'source_domain' | 'claim' | 'sandbox_probe' | string;
  verdict?: string | null;
  confidence?: number | null;
  url?: string | null;
  snippet?: string | null;
  excerpt?: string | null;
}

export interface DagEdge {
  srcId: string;
  dstId: string;
  relation: 'supports' | 'contradicts';
  kind?: string;
}

export interface ResearchDagCanvasProps {
  nodes: DagNode[];
  edges: DagEdge[];
  className?: string;
}

export const CLUSTER_THRESHOLD = 80;
const NODE_WIDTH = 180;
const NODE_HEIGHT = 60;
const WAVE_GAP_X = 240;
const NODE_GAP_Y = 85;
const PADDING_X = 40;
const PADDING_Y = 40;

const STATUS_CONFIG = {
  verified: {
    bg: 'var(--color-bg-surface)',
    border: 'var(--color-status-pass)',
    badgeBg: 'var(--color-bg-elevated)',
    badgeText: 'var(--color-status-pass)',
    label: 'Verified',
  },
  contradicted: {
    bg: 'var(--color-bg-surface)',
    border: 'var(--color-status-fail)',
    badgeBg: 'var(--color-bg-elevated)',
    badgeText: 'var(--color-status-fail)',
    label: 'Contradicted',
  },
  pending: {
    bg: 'var(--color-bg-surface)',
    border: 'var(--color-border-strong)',
    badgeBg: 'var(--color-bg-elevated)',
    badgeText: 'var(--color-text-secondary)',
    label: 'Pending',
  },
} as const;

const KIND_CONFIG: Record<string, { bg: string; border: string; badgeBg: string; badgeText: string; label: string }> = {
  query: {
    bg: 'var(--color-bg-elevated)',
    border: 'var(--color-accent-default)',
    badgeBg: 'var(--color-bg-surface)',
    badgeText: 'var(--color-text-primary)',
    label: 'Query',
  },
  subquery: {
    bg: 'var(--color-bg-surface)',
    border: 'var(--color-accent-default)',
    badgeBg: 'var(--color-bg-elevated)',
    badgeText: 'var(--color-accent-default)',
    label: 'Subquery',
  },
  source_domain: {
    bg: 'var(--color-bg-surface)',
    border: 'var(--color-status-info)',
    badgeBg: 'var(--color-bg-elevated)',
    badgeText: 'var(--color-status-info)',
    label: 'Source',
  },
  sandbox_probe: {
    bg: 'var(--color-bg-surface)',
    border: 'var(--color-status-warn)',
    badgeBg: 'var(--color-bg-elevated)',
    badgeText: 'var(--color-status-warn)',
    label: 'Sandbox',
  },
};

function computeDagLayout(nodes: DagNode[]) {
  if (nodes.length === 0) {
    return { nodeCoords: new Map<string, { x: number; y: number }>(), canvasWidth: 500, canvasHeight: 200 };
  }
  const waveMap = new Map<number, DagNode[]>();
  for (const node of nodes) {
    const list = waveMap.get(node.wave) ?? [];
    list.push(node);
    waveMap.set(node.wave, list);
  }
  const coords = new Map<string, { x: number; y: number }>();
  const waves = Array.from(waveMap.keys()).sort((a, b) => a - b);
  const waveIndexMap = new Map<number, number>();
  waves.forEach((w, idx) => waveIndexMap.set(w, idx));

  for (const [wave, list] of waveMap.entries()) {
    const colIdx = waveIndexMap.get(wave) ?? 0;
    list.forEach((node, rowIdx) => {
      coords.set(node.id, { x: PADDING_X + colIdx * WAVE_GAP_X, y: PADDING_Y + rowIdx * NODE_GAP_Y });
    });
  }

  const maxWaveIndex = Math.max(0, waves.length - 1);
  const maxNodesInWave = Math.max(1, ...Array.from(waveMap.values()).map((l) => l.length));
  return {
    nodeCoords: coords,
    canvasWidth: Math.max(500, PADDING_X * 2 + maxWaveIndex * WAVE_GAP_X + NODE_WIDTH),
    canvasHeight: Math.max(200, PADDING_Y * 2 + (maxNodesInWave - 1) * NODE_GAP_Y + NODE_HEIGHT),
  };
}

interface DagSvgGraphProps {
  nodes: DagNode[];
  edges: DagEdge[];
  selectedNodeId: string | null;
  onToggleSelect: (id: string) => void;
  ariaLabel?: string;
}

function DagSvgGraph({
  nodes,
  edges,
  selectedNodeId,
  onToggleSelect,
  ariaLabel = 'Epistemic Research DAG',
}: DagSvgGraphProps): JSX.Element {
  const { nodeCoords, canvasWidth, canvasHeight } = useMemo(() => computeDagLayout(nodes), [nodes]);

  return (
    <div className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-elevated p-3">
      <svg
        role="region"
        aria-label={ariaLabel}
        viewBox={`0 0 ${canvasWidth} ${canvasHeight}`}
        className="w-full h-auto min-w-[500px]"
        style={{ minHeight: `${canvasHeight}px` }}
      >
        <title>{ariaLabel}</title>
        <desc>Dynamic acyclic graph showing evidence waves, claim status, and relational edge arrows</desc>
        <defs>
          <marker id="arrow-supports" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
            <path d="M 0 1 L 10 5 L 0 9 z" fill="var(--color-status-pass)" />
          </marker>
          <marker id="arrow-contradicts" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
            <path d="M 0 1 L 10 5 L 0 9 z" fill="var(--color-status-fail)" />
          </marker>
        </defs>
        <g aria-label="DAG Edges">
          {edges.map((edge) => {
            const src = nodeCoords.get(edge.srcId);
            const dst = nodeCoords.get(edge.dstId);
            if (!src || !dst) return null;
            const x1 = src.x + NODE_WIDTH;
            const y1 = src.y + NODE_HEIGHT / 2;
            const x2 = dst.x;
            const y2 = dst.y + NODE_HEIGHT / 2;
            const dx = Math.max(30, (x2 - x1) / 2);
            const d = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
            const isContradiction = edge.relation === 'contradicts' || edge.kind === 'contradicts';
            return (
              <path
                key={`${edge.srcId}-${edge.dstId}-${edge.relation}`}
                d={d}
                fill="none"
                stroke={isContradiction ? 'var(--color-status-fail)' : 'var(--color-status-pass)'}
                strokeWidth="1.75"
                strokeDasharray={isContradiction ? '4 4' : undefined}
                markerEnd={isContradiction ? 'url(#arrow-contradicts)' : 'url(#arrow-supports)'}
                className="transition-opacity duration-200"
              >
                <title>{`${edge.srcId} ${edge.kind ?? edge.relation} ${edge.dstId}`}</title>
              </path>
            );
          })}
        </g>
        <g aria-label="DAG Nodes">
          {nodes.map((node) => {
            const coord = nodeCoords.get(node.id);
            if (!coord) return null;
            const kindCfg = node.kind && node.kind !== 'claim' ? KIND_CONFIG[node.kind] : undefined;
            const statusCfg = kindCfg ?? STATUS_CONFIG[node.status] ?? STATUS_CONFIG.pending;
            const isSelected = selectedNodeId === node.id;
            const displayLabel = node.label.length > 22 ? `${node.label.slice(0, 20)}...` : node.label;

            return (
              <g
                key={node.id}
                data-testid={`dag-node-${node.id}`}
                transform={`translate(${coord.x}, ${coord.y})`}
                tabIndex={0}
                role="group"
                aria-label={`Node: ${node.label}, status: ${statusCfg.label}, wave: ${node.wave}`}
                onClick={() => onToggleSelect(node.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    onToggleSelect(node.id);
                  }
                }}
                className="cursor-pointer outline-none focus:ring-1 focus:ring-brass"
              >
                <rect
                  width={NODE_WIDTH}
                  height={NODE_HEIGHT}
                  rx={8}
                  ry={8}
                  fill={statusCfg.bg}
                  stroke={isSelected ? 'var(--color-accent-default)' : statusCfg.border}
                  strokeWidth={isSelected ? '2.5' : '1.5'}
                  className="transition-colors duration-150"
                />
                <text x={12} y={24} fill="var(--color-text-primary)" fontSize="12" fontWeight="600" className="pointer-events-none select-none font-sans">
                  {displayLabel}
                </text>
                <g transform="translate(12, 34)">
                  <rect width={84} height={18} rx={4} ry={4} fill={statusCfg.badgeBg} stroke={statusCfg.border} strokeWidth="0.75" />
                  <text x={42} y={13} fill={statusCfg.badgeText} fontSize="10" fontWeight="700" textAnchor="middle" className="pointer-events-none select-none font-display uppercase tracking-wider">
                    {statusCfg.label}
                  </text>
                </g>
                <text x={NODE_WIDTH - 12} y={46} fill="var(--color-text-muted)" fontSize="11" fontWeight="500" textAnchor="end" className="pointer-events-none select-none font-sans">
                  W{node.wave}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}

export function ResearchDagCanvas({ nodes, edges, className = '' }: ResearchDagCanvasProps): JSX.Element {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [expandedWaves, setExpandedWaves] = useState<Set<number>>(() => new Set());

  const isClustered = nodes.length > CLUSTER_THRESHOLD;

  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId],
  );

  const waveClusters = useMemo(() => {
    if (!isClustered) return [];
    const grouped = new Map<number, DagNode[]>();
    const nodeWave = new Map<string, number>();
    for (const n of nodes) {
      const list = grouped.get(n.wave) ?? [];
      list.push(n);
      grouped.set(n.wave, list);
      nodeWave.set(n.id, n.wave);
    }
    const edgesByWave = new Map<number, DagEdge[]>();
    for (const e of edges) {
      const srcWave = nodeWave.get(e.srcId);
      if (srcWave !== undefined && srcWave === nodeWave.get(e.dstId)) {
        const list = edgesByWave.get(srcWave) ?? [];
        list.push(e);
        edgesByWave.set(srcWave, list);
      }
    }
    return Array.from(grouped.entries())
      .sort(([a], [b]) => a - b)
      .map(([wave, waveNodes]) => {
        let verified = 0;
        let contradicted = 0;
        for (const n of waveNodes) {
          if (n.status === 'verified') verified += 1;
          else if (n.status === 'contradicted') contradicted += 1;
        }
        const pending = waveNodes.length - verified - contradicted;
        const waveEdges = edgesByWave.get(wave) ?? [];
        return { wave, waveNodes, waveEdges, verified, contradicted, pending };
      });
  }, [isClustered, nodes, edges]);

  const toggleSelect = (id: string) => setSelectedNodeId((prev) => (prev === id ? null : id));
  const toggleWave = (wave: number) =>
    setExpandedWaves((prev) => {
      const next = new Set(prev);
      if (next.has(wave)) next.delete(wave);
      else next.add(wave);
      return next;
    });

  return (
    <div className={`space-y-2 ${className}`}>
      {isClustered ? (
        <div data-testid="dag-clustered-container" className="space-y-2 rounded-lg border border-border-subtle bg-bg-surface p-3">
          <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
            <span className="text-xs text-text-secondary">
              Clustered DAG ({nodes.length} nodes across {waveClusters.length} waves; collapsed above {CLUSTER_THRESHOLD} nodes)
            </span>
            <div className="flex items-center gap-1.5">
              <button
                type="button"
                data-slot="button"
                data-testid="dag-expand-all-waves"
                onClick={() => setExpandedWaves(new Set(waveClusters.map((c) => c.wave)))}
                className="h-8 rounded border border-border-subtle bg-bg-elevated px-2.5 text-xs font-medium text-text-secondary hover:text-text-primary transition-colors"
              >
                Expand all
              </button>
              <button
                type="button"
                data-slot="button"
                data-testid="dag-collapse-all-waves"
                onClick={() => setExpandedWaves(new Set())}
                className="h-8 rounded border border-border-subtle bg-bg-elevated px-2.5 text-xs font-medium text-text-secondary hover:text-text-primary transition-colors"
              >
                Collapse all
              </button>
            </div>
          </div>

          <div className="space-y-2">
            {waveClusters.map(({ wave, waveNodes, waveEdges, verified, contradicted, pending }) => {
              const isExpanded = expandedWaves.has(wave);
              return (
                <div
                  key={wave}
                  data-testid={`dag-wave-cluster-${wave}`}
                  className="rounded-lg border border-border-subtle bg-bg-panel"
                >
                  <button
                    type="button"
                    data-slot="accordion-trigger"
                    data-testid={`dag-wave-toggle-${wave}`}
                    aria-expanded={isExpanded}
                    onClick={() => toggleWave(wave)}
                    className="flex h-9 w-full flex-wrap items-center justify-between gap-2 px-3 text-left text-xs hover:bg-bg-elevated transition-colors"
                  >
                    <div className="flex items-center gap-2">
                      <Icon.branch className="size-3.5 text-brass shrink-0" aria-hidden="true" />
                      <span className="font-mono font-semibold text-text-primary">Wave {wave}</span>
                      <span className="rounded bg-bg-elevated border border-border-subtle px-1.5 py-0.5 font-mono text-[11px] text-text-secondary">
                        {waveNodes.length} nodes
                      </span>
                    </div>
                    <div className="flex items-center gap-2 font-mono text-[11px]">
                      <span className="text-(--color-status-pass)">{verified} verified</span>
                      <span className="text-text-muted">{pending} pending</span>
                      {contradicted > 0 && <span className="text-(--color-status-fail)">{contradicted} contradicted</span>}
                      {isExpanded ? (
                        <Icon.chevronUp className="size-3.5 text-text-muted shrink-0" aria-hidden="true" />
                      ) : (
                        <Icon.chevronDown className="size-3.5 text-text-muted shrink-0" aria-hidden="true" />
                      )}
                    </div>
                  </button>
                  {isExpanded && (
                    <div className="border-t border-border-subtle p-2">
                      <DagSvgGraph
                        nodes={waveNodes}
                        edges={waveEdges}
                        selectedNodeId={selectedNodeId}
                        onToggleSelect={toggleSelect}
                        ariaLabel={`Epistemic Research DAG Wave ${wave}`}
                      />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        <DagSvgGraph
          nodes={nodes}
          edges={edges}
          selectedNodeId={selectedNodeId}
          onToggleSelect={toggleSelect}
        />
      )}

      {selectedNode && (
        <div data-testid="dag-node-inspector" className="rounded-lg border border-brass/30 bg-bg-elevated p-3 text-xs space-y-1.5">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <span className="rounded bg-brass/20 px-2 py-0.5 font-mono text-[11px] uppercase tracking-wider text-brass">
                {selectedNode.kind ?? 'claim'}
              </span>
              {selectedNode.verdict && (
                <span className="font-mono text-[11px] text-text-secondary">Verdict: {selectedNode.verdict}</span>
              )}
              {selectedNode.confidence != null && (
                <span className="font-mono text-[11px] text-(--color-status-pass)">
                  {Math.round(selectedNode.confidence * 100)}% confidence
                </span>
              )}
            </div>
            <button
              type="button"
              data-slot="button"
              onClick={() => setSelectedNodeId(null)}
              className="h-7 rounded border border-border-subtle bg-bg-surface px-2 text-[11px] font-medium text-text-secondary hover:text-text-primary transition-colors"
            >
              Close
            </button>
          </div>
          <div className="text-sm font-serif text-text-primary leading-relaxed">{selectedNode.label}</div>
          {(selectedNode.snippet || selectedNode.excerpt) && (
            <div className="rounded border border-border-subtle bg-bg-surface p-2 text-xs text-text-secondary leading-relaxed">
              {selectedNode.snippet ?? selectedNode.excerpt}
            </div>
          )}
          {selectedNode.url && (
            <div className="pt-0.5">
              <SafeExternalLink url={selectedNode.url} className="font-mono text-[11px] text-brass underline decoration-dotted" />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
