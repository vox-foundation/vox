import React, { useMemo, useCallback } from 'react';
import { 
  ReactFlow, 
  Background, 
  Controls, 
  Panel,
  useNodesState,
  useEdgesState,
  addEdge,
  MarkerType,
  Handle,
  Position
} from '@xyflow/react';
import { Activity, CheckCircle2, Clock, AlertCircle, Ban, ShieldAlert } from 'lucide-react';
import '@xyflow/react/dist/style.css';

// Custom Node for Agent Tasks
const TaskNode = ({ data }: any) => {
  let statusColor = 'zinc';
  let nodeClass = '';
  let Icon = Clock;
  let iconClass = 'text-zinc-500';

  if (data.status === 'Completed') {
    statusColor = 'emerald';
    nodeClass = 'node-completed';
    Icon = CheckCircle2;
    iconClass = 'text-emerald-500';
  } else if (data.status === 'InProgress') {
    // Map execution mode to color
    if (data.mode === 'Fast') statusColor = 'rose';
    else if (data.mode === 'Verbose') statusColor = 'blue';
    else if (data.mode === 'Precision') statusColor = 'violet';
    else statusColor = 'emerald'; // Efficient
    Icon = Activity;
    iconClass = `text-${statusColor}-500 animate-pulse`;
  } else if (data.status.startsWith('Failed')) {
    statusColor = 'rose';
    nodeClass = 'node-failed';
    Icon = AlertCircle;
    iconClass = 'text-rose-500';
  } else if (data.status === 'Cancelled') {
    statusColor = 'zinc';
    nodeClass = 'node-cancelled';
    Icon = Ban;
  } else if (data.status === 'Blocked') {
    statusColor = 'amber';
    nodeClass = 'node-blocked';
    Icon = ShieldAlert;
    iconClass = 'text-amber-500';
  }

  return (
    <div className={`w-64 glass rounded-xl border border-${statusColor}-500/30 overflow-hidden glow-${statusColor} ${nodeClass}`}>
      <div className={`h-1 bg-${statusColor}-500 ${data.status === 'InProgress' ? 'animate-pulse' : ''}`} />
      <div className="p-4">
        <div className="flex justify-between items-start mb-3">
          <span className={`text-[10px] font-bold uppercase tracking-widest text-${statusColor}-500`}>{data.id}</span>
          <Icon size={12} className={iconClass} />
        </div>
        <p className="text-xs font-semibold text-white/90 leading-relaxed mb-4">{data.description}</p>
        <div className="flex items-center justify-between">
           <span className="text-[10px] px-2 py-0.5 rounded bg-white/5 border border-white/10 text-zinc-400 font-mono italic">@{data.agent_id}</span>
           <div className="flex flex-col items-end">
             <span className="text-[10px] text-zinc-500 font-mono">{data.priority}</span>
             {data.mode && <span className={`text-[9px] font-bold uppercase text-${statusColor}-500 opacity-80 mt-1`}>{data.mode}</span>}
           </div>
        </div>
      </div>
      <Handle type="target" position={Position.Top} className="opacity-0" />
      <Handle type="source" position={Position.Bottom} className="opacity-0" />
    </div>
  );
};

const nodeTypes = {
  task: TaskNode,
};

export const AgentFlow = ({ tasks = [] }: { tasks: any[] }) => {
  const initialNodes = useMemo(() => tasks.map((t, idx) => ({
    id: t.id.toString(),
    type: 'task',
    position: { x: (idx % 3) * 300, y: Math.floor(idx / 3) * 200 },
    data: { ...t, agent_id: t.agent_id || 'A-01', mode: t.mode || 'Efficient' },
  })), [tasks]);

  const initialEdges = useMemo(() => {
    const edges: any[] = [];
    tasks.forEach(t => {
      if (t.depends_on) {
        t.depends_on.forEach((dep: any) => {
          let edgeClass = '';
          let edgeColor = '#3b82f6';
          
          if (t.status === 'InProgress') {
            if (t.mode === 'Efficient') { edgeClass = 'mode-efficient'; edgeColor = '#4ADE80'; }
            else if (t.mode === 'Fast') { edgeClass = 'mode-fast'; edgeColor = '#EF4444'; }
            else if (t.mode === 'Verbose') { edgeClass = 'mode-verbose'; edgeColor = '#60A5FA'; }
            else if (t.mode === 'Precision') { edgeClass = 'mode-precision'; edgeColor = '#A78BFA'; }
          } else if (t.status === 'Completed') {
            edgeColor = '#10b981';
          } else if (t.status === 'Blocked') {
            edgeColor = '#f59e0b';
          } else if (t.status === 'Cancelled') {
            edgeColor = '#71717a';
          }

          edges.push({
            id: `e-${dep}-${t.id}`,
            source: dep.toString(),
            target: t.id.toString(),
            type: t.mode === 'Precision' ? 'straight' : 'smoothstep', // Precision uses converging lines
            animated: t.status === 'InProgress',
            className: edgeClass,
            style: { stroke: edgeColor, strokeWidth: t.mode === 'Verbose' ? 4 : 2, opacity: t.status === 'Cancelled' ? 0.2 : 0.6 },
            markerEnd: t.mode === 'Precision' ? undefined : { type: MarkerType.ArrowClosed, color: edgeColor },
          });
        });
      }
    });
    return edges;
  }, [tasks]);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  const onConnect = useCallback((params: any) => setEdges((eds) => addEdge(params, eds)), []);

  return (
    <div className="h-full w-full bg-[#09090b] relative">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        fitView
      >
        <Background color="#27272a" gap={20} />
        <Controls className="!bg-[#09090b] !border-white/10 [&>button]:!bg-transparent [&>button]:!border-white/10 [&>button>svg]:!fill-white/50" />
        <Panel position="top-right" className="glass p-4 rounded-xl border border-white/5 flex flex-col gap-3">
           <div className="text-[10px] font-bold uppercase tracking-widest text-zinc-500 border-b border-white/5 pb-2">Execution Modes</div>
           <div className="flex items-center gap-2">
             <div className="w-2 h-2 rounded-full shadow-[0_0_10px_rgba(74,222,128,0.5)] bg-emerald-400" />
             <span className="text-[10px] text-zinc-300 font-bold uppercase">Efficient <span className="text-zinc-600 font-normal normal-case ml-1">(Linear)</span></span>
           </div>
           <div className="flex items-center gap-2">
             <div className="w-2 h-2 rounded-full shadow-[0_0_10px_rgba(239,68,68,0.5)] bg-red-500" />
             <span className="text-[10px] text-zinc-300 font-bold uppercase">Fast <span className="text-zinc-600 font-normal normal-case ml-1">(Burst)</span></span>
           </div>
           <div className="flex items-center gap-2">
             <div className="w-2 h-2 rounded-full shadow-[0_0_10px_rgba(96,165,250,0.5)] bg-blue-400" />
             <span className="text-[10px] text-zinc-300 font-bold uppercase">Verbose <span className="text-zinc-600 font-normal normal-case ml-1">(Wisp Glow)</span></span>
           </div>
           <div className="flex items-center gap-2">
             <div className="w-2 h-2 rounded-full shadow-[0_0_10px_rgba(167,139,250,0.5)] bg-violet-400" />
             <span className="text-[10px] text-zinc-300 font-bold uppercase">Precision <span className="text-zinc-600 font-normal normal-case ml-1">(Pulse Converge)</span></span>
           </div>
        </Panel>
      </ReactFlow>
    </div>
  );
};
