import React from 'react';
import { AgentFlow } from './AgentFlow';
import { AstView } from './AstView';
import { IntentionMatrix } from './IntentionMatrix';
import { WorkflowScrubber } from './WorkflowScrubber';
import { ContextExplorer } from './ContextExplorer';

export const EngineeringDiagnostics = ({ 
    tasks, capabilities, ast, activeFile, intentionMatrix, voxStatus, workflowStatus, inspectorState 
}: any) => {
    return (
        <div className="flex flex-col h-full overflow-y-auto gap-4 p-4 text-[var(--vscode-editor-foreground)]">
            <h2 className="text-xl font-bold tracking-tight mb-2">Systems & Diagnostics</h2>
            
            <div className="min-h-[400px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden bg-[var(--vscode-editor-background)]">
                <div className="bg-[var(--vscode-sideBar-background)] px-4 py-2 border-b border-[var(--vscode-panel-border)] text-xs font-bold uppercase tracking-widest">
                    Execution Flow
                </div>
                <div className="h-[360px] relative">
                    <AgentFlow tasks={tasks} capabilities={capabilities} />
                </div>
            </div>

            <div className="min-h-[300px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden bg-[var(--vscode-editor-background)]">
                <div className="bg-[var(--vscode-sideBar-background)] px-4 py-2 border-b border-[var(--vscode-panel-border)] text-xs font-bold uppercase tracking-widest">
                    AST Inspector Tracker
                </div>
                <div className="h-[260px] relative">
                    <AstView ast={ast} activeFile={activeFile} />
                </div>
            </div>

            <div className="min-h-[300px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden bg-[var(--vscode-editor-background)]">
                <div className="bg-[var(--vscode-sideBar-background)] px-4 py-2 border-b border-[var(--vscode-panel-border)] text-xs font-bold uppercase tracking-widest">
                    Orchestrator Intention Matrix
                </div>
                <div className="h-[260px] relative p-4 overflow-y-auto">
                    <IntentionMatrix intents={intentionMatrix} socratesStatus={voxStatus} />
                </div>
            </div>

            <div className="min-h-[300px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden bg-[var(--vscode-editor-background)]">
                <div className="bg-[var(--vscode-sideBar-background)] px-4 py-2 border-b border-[var(--vscode-panel-border)] text-xs font-bold uppercase tracking-widest">
                    Workflow Scrubber
                </div>
                <div className="h-[260px] relative p-4 overflow-y-auto">
                    <WorkflowScrubber snapshots={workflowStatus} />
                </div>
            </div>
            
            <div className="min-h-[300px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden bg-[var(--vscode-editor-background)]">
                <div className="bg-[var(--vscode-sideBar-background)] px-4 py-2 border-b border-[var(--vscode-panel-border)] text-xs font-bold uppercase tracking-widest flex items-center justify-between">
                    <span>Context Explorer</span>
                </div>
                <div className="h-[260px] relative p-4 overflow-y-auto">
                     <ContextExplorer inspector={inspectorState} />
                </div>
            </div>

            <div className="min-h-[300px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden bg-[var(--vscode-editor-background)]">
                <div className="bg-[var(--vscode-sideBar-background)] px-4 py-2 border-b border-[var(--vscode-panel-border)] text-xs font-bold uppercase tracking-widest flex items-center justify-between">
                    <span>Tool Registry & Capabilities</span>
                    <span className="opacity-60 font-mono lowercase">tools: {capabilities?.toolCount ?? 0}</span>
                </div>
                <div className="h-[260px] relative p-4 overflow-y-auto">
                    {Array.isArray(capabilities?.loadedSchemas) && capabilities.loadedSchemas.length > 0 ? (
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            {capabilities.loadedSchemas.map((tool: any, idx: number) => (
                                <div key={idx} className="p-3 border border-[var(--vscode-editorGroup-border)] rounded bg-[var(--vscode-textBlockQuote-background)]">
                                    <div className="font-bold text-[11px] font-mono text-[var(--vscode-symbolIcon-functionForeground)]">{tool.name}</div>
                                    {tool.description && <div className="text-[10px] mt-2 opacity-70 leading-relaxed">{tool.description}</div>}
                                </div>
                            ))}
                        </div>
                    ) : (
                        <div className="flex h-full items-center justify-center text-xs opacity-50 uppercase tracking-widest">
                            No tools discovered explicitly by client schema
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};
