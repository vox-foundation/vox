import React, { useMemo, useCallback, useState, useEffect } from 'react';
import { 
  ReactFlow, 
  Background, 
  Controls, 
  Panel,
  useNodesState,
  useEdgesState,
  MarkerType,
  Handle,
  Position,
  Edge
} from '@xyflow/react';
import { Server, Zap, Globe2, Cpu, Activity, Send } from 'lucide-react';
import '@xyflow/react/dist/style.css';

const MeshNode = ({ data }: any) => {
  const isPrimary = data.type === 'primary';
  const roleColor = isPrimary ? 'emerald' : 'blue';

  return (
    <div className={`w-56 glass rounded-2xl border ${isPrimary ? 'border-emerald-500/50 glow-emerald' : 'border-blue-500/30'} bg-black/50 overflow-hidden`}>
      <div className={`px-4 py-3 border-b border-white/5 flex justify-between items-center bg-${roleColor}-500/10`}>
        <div className="flex items-center gap-2">
          {isPrimary ? <Globe2 size={14} className="text-emerald-500" /> : <Server size={14} className="text-blue-500" />}
          <span className={`text-xs font-bold uppercase tracking-widest text-${roleColor}-500`}>{data.id}</span>
        </div>
        <div className="flex gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
        </div>
      </div>
      
      <div className="p-4 flex flex-col gap-3 relative">
        <div className="flex justify-between items-center text-[10px] font-mono text-zinc-400">
           <span>Region: {data.region}</span>
           <span className="text-emerald-500">{data.latency}</span>
        </div>
        
        <div className="flex items-center gap-2 mt-2">
           <Cpu size={12} className="text-zinc-500" />
           <div className="flex-1 h-1.5 bg-white/5 rounded-full overflow-hidden">
               <div className={`h-full bg-${roleColor}-500`} style={{ width: `${data.cpu}%` }} />
           </div>
           <span className="text-[9px] font-mono text-zinc-500">{data.cpu}%</span>
        </div>

        <div className="mt-2 text-[9px] font-bold uppercase tracking-widest text-zinc-500 border-t border-white/5 pt-3">Local Actors</div>
        <div className="flex flex-wrap gap-1">
            {data.actors.map((actor: string, i: number) => (
                <span key={i} className="text-[8px] font-mono bg-white/5 px-1.5 py-0.5 rounded border border-white/10 text-zinc-300">@{actor}</span>
            ))}
        </div>
      </div>
      <Handle type="target" position={Position.Left} className="opacity-0" />
      <Handle type="source" position={Position.Right} className="opacity-0" />
      <Handle type="source" position={Position.Bottom} id="bottom" className="opacity-0" />
      <Handle type="target" position={Position.Top} id="top" className="opacity-0" />
    </div>
  );
};

const nodeTypes = {
  meshNode: MeshNode,
};

export function MeshTopology({ topology }: any) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  useEffect(() => {
    if (!topology) return;
    
    setNodes((topology.nodes || []).map((n: any, i: number) => ({
        id: n.id, type: 'meshNode', position: n.position || { x: 100 + (i * 300), y: 150 + ((i%2) * 150) },
        data: { id: n.id, type: n.node_type, region: n.region, latency: n.latency_ms + 'ms', cpu: n.cpu_pct, actors: n.resident_actors || [] }
    })));

    setEdges((topology.edges || []).map((e: any) => ({
        id: e.id, source: e.from, target: e.to, animated: e.status === 'ws',
        style: { stroke: e.status === 'ws' ? '#3b82f6' : '#52525b', strokeWidth: e.status === 'ws' ? 2 : 1 }
    })));
  }, [topology]);

  return (
    <div className="h-full w-full bg-[#09090b] relative text-white">
        <div className="absolute top-10 left-10 z-10 pointer-events-none">
            <h2 className="text-3xl font-black tracking-tighter uppercase mb-2 flex items-center gap-3">
                <Globe2 size={28} className="text-emerald-500" />
                Vox<span className="text-emerald-500">Mesh</span>
            </h2>
            <p className="text-xs text-zinc-400 font-bold tracking-widest uppercase">Distributed Compute Topology</p>
        </div>

      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        fitView
      >
        <Background color="#27272a" gap={20} />
        <Controls className="!bg-[#09090b] !border-white/10 [&>button]:!bg-transparent [&>button]:!border-white/10 [&>button>svg]:!fill-white/50" />
        <Panel position="top-right" className="glass p-6 rounded-2xl border border-white/5 flex flex-col gap-4 w-72">
           <div className="text-[10px] font-bold uppercase tracking-widest text-zinc-500 border-b border-white/5 pb-2 flex items-center gap-2">
               <Activity size={12} /> Live Network Status
           </div>
           
           <div className="flex justify-between items-center text-xs">
               <span className="text-zinc-400 font-bold uppercase tracking-widest">Active Nodes</span>
               <span className="font-mono text-emerald-500">{nodes.length} Online</span>
           </div>

           <div className="flex justify-between items-center text-xs">
               <span className="text-zinc-400 font-bold uppercase tracking-widest">WebSocket Links</span>
               <span className="font-mono text-blue-500">{edges.filter(e => e.animated).length} Active</span>
           </div>

           <div className="flex justify-between items-center text-xs border-b border-white/5 pb-4">
               <span className="text-zinc-400 font-bold uppercase tracking-widest">Actor Hops</span>
               <span className="font-mono text-violet-500">{topology?.active_migrations?.length || 0} recent</span>
           </div>

           {(topology?.active_migrations || []).map((m: any, i: number) => (
               <div key={i} className="mt-2 bg-violet-500/10 border border-violet-500/30 rounded-xl p-3 flex flex-col gap-2 relative overflow-hidden">
                   <div className="absolute inset-0 bg-violet-400/5 animate-pulse" />
                   <div className="flex items-center gap-2 text-violet-400 relative z-10">
                       <Send size={12} />
                       <span className="text-[9px] font-bold uppercase tracking-widest">Location Transparency Hop</span>
                   </div>
                   <div className="text-[10px] font-mono text-zinc-300 relative z-10">
                       Migrating <span className="text-white">@{m.actor}</span>
                       <br/><span className="text-zinc-500">{m.from_node} → {m.to_node}</span>
                   </div>
               </div>
           ))}
        </Panel>
      </ReactFlow>
    </div>
  );
}
