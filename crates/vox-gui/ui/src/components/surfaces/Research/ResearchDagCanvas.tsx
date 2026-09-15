import React, { useMemo, type JSX } from 'react';

export interface DagNode {
  id: string;
  label: string;
  wave: number;
  status: 'pending' | 'verified' | 'contradicted';
}

export interface DagEdge {
  srcId: string;
  dstId: string;
  relation: 'supports' | 'contradicts';
}

export interface ResearchDagCanvasProps {
  nodes: DagNode[];
  edges: DagEdge[];
  className?: string;
}

const NODE_WIDTH = 180;
const NODE_HEIGHT = 60;
const WAVE_GAP_X = 240;
const NODE_GAP_Y = 85;
const PADDING_X = 40;
const PADDING_Y = 40;

const STATUS_CONFIG = {
  verified: {
    bg: '#064e3b',
    border: '#10b981',
    badgeBg: '#047857',
    badgeText: '#ecfdf5',
    label: 'Verified',
  },
  contradicted: {
    bg: '#450a0a',
    border: '#ef4444',
    badgeBg: '#b91c1c',
    badgeText: '#fff1f2',
    label: 'Contradicted',
  },
  pending: {
    bg: '#1c1917',
    border: '#78716c',
    badgeBg: '#44403c',
    badgeText: '#f5f5f4',
    label: 'Pending',
  },
} as const;

export function ResearchDagCanvas({ nodes, edges, className = '' }: ResearchDagCanvasProps): JSX.Element {
  const { nodeCoords, canvasWidth, canvasHeight } = useMemo(() => {
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

    // Compute contiguous normalized wave index
    const waveIndexMap = new Map<number, number>();
    waves.forEach((w, idx) => waveIndexMap.set(w, idx));

    for (const [wave, list] of waveMap.entries()) {
      const colIdx = waveIndexMap.get(wave) ?? 0;
      list.forEach((node, rowIdx) => {
        coords.set(node.id, {
          x: PADDING_X + colIdx * WAVE_GAP_X,
          y: PADDING_Y + rowIdx * NODE_GAP_Y,
        });
      });
    }

    const maxWaveIndex = Math.max(0, waves.length - 1);
    const maxNodesInWave = Math.max(1, ...Array.from(waveMap.values()).map((l) => l.length));

    const width = Math.max(500, PADDING_X * 2 + maxWaveIndex * WAVE_GAP_X + NODE_WIDTH);
    const height = Math.max(200, PADDING_Y * 2 + (maxNodesInWave - 1) * NODE_GAP_Y + NODE_HEIGHT);

    return { nodeCoords: coords, canvasWidth: width, canvasHeight: height };
  }, [nodes]);

  return (
    <div className={`overflow-x-auto rounded-lg border border-border-subtle bg-black/40 p-2 ${className}`}>
      <svg
        role="region"
        aria-label="Epistemic Research DAG"
        viewBox={`0 0 ${canvasWidth} ${canvasHeight}`}
        className="w-full h-auto min-w-[500px]"
        style={{ minHeight: `${canvasHeight}px` }}
      >
        <title>Epistemic Research DAG</title>
        <desc>Dynamic acyclic graph showing evidence waves, claim status, and relational edge arrows</desc>

        <defs>
          <marker
            id="arrow-supports"
            viewBox="0 0 10 10"
            refX="8"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 10 5 L 0 9 z" fill="#10b981" />
          </marker>
          <marker
            id="arrow-contradicts"
            viewBox="0 0 10 10"
            refX="8"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 10 5 L 0 9 z" fill="#ef4444" />
          </marker>
        </defs>

        {/* Render edges */}
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
            const isContradiction = edge.relation === 'contradicts';
            const color = isContradiction ? '#ef4444' : '#10b981';
            const marker = isContradiction ? 'url(#arrow-contradicts)' : 'url(#arrow-supports)';

            return (
              <path
                key={`${edge.srcId}-${edge.dstId}-${edge.relation}`}
                d={d}
                fill="none"
                stroke={color}
                strokeWidth="2"
                strokeDasharray={isContradiction ? '4 4' : undefined}
                markerEnd={marker}
                className="transition-all duration-200"
              >
                <title>{`${edge.srcId} ${edge.relation} ${edge.dstId}`}</title>
              </path>
            );
          })}
        </g>

        {/* Render nodes */}
        <g aria-label="DAG Nodes">
          {nodes.map((node) => {
            const coord = nodeCoords.get(node.id);
            if (!coord) return null;

            const statusCfg = STATUS_CONFIG[node.status] ?? STATUS_CONFIG.pending;
            const displayLabel =
              node.label.length > 22 ? `${node.label.slice(0, 20)}…` : node.label;

            return (
              <g
                key={node.id}
                transform={`translate(${coord.x}, ${coord.y})`}
                tabIndex={0}
                role="group"
                aria-label={`Node: ${node.label}, status: ${statusCfg.label}, wave: ${node.wave}`}
                className="outline-none focus:ring-1 focus:ring-brass"
              >
                {/* Node Box */}
                <rect
                  width={NODE_WIDTH}
                  height={NODE_HEIGHT}
                  rx={8}
                  ry={8}
                  fill={statusCfg.bg}
                  stroke={statusCfg.border}
                  strokeWidth="1.5"
                  className="transition-colors duration-150"
                />

                {/* Node Label Text */}
                <text
                  x={12}
                  y={24}
                  fill="#f3f4f6"
                  fontSize="12"
                  fontWeight="600"
                  className="pointer-events-none select-none font-sans"
                >
                  {displayLabel}
                </text>

                {/* Status Badge */}
                <g transform="translate(12, 34)">
                  <rect
                    width={72}
                    height={18}
                    rx={4}
                    ry={4}
                    fill={statusCfg.badgeBg}
                  />
                  <text
                    x={36}
                    y={13}
                    fill={statusCfg.badgeText}
                    fontSize="10"
                    fontWeight="700"
                    textAnchor="middle"
                    className="pointer-events-none select-none font-mono uppercase tracking-wider"
                  >
                    {statusCfg.label}
                  </text>
                </g>

                {/* Wave Badge */}
                <text
                  x={NODE_WIDTH - 12}
                  y={46}
                  fill="#9ca3af"
                  fontSize="10"
                  fontWeight="500"
                  textAnchor="end"
                  className="pointer-events-none select-none font-mono"
                >
                  w{node.wave}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
