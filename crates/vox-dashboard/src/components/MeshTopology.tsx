import React, { useMemo } from 'react';
import { ReactFlow, Controls, Background, MarkerType } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

const nodeStyle = {
  background: 'var(--vox-machine)',
  color: 'var(--vox-primary)',
  border: '1px solid var(--vox-copper)',
  boxShadow: '0 0 10px var(--vox-amber-glow)',
  borderRadius: '8px',
  padding: '10px 15px',
  fontFamily: 'Rajdhani, sans-serif',
  fontWeight: 'bold',
  textTransform: 'uppercase' as const,
  letterSpacing: '0.1em',
};

export const MeshTopology = ({ meshTopology }: { meshTopology: any }) => {
  const nodes = useMemo(() => {
    if (!meshTopology?.nodes || !Array.isArray(meshTopology.nodes)) return [];
    return meshTopology.nodes.map((n: any, idx: number) => ({
      id: String(n.id || `node-${idx}`),
      position: { x: (idx % 3) * 200 + 50, y: Math.floor(idx / 3) * 150 + 50 },
      data: { label: n.name || n.id || `Node ${idx}` },
      style: nodeStyle,
      type: 'default',
    }));
  }, [meshTopology]);

  const edges = useMemo(() => {
    if (!meshTopology?.edges || !Array.isArray(meshTopology.edges)) return [];
    return meshTopology.edges.map((e: any, idx: number) => ({
      id: String(e.id || `edge-${idx}`),
      source: String(e.source),
      target: String(e.target),
      animated: true,
      style: { stroke: 'var(--vox-cyan)', strokeWidth: 2 },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: 'var(--vox-cyan)',
      },
    }));
  }, [meshTopology]);

  return (
    <div className="w-full h-full bg-void rounded-xl border border-border shadow-[inset_0_0_20px_rgba(0,0,0,1)] relative overflow-hidden">
      <div className="absolute top-4 left-4 z-10">
          <h2 className="text-2xl font-rajdhani text-brass tracking-wider">RETE</h2>
          <div className="px-2 py-0.5 rounded bg-machine border border-border text-[9px] font-mono text-steel uppercase tracking-widest inline-block mt-1">
              {nodes.length} Nodes Online
          </div>
      </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        className="bg-transparent"
      >
        <Background 
          color="var(--vox-steel)" 
          gap={24} 
          size={1} 
          style={{ opacity: 0.1 }}
        />
        <Controls 
          className="bg-machine border-border fill-steel !shadow-none [&>button]:border-b-border [&>button:hover]:bg-surface" 
        />
      </ReactFlow>
    </div>
  );
};
