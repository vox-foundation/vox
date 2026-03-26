import React from 'react';
import { Layers, Terminal, Activity, CheckCircle2, Zap, Target, Cpu, MessageSquare, AlertCircle } from 'lucide-react';
import { motion } from 'framer-motion';
import { getVsCodeApi } from '../utils/vscode';
import { Panel } from './ui/Panel';
import { StateChip } from './ui/StateChip';

const vscode = getVsCodeApi();

const surfaceMuted = 'var(--vscode-descriptionForeground, rgba(161,161,170,1))';
const accentText = 'var(--vscode-textLink-foreground, #60a5fa)';

export const Dashboard = ({ ops = [], stats = {}, pipeline = null }: any) => {
  return (
    <div
      className="p-10 grid grid-cols-12 gap-8 overflow-y-auto h-full"
      style={{
        background: 'var(--vscode-sideBar-background, #09090b)',
        color: 'var(--vscode-sideBar-foreground, #fafafa)',
      }}
    >
      <div className="col-span-12 mb-2">
        <h2 className="text-3xl font-extrabold tracking-tight mb-2 flex items-center gap-3">
          Fleet <span style={{ color: accentText }}>Dashboard</span>
          <div
            className="px-2 py-0.5 rounded border text-[10px] font-bold uppercase tracking-widest"
            style={{
              borderColor: 'var(--vscode-panel-border, rgba(255,255,255,0.08))',
              color: surfaceMuted,
              background: 'var(--vscode-textBlockQuote-background, rgba(255,255,255,0.03))',
            }}
          >
            MCP-backed
          </div>
        </h2>
        <p className="text-sm max-w-2xl font-medium tracking-wide" style={{ color: surfaceMuted }}>
          Orchestrator monitoring and compiler telemetry from the connected MCP server.
        </p>
      </div>

      {/* Primary Stats row */}
      <div className="col-span-3">
        <StatCard title="Active Agents" value={stats.activeAgents ?? "--"} color="blue" icon={<Cpu size={16} />} />
      </div>
      <div className="col-span-3">
        <StatCard title="Queue Depth" value={stats.queueDepth ?? "--"} color="emerald" icon={<Layers size={16} />} />
      </div>
      <div className="col-span-3">
        <StatCard title="Avg Latency" value={stats.latency ?? "--"} color="purple" icon={<Zap size={16} />} />
      </div>
      <div className="col-span-3">
        <StatCard title="Fleet Budget" value={stats.budget ?? "--"} color="amber" icon={<Target size={16} />} />
      </div>

      {/* Main interaction row */}
      <div className="col-span-8">
        <Panel className="h-full !p-8">
           <div className="flex items-center justify-between mb-8">
             <div className="flex items-center gap-3">
               <div
                  className="w-10 h-10 rounded-2xl flex items-center justify-center"
                  style={{
                    background: 'var(--vscode-button-secondaryBackground, rgba(59,130,246,0.1))',
                    color: accentText,
                  }}
               >
                  <Activity size={20} />
               </div>
               <div>
                 <h3 className="text-sm font-bold uppercase tracking-widest leading-tight opacity-90">Operation Stream</h3>
                 <span className="text-[11px] font-medium" style={{ color: surfaceMuted }}>
                   From oplog / task queue (fallback when oplog empty)
                 </span>
               </div>
             </div>
             <button
               type="button"
               className="px-4 py-2 rounded-xl text-[11px] font-bold transition-all uppercase tracking-widest"
               style={{
                  borderWidth: 1,
                  borderStyle: 'solid',
                  borderColor: 'var(--vscode-button-border, rgba(255,255,255,0.12))',
                  background: 'var(--vscode-button-secondaryBackground, transparent)',
                  color: 'var(--vscode-button-secondaryForeground, inherit)',
               }}
             >
               View All Logs
             </button>
           </div>

           <div className="space-y-1">
             {ops && ops.length > 0 ? ops.slice(0, 10).map((entry: any, idx: number) => (
                <OpRow 
                  key={entry.id ?? entry.description ?? idx} 
                  label={entry.description || entry.op_type} 
                  agent={entry.agent_id ?? "--"} 
                  status={entry.status || "Completed"} 
                  time={entry.duration_ms ? `${entry.duration_ms}ms` : "--"} 
                  active={entry.status === 'Running'} 
                />
             )) : (
                <div className="py-8 text-center text-xs font-bold uppercase tracking-widest" style={{ color: surfaceMuted }}>
                  No recent operations
                </div>
             )}
           </div>
        </Panel>
      </div>

      <div className="col-span-4 flex flex-col gap-8">
        <Panel className="flex-1 !p-8">
           <div className="flex items-center gap-3 mb-8">
              <div
                className="w-10 h-10 rounded-2xl flex items-center justify-center"
                style={{
                  background: 'rgba(52, 211, 153, 0.12)',
                  color: 'var(--vscode-testing-iconPassed, #34d399)',
                }}
              >
                 <MessageSquare size={20} />
              </div>
              <h3 className="text-sm font-bold uppercase tracking-widest leading-tight opacity-90">Pipeline Health</h3>
           </div>
           
           <div className="flex flex-col items-center justify-center h-48 py-10 relative">
              {pipeline == null ? (
                <>
                  <AlertCircle size={28} style={{ color: 'var(--vscode-editorWarning-foreground, #eab308)' }} className="mb-2" />
                  <p className="text-[11px] font-bold uppercase tracking-widest mt-2 text-center px-4" style={{ color: surfaceMuted }}>
                    No vox_pipeline_status yet
                  </p>
                  <span className="text-[10px] mt-1 text-center px-4" style={{ color: surfaceMuted }}>
                    Open Pipeline tab after MCP connects
                  </span>
                </>
              ) : pipeline && typeof pipeline === 'object' && (pipeline as { ok?: boolean }).ok === false ? (
                <>
                  <AlertCircle size={32} className="text-amber-500 mb-2" />
                  <p className="text-[11px] font-bold text-amber-500 uppercase tracking-widest mt-2 text-center px-4">Compiler pipeline reported issues</p>
                  <span className="text-[10px] text-zinc-500 mt-1 text-center px-4">See Pipeline tab for details</span>
                </>
              ) : (
                <>
              <div
                className="w-16 h-16 rounded-full flex items-center justify-center mb-4 border"
                style={{
                  borderColor: 'var(--vscode-testing-iconPassed, #34d399)',
                  color: 'var(--vscode-testing-iconPassed, #34d399)',
                  background: 'var(--vscode-textfield-background, rgba(52,211,153,0.06))',
                }}
              >
                 <CheckCircle2 size={32} />
              </div>
              <p className="text-[11px] font-bold uppercase tracking-widest mt-2" style={{ color: 'var(--vscode-testing-iconPassed, #34d399)' }}>Pipeline OK</p>
              <span className="text-[10px] mt-1" style={{ color: surfaceMuted }}>From vox_pipeline_status</span>
                </>
              )}
           </div>
        </Panel>

        {/* Action Quicklinks */}
        <div className="grid grid-cols-2 gap-4">
           <ActionBtn icon={<Terminal size={16} />} label="Fmt Build" onClick={() => vscode.postMessage({ type: 'runCommand', value: 'vox.build' })} />
           <ActionBtn icon={<Layers size={16} />} label="Rebalance" onClick={() => vscode.postMessage({ type: 'rebalance' })} />
        </div>
      </div>
    </div>
  );
};

