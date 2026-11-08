import React, { useState } from 'react';
import { Glass } from '../../ui/Glass';
import { Pill, PhaseKind } from '../../ui/Pill';
import { Agent } from '../../../types/dashboard';
import { phaseStroke, viz } from '../../../lib/visualTokens';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

interface GraphNode {
  id: string;
  label: string;
  phase: string;
  x: number;
  y: number;
}

interface GraphEdge {
  from: string;
  to: string;
  flow: number;
}

interface AgentGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

interface AgentFlowProps {
  agents: Agent[];
  graph?: AgentGraph;
  onSelect?: (id: string) => void;
  selectedId?: string;
}

function Legend({ color, label, purpose }: { color: string; label: string; purpose?: string }) {
  const content = (
    <span className="flex items-center gap-1.5 text-text-muted hover:text-[#F3E5AB] transition-colors cursor-default">
      <span className={`size-1.5 rounded-full ${color}`} />
      <span className="text-[10px]">{label}</span>
    </span>
  );
  if (!purpose) return content;
  return (
    <RomanTooltip
      title={`${label} Phase`}
      purpose={purpose}
      placement="bottom"
    >
      {content}
    </RomanTooltip>
  );
}

/** Build a synthetic graph from live agent data when no pre-computed graph is provided. */
function buildGraphFromAgents(agents: Agent[]): AgentGraph {
  const root: GraphNode = { id: 'ROOT', label: 'Orchestrator', phase: 'Root', x: 0.5, y: 0.42 };

  // Distribute agents in a circle around the root.
  const nodes: GraphNode[] = [root];
  const edges: GraphEdge[] = [];
  const radius = 0.32;

  agents.forEach((a, i) => {
    const angle = (i / agents.length) * Math.PI * 2 - Math.PI / 2;
    nodes.push({
      id: a.id,
      label: a.codename,
      phase: a.phase,
      x: Math.min(0.9, Math.max(0.1, 0.5 + Math.cos(angle) * radius)),
      y: Math.min(0.85, Math.max(0.1, 0.42 + Math.sin(angle) * radius)),
    });
    edges.push({ from: 'ROOT', to: a.id, flow: a.progress ?? 0.5 });
  });

  // Wire co-dependent agents (e.g. verifying → verified).
  agents.forEach((a) => {
    if (a.phase === 'Verifying') {
      const compiler = agents.find(x => x.skill === 'compiler' && x.id !== a.id);
      if (compiler) {
        edges.push({ from: a.id, to: compiler.id, flow: 0.4 });
      }
    }
  });

  return { nodes, edges };
}

function AgentInspector({ node, agent }: { node: GraphNode; agent?: Agent }) {
  return (
    <div className="vox-marble-tablet w-full min-w-0 rounded-xl border border-[#C59B27] p-3 text-[#1C140C] shadow-[inset_0_1px_0_rgba(255,255,255,0.9)]">
      <div className="flex items-center justify-between">
        <div>
          <div className="font-display text-[14px] font-bold text-[#1C140C]">{node.label}</div>
          <div className="font-serif text-[10.5px] font-semibold text-[#5C441E]">{node.id} · crates/vox-orchestrator/src/router.rs</div>
        </div>
        <Pill phase={node.phase as PhaseKind} />
      </div>
      {agent ? (
        <>
          <div className="mt-2 text-[12px] text-[#2B1F11] leading-relaxed font-medium">{agent.task}</div>
          <div className="mt-2 grid grid-cols-4 gap-2 text-[10px]">
            {[
              ['Cost',   `$${(agent.cost ?? 0).toFixed(2)}`],
              ['Budget', agent.budget != null ? `$${agent.budget.toFixed(2)}` : '—'],
              ['ETA',    agent.eta],
              ['Skill',  agent.skill ?? '—'],
            ].map(([label, value]) => (
              <div key={label} className="rounded-md border border-[#8C6825]/60 bg-[#251C0E] px-2 py-1 text-[#F3E5AB]">
                <div className="text-[8.5px] font-display uppercase tracking-widest text-[#D4AF37]">{label}</div>
                <div className="mt-0.5 font-serif text-[11px] font-bold text-[#F3E5AB]">{value}</div>
              </div>
            ))}
          </div>
          {/* Progress bar — AgentRow pattern: ellipsis + no progressbar when unknown */}
          <div className="mt-2">
            <div className="flex items-center justify-between text-[9px] font-display font-bold uppercase tracking-widest text-[#4A3620]">
              <span>Progress</span>
              <span
                data-testid="agent-inspector-progress"
                className="font-serif font-bold text-[#1C140C]"
              >
                {agent.progress != null ? `${Math.round(agent.progress * 100)}%` : '…'}
              </span>
            </div>
            {agent.progress != null ? (
              <div
                className="mt-1 h-1.5 overflow-hidden rounded-full bg-[#D8C9AE] border border-[#8C6825]/50"
                role="progressbar"
                aria-label={`${node.label} progress`}
                aria-valuenow={Math.round(agent.progress * 100)}
                aria-valuemin={0}
                aria-valuemax={100}
              >
                <div
                  className="h-full bg-linear-to-r from-[#8C6825] to-[#4D5D1E] transition-[width] duration-700"
                  style={{ width: `${agent.progress * 100}%` }}
                />
              </div>
            ) : (
              <div className="relative mt-1 h-1.5 overflow-hidden rounded-full bg-[#D8C9AE] border border-[#8C6825]/50">
                <div className="absolute inset-y-0 left-0 w-1/3 rounded-full bg-[#8C6825] animate-pulse" />
              </div>
            )}
          </div>
        </>
      ) : (
        <div className="mt-2 text-[11.5px] font-medium text-[#3B2E1E]">
          Root coordinator · routes all task fan-out via file-affinity policy (`crates/vox-orchestrator/src/`).
        </div>
      )}
    </div>
  );
}

