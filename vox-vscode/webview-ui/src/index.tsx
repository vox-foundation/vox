import React, { useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { MessageSquare, LayoutDashboard, Activity as ActivityIcon, Settings2, Sparkles, Terminal } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import './index.css';
import { getVsCodeApi } from './utils/vscode';
import { parseHostToWebviewMessage } from '../../src/protocol/hostToWebviewMessages';
import type { ChatSessionMeta, ComposerState, WorkspaceInspectorState } from '../../src/types';

import { UnifiedDashboard } from './components/UnifiedDashboard';
import { EngineeringDiagnostics } from './components/EngineeringDiagnostics';
import { ComposerPanel } from './components/ComposerPanel';

import { ErrorBoundary } from './components/ErrorBoundary';
import { CodeBlock } from './components/CodeBlock';

const vscode = getVsCodeApi();

type TabId = 'dashboard' | 'chat' | 'diagnostics';

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('chat');
  
  // Data states
  const [voxStatus, setVoxStatus] = useState<any>(null);
  const [gamify, setGamify] = useState<any>(null);
  const [ast, setAst] = useState<any>(null);
  const [pipeline, setPipeline] = useState<any>(null);
  const [activeFile, setActiveFile] = useState<string>('');
  const [tasks, setTasks] = useState<any[]>([]);
  const [workflowStatus, setWorkflowStatus] = useState<any>(null);
  const [meshStatus, setMeshStatus] = useState<any>(null);
  const [intentionMatrix, setIntentionMatrix] = useState<any>(null);
  const [oplog, setOplog] = useState<any[]>([]);
  const [budgetHistory, setBudgetHistory] = useState<any[]>([]);
  const [modelList, setModelList] = useState<any[]>([]);
  const [agents, setAgents] = useState<any[]>([]);
  const [capabilities, setCapabilities] = useState<any>(null);
  const [ludusSnapshot, setLudusSnapshot] = useState<Record<string, unknown> | null>(null);
  const [chatMessages, setChatMessages] = useState<any[]>([]);
  const [chatMeta, setChatMeta] = useState<ChatSessionMeta | null>(null);
  const [workspaceContext, setWorkspaceContext] = useState<any>(null);
  const [composerState, setComposerState] = useState<ComposerState | null>(null);
  const [inspectorState, setInspectorState] = useState<WorkspaceInspectorState | null>(null);
  const [planAdequacyQuestions, setPlanAdequacyQuestions] = useState<string[]>([]);

  // Local UI states
  const [chatInput, setChatInput] = useState<string>('');
  const [chatSessionId, setChatSessionId] = useState<string>('vscode-sidebar');
  const [chatProfile, setChatProfile] = useState<'fast' | 'reasoning' | 'creative'>('reasoning');
  const [pinnedFiles, setPinnedFiles] = useState<string[]>([]);
  const [composerVisible, setComposerVisible] = useState(false);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const parsed = parseHostToWebviewMessage(event.data);
      if (!parsed) return;
      switch (parsed.type) {
        case 'voxStatus': setVoxStatus(parsed.value); break;
        case 'gamifyUpdate': setGamify(parsed.value); break;
        case 'astResult': setAst(parsed.value); break;
        case 'pipelineStatus': setPipeline(parsed.value); break;
        case 'activeEditorChanged': setActiveFile(String(parsed.value ?? '')); break;
        case 'a2aTasks': setTasks(Array.isArray(parsed.value) ? parsed.value : []); break;
        case 'budgetHistory': if (parsed.value) setBudgetHistory(parsed.value as any[]); break;
        case 'modelList': if (parsed.value) setModelList(parsed.value as any[]); break;
        case 'workflowStatus': setWorkflowStatus(parsed.value); break;
        case 'meshStatus': setMeshStatus(parsed.value); break;
        case 'intentionMatrix': setIntentionMatrix(parsed.value); break;
        case 'oplog': if (parsed.value) setOplog(parsed.value as any[]); break;
        case 'agentsUpdate': if (parsed.value) setAgents(parsed.value as any[]); break;
        case 'capabilitiesUpdate': setCapabilities(parsed.value); break;
        case 'ludusProgressSnapshot':
          if (parsed.value && typeof parsed.value === 'object' && !Array.isArray(parsed.value)) {
            setLudusSnapshot(parsed.value as Record<string, unknown>);
          }
          break;
        case 'chatHistory': setChatMessages(Array.isArray(parsed.value) ? parsed.value : []); break;
        case 'chatMeta': setChatMeta((parsed.value as ChatSessionMeta) ?? null); break;
        case 'workspaceContext': setWorkspaceContext(parsed.value); break;
        case 'composerState': setComposerState((parsed.value as ComposerState) ?? null); break;
        case 'inspectorState': setInspectorState((parsed.value as WorkspaceInspectorState) ?? null); break;
        case 'planAdequacyQuestions': {
          const pv = parsed.value as { questions?: string[]; score?: number } | undefined;
          if (Array.isArray(pv?.questions) && pv.questions.length > 0) {
            setPlanAdequacyQuestions(pv.questions);
          }
          break;
        }
      }
    };
    window.addEventListener('message', handler);
    vscode.postMessage({ type: 'getInitialData' });
    return () => window.removeEventListener('message', handler);
  }, []);

  useEffect(() => {
    const files = workspaceContext?.openFiles;
    if (!Array.isArray(files) || files.length === 0) return;
    if (pinnedFiles.length === 0) {
      setPinnedFiles(files.slice(0, Math.min(3, files.length)));
    }
  }, [workspaceContext, pinnedFiles.length]);

  const orch = gamify as Record<string, unknown> | null;
  const agentCount = typeof orch?.agent_count === 'number' 
      ? orch.agent_count 
      : Array.isArray(agents) ? agents.filter((agent) => agent.status === 'working').length : 0;
  
  const dashboardStats = {
    activeAgents: String(agentCount),
    queueDepth: tasks.length ? tasks.length.toString() : null,
    latency: voxStatus?.avg_latency_ms ? `${voxStatus.avg_latency_ms}ms` : null,
    budget: voxStatus?.total_cost_usd != null ? `$${voxStatus.total_cost_usd.toFixed(2)}` : null,
  };

  const togglePinnedFile = (file: string) => {
    setPinnedFiles((prev) => (prev.includes(file) ? prev.filter((entry) => entry !== file) : [...prev, file]));
  };

  const execHint =
    capabilities?.execution_mode === 'queue_only'
      ? 'Orchestrator: queue-only (no worker handles)'
      : capabilities?.execution_mode === 'workers_attached'
        ? 'Orchestrator: workers attached'
        : capabilities?.execution_mode
          ? `Orchestrator: ${String(capabilities.execution_mode)}`
          : capabilities?.mcpConnected === false
            ? 'MCP: disconnected'
            : 'Orchestrator: status unknown (no snapshot yet)';

  const renderContent = () => {
    switch (activeTab) {
      case 'dashboard':
        return (
            <UnifiedDashboard 
                ops={oplog} 
                stats={dashboardStats} 
                pipeline={pipeline} 
                budgetHistory={budgetHistory} 
                modelList={modelList} 
                ludusSnapshot={ludusSnapshot} 
                meshTopology={meshStatus} 
            />
        );
      case 'diagnostics':
        return (
            <EngineeringDiagnostics 
                tasks={tasks} 
                capabilities={capabilities} 
                ast={ast} 
                activeFile={activeFile} 
                intentionMatrix={intentionMatrix} 
                voxStatus={voxStatus} 
                workflowStatus={workflowStatus} 
                inspectorState={inspectorState} 
            />
        );
      case 'chat':
        return (
          <div className="flex flex-col h-full min-h-0 p-4 gap-3 text-[var(--vscode-editor-foreground)]">
            <div className="flex items-center justify-between">
                <h2 className="text-xl font-bold tracking-tight">Vox Assistant</h2>
                <button 
                   onClick={() => setComposerVisible(!composerVisible)}
                   className={`px-3 py-1 rounded text-xs font-bold uppercase tracking-widest border border-[var(--vscode-button-border)] flex items-center gap-1 transition-all ${composerVisible ? 'bg-[var(--vscode-charts-blue)] text-white' : 'bg-[var(--vscode-button-secondaryBackground)]'}`}
                >
                    <Sparkles size={12} /> Composer
                </button>
            </div>

            {composerVisible && composerState && (
                <div className="shrink-0 h-[300px] border border-[var(--vscode-panel-border)] rounded-xl overflow-hidden relative shadow-lg z-10 bg-[var(--vscode-editor-background)]">
                   <ComposerPanel composerState={composerState} />
                </div>
            )}

            <div className="rounded-xl border border-[var(--vscode-panel-border)] p-3 bg-[var(--vscode-editorWrapper-background)]">
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className="rounded border border-[var(--vscode-input-border)] bg-[var(--vscode-input-background)] p-1 px-2 text-xs text-[var(--vscode-input-foreground)] outline-none w-32"
                  value={chatSessionId}
                  onChange={(event) => setChatSessionId(event.target.value)}
                  placeholder="session id"
                />
                <select
                  className="rounded border border-[var(--vscode-dropdown-border)] bg-[var(--vscode-dropdown-background)] p-1 px-2 text-xs text-[var(--vscode-dropdown-foreground)] outline-none"
                  value={chatProfile}
                  onChange={(event) => setChatProfile(event.target.value as 'fast' | 'reasoning' | 'creative')}
                >
                  <option value="fast">Fast</option>
                  <option value="reasoning">Reasoning</option>
                  <option value="creative">Creative</option>
                </select>
                <span className="text-[11px] opacity-60 font-mono">
                  {chatMeta?.model_used ? chatMeta.model_used : 'model auto'}
                  {typeof chatMeta?.tokens === 'number' ? ` · ${chatMeta.tokens} tok` : ''}
                </span>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {(workspaceContext?.openFiles ?? []).map((file: string) => (
                  <button
                    key={file}
                    type="button"
                    className={`px-2 py-1 rounded-full border text-[10px] uppercase font-bold tracking-widest transition-colors ${
                      pinnedFiles.includes(file)
                        ? 'bg-[var(--vscode-charts-blue)] bg-opacity-20 border-[var(--vscode-charts-blue)] text-[var(--vscode-textLink-foreground)]'
                        : 'bg-[var(--vscode-editor-background)] border-[var(--vscode-panel-border)] opacity-60'
                    }`}
                    onClick={() => togglePinnedFile(file)}
                  >
                    {file.split(/[\/\\]/).pop()}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex-1 overflow-y-auto space-y-4 min-h-0 pr-1">
              {planAdequacyQuestions.length > 0 && (
                <div className="p-4 rounded-lg border border-[var(--vscode-editorWarning-foreground)] bg-yellow-500/10">
                  <h3 className="text-[var(--vscode-editorWarning-foreground)] font-bold text-xs uppercase tracking-widest mb-2 flex items-center gap-2">
                    Clarification Required
                  </h3>
                  <ul className="list-disc pl-5 text-[11px] opacity-90 space-y-1">
                    {planAdequacyQuestions.map((q, i) => (
                      <li key={i}>{q}</li>
                    ))}
                  </ul>
                </div>
              )}
              {chatMessages.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full opacity-40 gap-4">
                     <MessageSquare size={48} />
                     <p className="text-xs uppercase font-bold tracking-widest">Awaiting Prompt</p>
                </div>
              ) : (
                chatMessages.map((message: any) => {
                  const streaming = Boolean(message.is_streaming || message.isStreaming);
                  const body = typeof message.content === 'string' && message.content.length > 0 ? message.content : streaming ? '...' : '';
                  return (
                    <div key={String(message.id)} className="rounded-xl border border-[var(--vscode-panel-border)] p-4 bg-[var(--vscode-editor-background)] text-sm shadow-sm">
                      <div className="text-[10px] uppercase font-bold tracking-widest opacity-60 mb-3 flex items-center justify-between border-b border-[var(--vscode-panel-border)] pb-2">
                        <span className="flex items-center gap-2"><Terminal size={12}/> {message.role}</span>
                        {Array.isArray(message.context_files) && message.context_files.length > 0 ? (
                          <span>{message.context_files.length} context attached</span>
                        ) : null}
                      </div>
                      <div className="chat-markdown prose prose-invert prose-sm max-w-none text-[var(--vscode-editor-foreground)]">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ className, children, ...props }) {
                              const lang = /language-(\w+)/.exec(className ?? '')?.[1] ?? 'text';
                              const text = String(children);
                              if (text.includes('\n') || className?.startsWith('language-')) {
                                return <CodeBlock code={text.trimEnd()} lang={lang} />;
                              }
                              return <code className={`px-1 py-0.5 rounded bg-[var(--vscode-textCodeBlock-background)] text-[var(--vscode-textPreformat-foreground)] ${className}`} {...props}>{children}</code>;
                            },
                          }}
                        >
                          {body}
                        </ReactMarkdown>
                      </div>
                    </div>
                  );
                })
              )}
            </div>

            <div className="shrink-0 p-1">
              <form
                className="flex flex-col gap-2 rounded-xl border border-[var(--vscode-input-border)] bg-[var(--vscode-input-background)] p-1 shadow focus-within:border-[var(--vscode-focusBorder)] transition-colors"
                onSubmit={(event) => {
                  event.preventDefault();
                  const value = chatInput.trim();
                  if (!value) return;
                  vscode.postMessage({
                    type: 'submitTask',
                    value: {
                      prompt: value,
                      contextFiles: pinnedFiles,
                      sessionId: chatSessionId,
                      cognitiveProfile: chatProfile,
                    },
                  });
                  setChatInput('');
                }}
              >
                <textarea
                  className="w-full min-h-[64px] border-none bg-transparent p-2 text-sm resize-y outline-none text-[var(--vscode-input-foreground)] placeholder-[var(--vscode-input-placeholderForeground)]"
                  placeholder="Message Vox..."
                  value={chatInput}
                  onChange={(event) => setChatInput(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' && !event.shiftKey) {
                      event.preventDefault();
                      event.currentTarget.form?.requestSubmit();
                    }
                  }}
                />
                <button
                  type="submit"
                  className="px-4 py-1.5 rounded bg-[var(--vscode-button-background)] text-[var(--vscode-button-foreground)] text-[11px] font-bold uppercase tracking-widest shrink-0 self-end hover:bg-[var(--vscode-button-hoverBackground)]"
                >
                  Confirm
                </button>
              </form>
            </div>
          </div>
        );
      default:
        return null;
    }
  };

  const tabs = useMemo(
    () => [
      { id: 'chat' as const, icon: <MessageSquare size={18} /> },
      { id: 'dashboard' as const, icon: <LayoutDashboard size={18} /> },
      { id: 'diagnostics' as const, icon: <ActivityIcon size={18} /> },
    ],
    [],
  );

  return (
    <div className="flex h-screen w-screen overflow-hidden vox-root bg-[var(--vscode-editor-background)] text-[var(--vscode-editor-foreground)]">
      <aside className="vox-nav-rail w-12 shrink-0 border-r border-[var(--vscode-sideBarSectionHeader-border)] flex flex-col items-center py-4 gap-3 z-50 bg-[var(--vscode-sideBar-background)]">
        {tabs.map((tab) => (
          <NavIcon key={tab.id} icon={tab.icon} active={activeTab === tab.id} onClick={() => setActiveTab(tab.id)} />
        ))}
        <div className="mt-auto flex flex-col gap-3 mb-2">
          <NavIcon icon={<Settings2 size={16} />} onClick={() => vscode.postMessage({ type: 'pickModel' })} />
          <div className="w-6 h-6 rounded-md bg-[var(--vscode-textLink-foreground)] bg-opacity-10 border border-[var(--vscode-textLink-foreground)] flex items-center justify-center text-[var(--vscode-textLink-foreground)] text-[10px] font-bold">V</div>
        </div>
      </aside>

      <main className="flex-1 relative overflow-hidden flex flex-col min-w-0">
        <div
          role="status"
          aria-live="polite"
          className="vox-exec-hint text-[10px] px-3 py-1 font-mono border-b border-[var(--vscode-panel-border)] shrink-0 opacity-80"
        >
          {execHint}
          {capabilities?.db_configured === false ? ' · events: transient' : ''}
          {typeof capabilities?.toolCount === 'number' ? ` · MCP tools: ${capabilities.toolCount}` : ''}
          {capabilities?.schemaFingerprint ? ` · cap fp: ${capabilities.schemaFingerprint}` : ''}
          {chatMeta?.socrates?.risk_decision ? ` · Socrates: ${String(chatMeta.socrates.risk_decision)}` : ''}
          {capabilities?.lastMcpError ? ` · MCP error: ${String(capabilities.lastMcpError).slice(0, 120)}` : ''}
        </div>
        
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.15 }}
            className="flex-1 min-h-0 w-full"
          >
            <ErrorBoundary>{renderContent()}</ErrorBoundary>
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  );
}

const NavIcon = ({ icon, active, onClick }: any) => (
  <button
    onClick={onClick}
    className={`w-8 h-8 rounded-lg flex items-center justify-center transition-colors relative ${
      active 
        ? 'bg-[var(--vscode-textLink-activeForeground)] bg-opacity-20 text-[var(--vscode-textLink-foreground)] border border-[var(--vscode-textLink-foreground)]' 
        : 'text-[var(--vscode-icon-foreground)] opacity-60 hover:opacity-100 hover:bg-[var(--vscode-list-hoverBackground)]'
    }`}
  >
    {icon}
  </button>
);

const rootElement = document.getElementById('root');
if (rootElement) {
  createRoot(rootElement).render(<App />);
}
