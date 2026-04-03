import React, { useState } from 'react';
import { Terminal, Activity, Trophy, Server, Bell, Cpu, Layers, MessageSquare, AlertCircle, CheckCircle2 } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { getVsCodeApi } from '../utils/vscode';
import { Panel } from './ui/Panel';
import { StateChip } from './ui/StateChip';

const vscode = getVsCodeApi();

function opRowTone(status: string): 'success' | 'warning' | 'danger' | 'neutral' | 'info' {
    const normalized = status.toLowerCase();
    if (normalized.includes('run')) return 'info';
    if (normalized.includes('success') || normalized.includes('complete')) return 'success';
    if (normalized.includes('fail') || normalized.includes('error')) return 'danger';
    if (normalized.includes('queue') || normalized.includes('block')) return 'warning';
    return 'neutral';
}

export const UnifiedDashboard = ({
    ops = [],
    stats = {},
    pipeline = null,
    budgetHistory = [],
    modelList = [],
    ludusSnapshot = null,
    meshTopology = null,
}: {
    ops: any[];
    stats: any;
    pipeline: any;
    budgetHistory: any[];
    modelList: any[];
    ludusSnapshot: Record<string, unknown> | null;
    meshTopology: any;
}) => {
    return (
        <div className="p-4 grid grid-cols-12 gap-4 overflow-y-auto flex-1 min-h-0 text-[var(--vscode-sideBar-foreground)]">
            {/* Header Area */}
            <div className="col-span-12 flex justify-between items-center mb-2">
                <h2 className="text-xl font-bold tracking-tight">Unified Command Center</h2>
                <div className="flex gap-2 text-xs font-mono opacity-80">
                    <span className="flex items-center gap-1"><Cpu size={14} /> Active Agents: {stats.activeAgents ?? "0"}</span>
                    <span className="flex items-center gap-1"><Layers size={14} /> Queue: {stats.queueDepth ?? "0"}</span>
                    {stats.budget && <span className="flex items-center gap-1"><Activity size={14} /> Budget: {stats.budget}</span>}
                </div>
            </div>

            {/* Ludus KPI & Budget Header Widget (Only if data exists) */}
            {(ludusSnapshot?.kpi || budgetHistory.length > 0) && (
                <div className="col-span-12 grid grid-cols-12 gap-4">
                    {ludusSnapshot?.kpi && (
                        <Panel className="col-span-6 !p-4 flex gap-4 items-center">
                            <Trophy size={20} className="text-[var(--vscode-charts-orange)]" />
                            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 flex-1 text-xs">
                                <div><div className="opacity-60 text-[10px] uppercase">Events</div>{String((ludusSnapshot.kpi as any)?.events_recorded ?? '0')}</div>
                                <div><div className="opacity-60 text-[10px] uppercase">XP</div>{String((ludusSnapshot.kpi as any)?.total_xp_from_policy ?? (ludusSnapshot.kpi as any)?.total_xp ?? '0')}</div>
                                <div><div className="opacity-60 text-[10px] uppercase">Crystals</div>{String((ludusSnapshot.kpi as any)?.total_crystals_from_policy ?? (ludusSnapshot.kpi as any)?.total_crystals ?? '0')}</div>
                                <div><div className="opacity-60 text-[10px] uppercase">Streak</div>{String((ludusSnapshot.kpi as any)?.streak_days ?? '0')}</div>
                            </div>
                        </Panel>
                    )}
                    
                    {budgetHistory.length > 0 && (
                        <Panel className="col-span-6 !p-4 flex gap-4 items-center">
                            <Activity size={20} className="text-[var(--vscode-charts-green)]" />
                            <div className="flex-1 text-xs overflow-hidden text-ellipsis whitespace-nowrap">
                                <div className="opacity-60 text-[10px] uppercase mb-1">Recent Budget Spend</div>
                                ${parseFloat(budgetHistory[budgetHistory.length - 1]?.total_cost_usd || 0).toFixed(4)}
                            </div>
                        </Panel>
                    )}
                </div>
            )}

            {/* Notifications (Ludus) */}
            {(ludusSnapshot?.notifications as any[])?.length > 0 && (
                <div className="col-span-12">
                    <Panel className="!p-4 border-[var(--vscode-testing-iconFailed)] bg-[var(--vscode-editorError-background)]">
                        <div className="flex justify-between items-center mb-2">
                            <div className="flex items-center gap-2 font-bold text-[11px] uppercase tracking-wider">
                                <Bell size={14} /> Unread Notifications
                            </div>
                            <button
                                type="button"
                                className="text-[10px] font-bold uppercase tracking-wider px-2 py-1 rounded-md border-[1px] border-[var(--vscode-button-border)] bg-[var(--vscode-button-secondaryBackground)]"
                                onClick={() => vscode.postMessage({ type: 'ludusAckAllNotifications' })}
                            >
                                Ack All
                            </button>
                        </div>
                        <div className="space-y-1">
                            {((ludusSnapshot?.notifications as any) || []).map((n: any, i: number) => (
                                <div key={n.id ?? i} className="flex gap-2 items-center text-xs opacity-90 p-2 rounded bg-[var(--vscode-editor-background)]">
                                    <div className="flex-1 truncate"><strong>{n.title ?? n.heading}</strong> - {n.body ?? n.message}</div>
                                    <button
                                        type="button"
                                        className="text-[10px] px-2 py-1 bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)] rounded hover:opacity-80"
                                        onClick={() => vscode.postMessage({ type: 'ludusAckNotification', notificationId: String(n.id ?? '') })}
                                    >
                                        Ack
                                    </button>
                                </div>
                            ))}
                        </div>
                    </Panel>
                </div>
            )}

            {/* Main Operations Stream */}
            <div className="col-span-8">
                <Panel className="flex-1 !p-4 flex flex-col min-h-[400px]">
                    <div className="flex items-center justify-between mb-4">
                        <div className="flex items-center gap-2">
                            <Terminal size={16} className="text-[var(--vscode-charts-blue)]" />
                            <h3 className="text-sm font-bold uppercase tracking-widest opacity-90">Operation Stream</h3>
                        </div>
                        <div className="flex gap-2">
                            <button
                                type="button"
                                className="px-3 py-1 rounded text-[10px] font-bold transition-all uppercase tracking-widest border border-[var(--vscode-button-border)] bg-[var(--vscode-button-secondaryBackground)] hover:bg-[var(--vscode-button-secondaryHoverBackground)]"
                                onClick={() => vscode.postMessage({ type: 'rebalance' })}
                            >
                                Rebalance
                            </button>
                        </div>
                    </div>
                    
                    <div className="space-y-2 flex-1 overflow-y-auto pr-2">
                        {ops && ops.length > 0 ? ops.slice(0, 15).map((entry: any, idx: number) => (
                            <div 
                                key={entry.id ?? entry.description ?? idx} 
                                className="flex items-center justify-between p-3 rounded-lg border border-[var(--vscode-panel-border)] bg-[var(--vscode-editor-background)]"
                            >
                                <div className="flex-1 min-w-0 pr-4">
                                    <div className="text-sm font-bold truncate">{entry.description || entry.op_type}</div>
                                    {entry.agent_id && (
                                        <div className="text-[10px] font-mono opacity-60">@ {entry.agent_id}</div>
                                    )}
                                </div>
                                <div className="flex items-center gap-4 shrink-0">
                                    {entry.duration_ms && <span className="text-xs font-mono opacity-60">{entry.duration_ms}ms</span>}
                                    <StateChip label={entry.status || "Completed"} tone={opRowTone(entry.status || "Completed")} />
                                </div>
                            </div>
                        )) : (
                            <div className="flex items-center justify-center h-full text-xs font-bold uppercase tracking-widest opacity-40">
                                No recent operations
                            </div>
                        )}
                    </div>
                </Panel>
            </div>

            {/* Right column: Pipeline Health, Mesh Topology */}
            <div className="col-span-4 flex flex-col gap-4">
                <Panel className="!p-4">
                    <div className="flex items-center gap-2 mb-4">
                        <MessageSquare size={16} className="text-[var(--vscode-charts-purple)]" />
                        <h3 className="text-sm font-bold uppercase tracking-widest opacity-90">Pipeline Health</h3>
                    </div>
                    <div className="flex flex-col items-center justify-center py-4">
                        {pipeline == null ? (
                            <div className="text-center opacity-60 text-xs">Awaiting Pipeline Status</div>
                        ) : pipeline.ok === false ? (
                            <>
                                <AlertCircle size={32} className="text-[var(--vscode-editorError-foreground)] mb-2" />
                                <div className="text-xs font-bold text-[var(--vscode-editorError-foreground)] text-center">Pipeline Errors Detected</div>
                            </>
                        ) : (
                            <>
                                <CheckCircle2 size={32} className="text-[var(--vscode-testing-iconPassed)] mb-2" />
                                <div className="text-xs font-bold text-[var(--vscode-testing-iconPassed)]">Pipeline OK</div>
                            </>
                        )}
                    </div>
                </Panel>
                
                {meshTopology && Object.keys(meshTopology).length > 0 && (
                    <Panel className="!p-4 flex-1">
                        <div className="flex items-center gap-2 mb-4">
                            <Server size={16} className="text-[var(--vscode-charts-yellow)]" />
                            <h3 className="text-sm font-bold uppercase tracking-widest opacity-90">Mesh Nodes Node</h3>
                        </div>
                        <div className="text-xs space-y-2 opacity-80 overflow-y-auto">
                           {Array.isArray(meshTopology.nodes) && meshTopology.nodes.length > 0 
                               ? meshTopology.nodes.map((node: any, idx: number) => (
                                    <div key={idx} className="flex justify-between border-b border-[var(--vscode-panel-border)] pb-1">
                                        <span>{node.id || node.name || `Node ${idx}`}</span>
                                        <span className="font-mono">{node.status || 'Active'}</span>
                                    </div>
                               )) 
                               : <div>Mesh data attached but no nodes visible.</div>
                           }
                        </div>
                    </Panel>
                )}
            </div>
        </div>
    );
};
