import React, { useState } from 'react';
import { Play, Pause, SkipBack, SkipForward, AlertCircle, RotateCcw, Box, Zap, HardDrive } from 'lucide-react';
import { voxTransport } from '../transport';



export function WorkflowScrubber({ snapshots }: any) {
    const [isPlaying, setIsPlaying] = useState(false);

    const workflow = snapshots;

    const handleResume = (stepId: number) => {
        voxTransport.callTool('vox_plan_resume', { step: stepId });
    };

    return (
        <div className="p-10 bg-[#09090b] h-full overflow-y-auto w-full flex flex-col gap-8 text-white">
            <div className="flex justify-between items-center mb-4">
                <div>
                    <h2 className="text-3xl font-black tracking-tighter uppercase mb-2 flex items-center gap-3">
                        <RotateCcw size={28} className="text-blue-500" />
                        Time <span className="text-blue-500">Travel</span>
                    </h2>
                    <p className="text-xs text-zinc-400 font-bold tracking-widest uppercase">Durable Workflow State Inspector</p>
                </div>
                
                <div className="flex items-center gap-4 glass bg-white/[0.02] p-2 rounded-2xl border border-white/5">
                    <button className="w-10 h-10 flex items-center justify-center rounded-xl bg-white/5 hover:bg-white/10 text-zinc-400 transition-colors">
                        <SkipBack size={16} />
                    </button>
                    <button 
                        onClick={() => setIsPlaying(!isPlaying)}
                        className="w-12 h-12 flex items-center justify-center rounded-xl bg-blue-600 text-white shadow-[0_0_20px_rgba(59,130,246,0.3)] hover:scale-105 transition-all"
                    >
                        {isPlaying ? <Pause size={20} /> : <Play size={20} className="ml-1" />}
                    </button>
                    <button className="w-10 h-10 flex items-center justify-center rounded-xl bg-white/5 hover:bg-white/10 text-zinc-400 transition-colors">
                        <SkipForward size={16} />
                    </button>
                    <div className="px-4 text-xs font-mono text-blue-500">T - {workflow?.elapsed_ms ? (workflow.elapsed_ms / 1000).toFixed(1) + 's' : '0.0s'}</div>
                </div>
            </div>

            {!workflow ? (
                <div className="flex-1 flex flex-col items-center justify-center text-zinc-500">
                    <Box size={48} className="mb-4 opacity-50" />
                    <h3 className="text-sm font-bold uppercase tracking-widest text-zinc-400">No Active Workflow</h3>
                    <p className="text-xs mt-2">Durable state execution will appear here when orchestrated.</p>
                </div>
            ) : (
                <div className="grid grid-cols-12 gap-8">
                {/* Timeline */}
                <div className="col-span-8 glass p-8 rounded-[2rem] border border-white/5 flex flex-col">
                    <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-8">Workflow Sequence: <span className="text-white ml-2 font-mono">{workflow.name}</span></h3>
                    
                    <div className="relative pl-8 border-l-2 border-white/10 flex flex-col gap-8">
                        {workflow.steps.map((step, idx) => (
                            <div key={idx} className="relative">
                                {/* Timeline Dot */}
                                <div className={`absolute -left-[37px] w-4 h-4 rounded-full border-4 border-[#09090b] ${
                                    step.status === 'success' ? 'bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]' :
                                    step.status === 'failed' ? 'bg-rose-500 shadow-[0_0_10px_rgba(244,63,94,0.5)] animate-pulse' :
                                    'bg-zinc-700'
                                }`} />
                                
                                <div className="glass bg-white/[0.01] border border-white/5 rounded-2xl p-6 transition-all hover:bg-white/[0.03]">
                                    <div className="flex justify-between items-start mb-4">
                                        <div className="flex items-center gap-3">
                                            <span className={`text-sm font-bold font-mono ${
                                                step.status === 'success' ? 'text-emerald-400' :
                                                step.status === 'failed' ? 'text-rose-400' : 'text-zinc-400'
                                            }`}>
                                                {step.name}
                                            </span>
                                            {step.timeoutLabel && (
                                                <span className="text-[9px] px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20 font-mono">
                                                    {step.timeoutLabel}
                                                </span>
                                            )}
                                        </div>
                                        <span className="text-[10px] font-mono text-zinc-500">{step.time}</span>
                                    </div>

                                    {step.status === 'failed' && (
                                        <div className="bg-rose-500/10 border border-rose-500/20 rounded-xl p-4 mb-4">
                                            <div className="flex items-center gap-2 text-rose-500 mb-2">
                                                <AlertCircle size={14} />
                                                <span className="text-[10px] uppercase tracking-widest font-bold">Activity Trapped Err</span>
                                            </div>
                                            <code className="text-xs text-rose-400 font-mono block mb-3">{step.error}</code>
                                            <div className="flex gap-4 text-[10px] font-bold text-rose-500/70 uppercase tracking-widest">
                                                <span>Retries: {step.retries}/3</span>
                                                <span>Backoff: {step.backoff}</span>
                                            </div>
                                        </div>
                                    )}

                                    {step.status === 'success' && (
                                        <div className="text-[10px] font-mono text-zinc-400 bg-black/50 p-3 rounded-xl border border-white/5">
                                            Output state: <span className="text-emerald-500/70">{step.data}</span>
                                        </div>
                                    )}

                                    {(step.status === 'failed' || step.status === 'pending') && (
                                        <button 
                                            onClick={() => handleResume(step.id)}
                                            className="mt-4 flex items-center gap-2 text-[10px] font-bold uppercase tracking-widest bg-blue-500/10 hover:bg-blue-500/20 text-blue-500 px-4 py-2 rounded-xl transition-all border border-blue-500/20"
                                        >
                                            <Play size={12} />
                                            Resume from this step
                                        </button>
                                    )}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Actor Mailbox Topologies */}
                <div className="col-span-4 flex flex-col gap-8">
                    <div className="glass p-8 rounded-[2rem] border border-white/5">
                        <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-6 flex items-center gap-2">
                            <Box size={16} /> Actor Mailboxes
                        </h3>
                        
                        <div className="flex flex-col gap-4">
                            {workflow.actors.map((actor, idx) => (
                                <div key={idx} className="bg-black/30 p-4 rounded-xl border border-white/5 relative overflow-hidden">
                                    <div className="flex justify-between items-center mb-3">
                                        <span className="text-xs font-mono text-zinc-300">@{actor.name}</span>
                                        <div className="flex items-center gap-1 text-[10px] text-zinc-500 font-bold uppercase tracking-widest">
                                            <HardDrive size={10} /> {actor.stateSize}
                                        </div>
                                    </div>
                                    <div className="flex items-end gap-1 h-3">
                                        {Array.from({ length: Math.min(actor.inboxDepth, 15) }).map((_, i) => (
                                            <div key={i} className={`w-1 h-full rounded-sm ${i > 10 ? 'bg-rose-500' : 'bg-blue-500'}`} />
                                        ))}
                                        {actor.inboxDepth > 15 && <span className="text-[8px] text-rose-500 ml-1 font-bold">+{actor.inboxDepth - 15} msg</span>}
                                        {actor.inboxDepth === 0 && <span className="text-[8px] text-zinc-600 uppercase tracking-widest">Inbox Empty</span>}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>

                    <div className="glass p-8 rounded-[2rem] border border-white/5 flex-1 flex flex-col">
                        <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-4 flex items-center gap-2">
                            <Zap size={16} /> State Diff
                        </h3>
                        <div className="flex-1 bg-[#09090b] rounded-xl border border-white/5 p-4 font-mono text-xs overflow-y-auto">
                            <div className="text-rose-400">- status: "pending"</div>
                            <div className="text-emerald-400">+ status: "failed"</div>
                            <div className="text-emerald-400">+ retry_count: 3</div>
                        </div>
                    </div>
                </div>
            </div>
            )}
        </div>
    );
}