export function AgentFlow({ agents, graph, onSelect, selectedId }: AgentFlowProps) {
  const [sel, setSel] = useState<string>(selectedId ?? 'ROOT');
  const [isLive, setIsLive] = useState<boolean>(true);
  const [flowReceipt, setFlowReceipt] = useState<string | null>(null);
  const [flowYieldXp, setFlowYieldXp] = useState<number>(150);

  const g = graph ?? buildGraphFromAgents(agents);
  const W = 1200;
  const H = 640;

  const nodeAt = (id: string) => g.nodes.find(n => n.id === id);
  const pos = (n: GraphNode) => ({ x: n.x * W, y: n.y * H });

  const selectedNode = nodeAt(sel);
  const selectedAgent = agents.find(a => a.id === sel);
  const modelByIdx = [
    'Sonnet 4.5 · 184 tok/s',
    'Gemini 2.5 Pro · 240 tok/s',
    'MENS QLoRA · 312 tok/s',
    'Opus 4.6 · 128 tok/s',
  ];

  return (
    <Glass className="relative flex h-full min-h-0 flex-col overflow-hidden p-0">
      <div
        data-roman-frieze="true"
        className="vox-animate-marble-sheen relative flex shrink-0 flex-wrap items-center justify-between gap-x-3 gap-y-2 border-b-2 border-[#C59B27] px-4 py-3"
        style={{
          backgroundImage: `linear-gradient(115deg, rgba(255,252,245,0) 35%, rgba(255,248,225,0.48) 50%, rgba(255,252,245,0) 65%), linear-gradient(90deg, rgba(246,240,229,0.88) 0%, rgba(238,229,212,0.74) 48%, rgba(246,240,229,0.88) 100%), url('/assets/romanesque/white-marble-frieze-banner.jpg')`,
          backgroundSize: '240% 100%, 100% 100%, cover',
          backgroundPosition: 'center',
        }}
      >
        <div className="relative z-10 min-w-0">
          <div className="flex items-center gap-2 mb-0.5">
            <RomanMedallion active size="sm" {...({ icon: 'brazierTorch', tone: 'gold' } as any)}>
              <Icon.brazierTorch className="size-3.5 text-[#FFF8E7]" aria-hidden="true" />
            </RomanMedallion>
            <span className="rounded border border-[#785312] bg-linear-to-b from-[#F2D67B] via-[#D4AF37] to-[#9A741C] px-2 py-0.5 font-display text-[9px] font-bold tracking-[0.12em] uppercase text-[#1A1205]">
              Agent Flow
            </span>
          </div>
          <h2 className="font-display text-[16px] font-bold tracking-[0.12em] text-[#1C140C]">
            Agent topology
          </h2>
          <p className="font-serif text-[11.5px] font-medium text-[#3B2E1E]">
            Who is running and in what phase — click a node to inspect
          </p>
        </div>

        <div className="relative z-10 flex flex-wrap items-center gap-3">
          <div className="flex items-center gap-1.5">
            <RomanTooltip
              title={isLive ? 'Live Topology Stream' : 'Stream Paused'}
              purpose="Continuous real-time synchronization with active agent process states and tool execution telemetry."
              actionHint={isLive ? 'Click to pause live updates' : 'Click to resume live stream'}
              statusBadge={isLive ? 'LIVE' : 'PAUSED'}
            >
              <button
                type="button"
                tabIndex={0}
                onClick={() => setIsLive(!isLive)}
                className="inline-flex items-center gap-1.5 rounded-md border border-[#D4AF37]/40 bg-[#1D1811] px-2 py-1 font-mono text-[10px] text-[#F3E5AB] hover:border-[#D4AF37]"
              >
                <span className={`size-1.5 rounded-full ${isLive ? 'bg-[#8FA339] animate-pulse' : 'bg-[#8C7A5E]'}`} />
                <span>{isLive ? 'Live' : 'Paused'}</span>
              </button>
            </RomanTooltip>
            <RomanTooltip
              title="Reset View"
              purpose="Re-centers the canvas view and focuses the Root Orchestrator node."
              actionHint="Reset graph focus"
            >
              <button
                type="button"
                tabIndex={0}
                onClick={() => { setSel('ROOT'); onSelect?.('ROOT'); }}
                className="rounded-md border border-[#D4AF37]/40 bg-[#1D1811] px-2 py-1 font-mono text-[10px] text-[#C8B99E] hover:border-[#D4AF37] hover:text-[#F3E5AB]"
              >
                Reset View
              </button>
            </RomanTooltip>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Legend color="bg-brass"      label="Planning" purpose="Agent is planning subtasks, identifying dependencies, and generating execution steps." />
            <Legend color="bg-brass"      label="Executing" purpose="Agent is actively modifying source files, executing tool commands, or generating code." />
            <Legend color="bg-[#D97706]"  label="Verifying" purpose="Agent is running verification gates, test suites, or AST integrity checks." />
            <Legend color="bg-text-muted" label="Paused" purpose="Agent is suspended awaiting operator feedback or approval." />
            <Legend color="bg-[#8FA339]"  label="Validated" purpose="Agent changes passed verification and are certified safe." />
          </div>
        </div>
      </div>

      <div className="relative min-h-[220px] w-full flex-1 overflow-hidden bg-[#0E0B07]">
        {/* High-Resolution Roman Aqueduct & Castellum Divisorium Backdrop */}
        <img
          src="/assets/romanesque/aqueduct-sluice-cascade-bg.jpg"
          alt=""
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 h-full w-full object-cover opacity-55 select-none"
        />
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(18,13,8,0.25)_0%,rgba(14,10,6,0.72)_100%)]" />

        <svg
          viewBox={`0 0 ${W} ${H}`}
          className="absolute inset-0 h-full w-full"
          preserveAspectRatio="xMidYMid meet"
        >
          <defs>
            <linearGradient id="ag-edge-grad" x1="0" x2="1">
              <stop offset="0" stopColor="rgba(243, 229, 171, 0.95)" />
              <stop offset="0.5" stopColor="rgba(212, 175, 55, 0.85)" />
              <stop offset="1" stopColor="rgba(143, 163, 57, 0.88)" />
            </linearGradient>
            <radialGradient id="ag-root-glow" cx="0.5" cy="0.5" r="0.5">
              <stop offset="0%" stopColor="#F3E5AB" stopOpacity="0.95" />
              <stop offset="55%" stopColor="rgba(212, 175, 55, 0.42)" />
              <stop offset="100%" stopColor="rgba(212, 175, 55, 0)" />
            </radialGradient>
            <filter id="ag-soft-glow">
              <feGaussianBlur stdDeviation="6" />
            </filter>
          </defs>

          {/* Concentric Brass Astrolabe Orbit Rings around Praetorium Root */}
          <circle
            cx={W * 0.5}
            cy={H * 0.42}
            r={Math.min(W, H) * 0.32}
            fill="none"
            stroke="rgba(212, 175, 55, 0.28)"
            strokeWidth="1.5"
            strokeDasharray="6 6"
          />
          <circle
            cx={W * 0.5}
            cy={H * 0.42}
            r={Math.min(W, H) * 0.18}
            fill="none"
            stroke="rgba(243, 229, 171, 0.22)"
            strokeWidth="1.2"
          />

          {/* Aqueduct Hydraulic Conduit Edges */}
          {g.edges.map((e, i) => {
            const a = nodeAt(e.from);
            const b = nodeAt(e.to);
            if (!a || !b) return null;
            const pa = pos(a);
            const pb = pos(b);
            const mx = (pa.x + pb.x) / 2;
            const my = (pa.y + pb.y) / 2 + 34;
            const d = `M ${pa.x} ${pa.y} Q ${mx} ${my} ${pb.x} ${pb.y}`;
            return (
              <g key={i}>
                {/* Outer Carved Stone Aqueduct Channel */}
                <path d={d} stroke="rgba(24, 18, 12, 0.82)" strokeWidth="7" fill="none" />
                <path d={d} stroke="rgba(212, 175, 55, 0.38)" strokeWidth="4.2" fill="none" />
                {/* Animated Hydraulic Token Stream */}
                <path
                  d={d}
                  stroke="url(#ag-edge-grad)"
                  strokeWidth={2.2 + e.flow * 2.0}
                  strokeOpacity="0.96"
                  fill="none"
                  strokeDasharray="10 8"
                >
                  <animate
                    attributeName="stroke-dashoffset"
                    from="36"
                    to="0"
                    dur={`${(1.6 - Math.min(0.8, e.flow * 0.7)).toFixed(2)}s`}
                    repeatCount="indefinite"
                  />
                </path>
              </g>
            );
          })}

          {/* Nodes */}
          {g.nodes.map((n, idx) => {
            const { x, y } = pos(n);
            const isRoot = n.id === 'ROOT';
            const stroke = phaseStroke(n.phase);
            const r = isRoot ? 66 : 44;
            const isSel = sel === n.id;
            const fileByIdx = [
              'crates/vox-compiler/src/typeck',
              'crates/vox-orchestrator/src/router.rs',
              'crates/vox-mens/src/qlora.rs',
              'crates/vox-gui/ui/src/App.tsx',
              'crates/vox-db/src/turso.rs',
              'crates/vox-populi/src/mesh.rs',
            ];
            const modelTag = isRoot
              ? 'CASTELLUM · crates/vox-orchestrator'
              : `${modelByIdx[idx % modelByIdx.length]!} · ${fileByIdx[idx % fileByIdx.length]!}`;
            return (
              <g
                key={n.id}
                role="button"
                tabIndex={0}
                aria-label={`${n.label} — ${n.phase}`}
                aria-pressed={isSel}
                onClick={() => { setSel(n.id); onSelect?.(n.id); }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    setSel(n.id);
                    onSelect?.(n.id);
                  }
                }}
                className="cursor-pointer focus:outline-hidden focus-visible:[outline:2px_solid_#D4AF37]"
              >
                {isRoot && <circle cx={x} cy={y} r={120} fill="url(#ag-root-glow)" />}
                <circle
                  cx={x} cy={y} r={r + 12}
                  fill={stroke} fillOpacity={isSel ? 0.28 : 0.14}
                  filter="url(#ag-soft-glow)"
                />
                {/* Outer Burnished Brass & Carrara Marble Medallion Bezel */}
                <circle
                  cx={x} cy={y} r={r + 3}
                  fill="#F7F2E7"
                  stroke={isSel ? '#8C6825' : '#C59B27'}
                  strokeWidth={isSel ? 2.8 : 2.0}
                />
                <circle
                  cx={x} cy={y} r={r}
                  fill={isRoot ? '#FAF5E8' : '#F3ECE0'}
                  stroke={isSel ? '#8C6825' : '#B88E28'}
                  strokeOpacity={1}
                  strokeWidth={isSel ? 2.2 : 1.5}
                />
                <circle
                  cx={x} cy={y} r={r - 5}
                  fill="none"
                  stroke="#8C6825"
                  strokeOpacity={0.55}
                  strokeDasharray="3 4"
                  style={{ transformOrigin: `${x}px ${y}px` }}
                />
                <text
                  x={x} y={y - 4}
                  textAnchor="middle"
                  fill="#1C140C"
                  fontSize={isRoot ? 10.5 : 10.5}
                  fontWeight="700"
                  fontFamily="Cinzel, Palatino, Georgia, serif"
                  style={{ letterSpacing: '0.03em' }}
                >
                  {n.label}
                </text>
                <text
                  x={x} y={y + 11}
                  textAnchor="middle"
                  fill="#6B4C12"
                  fontSize="9"
                  fontWeight="700"
                  fontFamily="Cinzel, Palatino, Georgia, serif"
                  style={{ textTransform: 'uppercase', letterSpacing: '0.08em' }}
                >
                  {n.phase}
                </text>
                {/* Carrara Marble & Brass LLM Model + Active File Path Sub-Plaque */}
                <rect
                  x={x - (isRoot ? 98 : 88)}
                  y={y + r + 5}
                  width={isRoot ? 196 : 176}
                  height={18}
                  rx={4}
                  fill="rgba(250, 245, 234, 0.96)"
                  stroke="#B38728"
                  strokeWidth="1.2"
                />
                <text
                  x={x}
                  y={y + r + 17}
                  textAnchor="middle"
                  fill="#1E160D"
                  fontSize="8"
                  fontWeight="700"
                  fontFamily="Palatino, 'Palatino Linotype', 'Book Antiqua', Georgia, serif"
                >
                  {modelTag}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      {selectedNode && (
        <div className="shrink-0 border-t-2 border-[#C59B27]/65 bg-[#14100B] p-3">
          <AgentInspector node={selectedNode} agent={selectedAgent} />
          {/* Interactive Developer Orchestrator Control Bar */}
          <div
            data-testid="flow-orchestrator-control-bar"
            style={{
              backgroundImage:
                'linear-gradient(180deg, rgba(244,238,225,0.95) 0%, rgba(229,218,198,0.95) 100%), url(/assets/romanesque/white-marble-frieze-banner.jpg)',
              backgroundSize: 'cover',
              backgroundPosition: 'center',
            }}
            className="mt-2.5 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-[#B38728]/80 px-3 py-1.5 text-[#1C140C] shadow-sm"
          >
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="font-display text-[9px] font-bold uppercase tracking-wider text-[#5C4318]">
                Orchestrator Actions ({selectedNode.label}):
              </span>
              <button
                type="button"
                onClick={() =>
                  setFlowReceipt(`⚔ Dispatched sub-agent task to ${selectedNode.label} via file-affinity router`)
                }
                className="rounded border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB]"
              >
                [⚔ Dispatch Sub-Agent]
              </button>
              <button
                type="button"
                onClick={() =>
                  setFlowReceipt(`🌊 Rerouted ${selectedNode.label} stream to Cursus I Fast Local MENS`)
                }
                className="rounded border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB]"
              >
                [🌊 Reroute Aqueduct Tier]
              </button>
              <button
                type="button"
                onClick={() =>
                  setFlowReceipt(`🔍 Captured BLAKE3 tool-call trace for ${selectedNode.label}`)
                }
                className="rounded border border-[#8C6214]/85 bg-linear-to-b from-[#261D11] to-[#140F08] px-2 py-0.5 font-display text-[8.5px] font-semibold uppercase tracking-wider text-[#F3E5AB]"
              >
                [🔍 Inspect Trace]
              </button>
              <button
                type="button"
                onClick={() => {
                  setFlowYieldXp((xp) => xp + 150);
                  setFlowReceipt(`🏆 Claimed +150 XP · +12 Denarii from ${selectedNode.label} verification!`);
                }}
                className="rounded border border-[#B38728] bg-linear-to-b from-[#D4AF37] to-[#9A711C] px-2.5 py-0.5 font-display text-[8.5px] font-bold uppercase tracking-wider text-[#1A1208]"
              >
                [🏆 Claim Yield +{flowYieldXp} XP]
              </button>
            </div>
            {flowReceipt && (
              <span className="rounded border border-[#8C6214]/50 bg-[#1B140B] px-2 py-0.5 font-mono text-[8.5px] text-[#F3E5AB]">
                {flowReceipt}
              </span>
            )}
          </div>
        </div>
      )}
    </Glass>
  );
}
