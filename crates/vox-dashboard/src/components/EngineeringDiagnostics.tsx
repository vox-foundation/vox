import React, { useState } from 'react';
import { AgentFlow } from './AgentFlow';
import { AstView } from './AstView';
import { IntentionMatrix } from './IntentionMatrix';
import { WorkflowScrubber } from './WorkflowScrubber';
import { ContextExplorer } from './ContextExplorer';
import { Activity, LayoutTemplate, Network, Wrench, ShieldAlert, Sparkles, BrainCircuit } from 'lucide-react';
import { voxTransport } from '../transport';

export const EngineeringDiagnostics = ({ 
    tasks, capabilities, ast, activeFile, intentionMatrix, voxStatus, workflowStatus, inspectorState 
}: any) => {
    const [subTab, setSubTab] = useState<'flow' | 'chronicle' | 'intentions' | 'ast' | 'context' | 'tools' | 'mens'>('flow');

    const subTabs = [
        { id: 'flow', label: 'FLOW', icon: <Network size={14} /> },
        { id: 'chronicle', label: 'CHRONICLE', icon: <Sparkles size={14} /> },
        { id: 'intentions', label: 'INTENTIONS', icon: <LayoutTemplate size={14} /> },
        { id: 'ast', label: 'DOCTOR / AST', icon: <Activity size={14} /> },
        { id: 'context', label: 'CONTEXT / VAULT', icon: <ShieldAlert size={14} /> },
        { id: 'tools', label: 'SKILLS / MCP', icon: <Wrench size={14} /> },
        { id: 'mens', label: 'MENS ML', icon: <BrainCircuit size={14} /> },
    ];

    return (
        <div className="flex flex-col h-full w-full bg-background overflow-hidden border-t border-border">
            <div className="p-4 pb-0 shrink-0">
                <h2 className="text-2xl font-rajdhani text-brass tracking-wider uppercase mb-3">FABRICA</h2>
                <div className="flex flex-wrap gap-2 mb-4 border-b border-border pb-4">
                    {subTabs.map((t) => (
                        <button
                            key={t.id}
                            onClick={() => setSubTab(t.id as any)}
                            className={`flex items-center gap-2 px-3 py-1.5 rounded border text-[10px] font-bold tracking-widest uppercase transition-all ${
                                subTab === t.id 
                                    ? 'bg-cyan bg-opacity-10 border-cyan text-cyan shadow-[0_0_8px_var(--vox-cyan-glow)]'
                                    : 'bg-machine border-border text-steel hover:text-foreground hover:bg-surface'
                            }`}
                        >
                            {t.icon} {t.label}
                        </button>
                    ))}
                </div>
            </div>

            <div className="flex-1 overflow-y-auto p-4 pt-0 pb-20 custom-scrollbar text-foreground relative z-10 w-full min-h-0">
                {subTab === 'flow' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col">
                        <div className="bg-machine px-4 py-2 border-b border-border text-xs font-rajdhani font-bold uppercase tracking-widest text-brass">
                            Execution Flow
                        </div>
                        <div className="flex-1 relative">
                            <AgentFlow tasks={tasks} capabilities={capabilities} />
                        </div>
                    </div>
                )}

                {subTab === 'ast' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col">
                        <div className="bg-machine px-4 py-2 border-b border-border text-xs font-rajdhani font-bold uppercase tracking-widest text-brass">
                            AST Inspector Tracker
                        </div>
                        <div className="flex-1 relative">
                            <AstView ast={ast} activeFile={activeFile} />
                        </div>
                    </div>
                )}

                {subTab === 'intentions' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col">
                        <div className="bg-machine px-4 py-2 border-b border-border text-xs font-rajdhani font-bold uppercase tracking-widest text-brass">
                            Orchestrator Intention Matrix
                        </div>
                        <div className="flex-1 relative p-4 overflow-y-auto custom-scrollbar">
                            <IntentionMatrix intents={intentionMatrix} socratesStatus={voxStatus} />
                        </div>
                    </div>
                )}

                {subTab === 'chronicle' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col">
                        <div className="bg-machine px-4 py-2 border-b border-border text-xs font-rajdhani font-bold uppercase tracking-widest text-brass">
                            Workflow Scrubber
                        </div>
                        <div className="flex-1 relative p-4 overflow-y-auto custom-scrollbar">
                            <WorkflowScrubber snapshots={workflowStatus} />
                        </div>
                    </div>
                )}
                
                {subTab === 'context' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col">
                        <div className="bg-machine px-4 py-2 border-b border-border text-xs font-rajdhani font-bold uppercase tracking-widest text-brass flex items-center justify-between">
                            <span>Context Explorer</span>
                        </div>
                        <div className="flex-1 relative p-4 overflow-y-auto custom-scrollbar">
                             <ContextExplorer inspector={inspectorState} />
                        </div>
                    </div>
                )}

                {subTab === 'tools' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col">
                        <div className="bg-machine px-4 py-2 border-b border-border text-xs font-rajdhani font-bold uppercase tracking-widest text-brass flex items-center justify-between">
                            <span>Tool Registry & Capabilities</span>
                            <div className="flex gap-4 items-center">
                                <span className="opacity-80 font-mono text-cyan lowercase">tools: {capabilities?.toolCount ?? 0}</span>
                                <button 
                                    onClick={() => voxTransport.callTool('vox_orchestrator_start', {})}
                                    className="px-2 py-1 text-[9px] bg-surface rounded border border-border text-steel hover:text-white hover:border-copper transition-colors"
                                >
                                    PROBE SERVER
                                </button>
                            </div>
                        </div>
                        <div className="flex-1 relative p-4 overflow-y-auto custom-scrollbar">
                            {Array.isArray(capabilities?.loadedSchemas) && capabilities.loadedSchemas.length > 0 ? (
                                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    {capabilities.loadedSchemas.map((tool: any, idx: number) => (
                                        <div key={idx} className="p-3 border border-border rounded bg-machine shadow-[inset_0_2px_5px_rgba(0,0,0,0.5)]">
                                            <div className="font-bold text-[11px] font-mono text-copper uppercase tracking-widest">{tool.name}</div>
                                            {tool.description && <div className="text-[10px] mt-2 text-steel leading-relaxed col-span-1">{tool.description}</div>}
                                        </div>
                                    ))}
                                </div>
                            ) : (
                                <div className="flex h-full items-center justify-center text-[10px] text-steel font-mono uppercase tracking-widest">
                                    No tools discovered explicitly by client schema
                                </div>
                            )}
                        </div>
                    </div>
                )}

                {subTab === 'mens' && (
                    <div className="min-h-[500px] flex-1 border border-border rounded-xl overflow-hidden bg-surface shadow-[inset_0_0_10px_rgba(0,0,0,0.5)] flex flex-col items-center justify-center p-8 text-center relative">
                        <BrainCircuit size={48} className="text-primary mb-4 drop-shadow-[0_0_15px_var(--vox-amber-glow)]" />
                        <h3 className="font-rajdhani text-xl text-brass tracking-widest uppercase mb-2">Mens ML Training</h3>
                        <p className="text-steel font-mono text-[10px] leading-relaxed max-w-sm mb-6">
                            Local model refinement pipeline. Synchronize FableForge schemas and Vox QLoRA datasets to continuously align Populi reasoning behavior.
                        </p>
                        <button 
                            onClick={() => voxTransport.callTool('vox_schola_submit', { description: 'Initiate training cycle via dashboard' })}
                            className="px-6 py-2 bg-primary text-black font-rajdhani font-bold text-sm tracking-widest uppercase border border-transparent hover:bg-amber-400 hover:border-black shadow-[0_0_10px_var(--vox-amber-glow)] transition-all rounded"
                        >
                            INITIATE TRAINING CYCLE
                        </button>
                    </div>
                )}
            </div>
        </div>
    );
};