const OpRow = ({ label, agent, status, time, active: _active }: any) => (
  <motion.div 
    initial={false}
    animate={{ opacity: 1 }}
    className="group flex items-center justify-between py-4 border-b last:border-0 -mx-4 px-4 rounded-2xl transition-all duration-300"
    style={{ borderColor: 'var(--vscode-panel-border, rgba(255,255,255,0.06))' }}
  >
    <div className="flex items-center gap-4">
      <div className={`w-10 h-10 rounded-xl flex items-center justify-center transition-all ${
        status === 'Success' ? 'bg-emerald-500/5 text-emerald-500 group-hover:bg-emerald-500/10' : 
        status === 'Running' ? 'bg-blue-500/10 text-blue-500 animate-pulse border border-blue-500/20 shadow-[0_0_15px_rgba(59,130,246,0.1)]' : 
        'bg-zinc-500/10 text-zinc-500'
      }`}>
         <Terminal size={16} />
      </div>
      <div>
        <p className="text-sm font-bold opacity-90 group-hover:opacity-100 transition-colors tracking-tight">{label}</p>
        <span className="text-[10px] font-bold uppercase tracking-widest" style={{ color: surfaceMuted }}>@ {agent}</span>
      </div>
    </div>
    
    <div className="flex items-center gap-8">
       <span className="text-[11px] font-mono font-medium" style={{ color: surfaceMuted }}>{time}</span>
       <StateChip label={String(status)} tone={opRowTone(String(status))} />
    </div>
  </motion.div>
)

const StatCard = ({ title, value, delta, color: _color, icon }: any) => (
  <motion.div 
    whileHover={{ y: -4, scale: 1.02 }}
    className="rounded-[2rem] p-8 transition-all cursor-default group relative overflow-hidden"
    style={{
      background: 'var(--vscode-editorWidget-background, rgba(16,16,18,1))',
      borderWidth: 1,
      borderStyle: 'solid',
      borderColor: 'var(--vscode-panel-border, rgba(39,39,42,1))',
    }}
  >
     <div className="absolute -top-10 -right-10 w-32 h-32 blur-[40px] rounded-full transition-all duration-700 opacity-40" style={{ background: accentText }} />
     
     <div className="flex justify-between items-center mb-6">
       <div
         className="w-8 h-8 rounded-lg flex items-center justify-center transition-all"
         style={{
           background: 'var(--vscode-toolbar-hoverBackground, rgba(255,255,255,0.05))',
           borderWidth: 1,
           borderStyle: 'solid',
           borderColor: 'var(--vscode-panel-border)',
           color: surfaceMuted,
         }}
       >
          {icon}
       </div>
       {delta && <span className={`text-[10px] font-extrabold tracking-widest uppercase ${delta.startsWith('-') ? 'text-rose-500' : 'text-emerald-500'} bg-white/[0.02] px-2 py-0.5 rounded-full border border-white/5`}>{delta}</span>}
     </div>
     
     <div className="flex flex-col">
       <span className="text-[10px] font-bold mb-1 uppercase tracking-widest" style={{ color: surfaceMuted }}>{title}</span>
       <span className="text-4xl font-black tracking-tighter transition-all duration-300 group-hover:opacity-90">{value}</span>
     </div>
  </motion.div>
);

const ActionBtn = ({ icon, label, onClick }: any) => (
  <button
    type="button"
    onClick={onClick}
    className="flex-1 rounded-2xl py-5 px-4 flex flex-col items-center gap-3 group transition-all"
    style={{
      borderWidth: 1,
      borderStyle: 'solid',
      borderColor: 'var(--vscode-panel-border)',
      background: 'var(--vscode-button-secondaryBackground, transparent)',
    }}
  >
     <div className="transition-colors" style={{ color: surfaceMuted }}>{icon}</div>
     <span className="text-[10px] font-bold uppercase tracking-widest" style={{ color: surfaceMuted }}>{label}</span>
  </button>
)
