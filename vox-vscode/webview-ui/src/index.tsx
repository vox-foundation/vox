import React, { useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { MessageSquare, Crown, Network, Hammer, Settings2, Sparkles, Terminal, Mic } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';


import { getVsCodeApi } from './utils/vscode';
import { parseHostToWebviewMessage } from '../../src/protocol/hostToWebviewMessages';
import { ChatSessionMeta, ComposerState, WorkspaceInspectorState, AttentionStatusPayload } from '../../src/types';

import { UnifiedDashboard } from './components/UnifiedDashboard';
import { EngineeringDiagnostics } from './components/EngineeringDiagnostics';
import { ComposerPanel } from './components/ComposerPanel';
import { MeshTopology } from './components/MeshTopology';

import { ErrorBoundary } from './components/ErrorBoundary';
import { CodeBlock } from './components/CodeBlock';

const vscode = getVsCodeApi();

type TabId = 'speak' | 'command' | 'network' | 'forge';

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('speak');
  
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
  const [agents, setAgents] = useState<any[]>([]);
  const [capabilities, setCapabilities] = useState<any>(null);
  const [ludusSnapshot, setLudusSnapshot] = useState<Record<string, unknown> | null>(null);
  const [chatMessages, setChatMessages] = useState<any[]>([]);
  const [chatMeta, setChatMeta] = useState<ChatSessionMeta | null>(null);
  const [workspaceContext, setWorkspaceContext] = useState<any>(null);
  const [composerState, setComposerState] = useState<ComposerState | null>(null);
  const [inspectorState, setInspectorState] = useState<WorkspaceInspectorState | null>(null);
  const [planAdequacyQuestions, setPlanAdequacyQuestions] = useState<string[]>([]);
  const [attentionStatus, setAttentionStatus] = useState<AttentionStatusPayload | null>(null);
  const [attentionAlert, setAttentionAlert] = useState<any | null>(null);

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
        case 'attentionStatus': setAttentionStatus(parsed.value as AttentionStatusPayload); break;
        case 'attentionAlert': setAttentionAlert(parsed.value); break;
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
      case 'command':
        return (
            <UnifiedDashboard 
                ops={oplog} 
                stats={dashboardStats} 
                pipeline={pipeline} 
                budgetHistory={budgetHistory} 
                ludusSnapshot={ludusSnapshot} 
                meshTopology={undefined} 
                attentionStatus={attentionStatus}
                attentionAlert={attentionAlert}
            />
        );
      case 'network':
        return (
            <div className="w-full h-full p-4 relative text-foreground">
                <MeshTopology meshTopology={meshStatus} />
            </div>
        );
      case 'forge':
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
      case 'speak':
        return (
          <div className="flex flex-col h-full min-h-0 p-4 gap-4 text-foreground relative z-10 w-full">
            <div className="flex items-center justify-between pb-2 border-b border-border border-opacity-50 shrink-0">
                <div className="flex items-center gap-3">
                  <h2 className="text-2xl font-rajdhani text-brass tracking-wider">LOQUELA</h2>
                  <div className="px-2 py-0.5 rounded bg-machine border border-border text-[9px] font-mono text-steel uppercase tracking-widest hidden sm:block">
                    Vocal Synthesis Active
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button 
                     title="Oratio Microphone"
                     className="w-8 h-8 rounded-full border border-copper bg-machine flex items-center justify-center text-primary shadow-[0_0_8px_var(--vox-amber-glow)] hover:bg-primary hover:bg-opacity-20 transition-all cursor-pointer"
                  >
                      <Mic size={14} />
                  </button>
                  <button 
                     onClick={() => setComposerVisible(!composerVisible)}
                     className={`px-3 py-1.5 rounded text-[10px] font-bold uppercase tracking-widest border flex items-center gap-1.5 transition-all ${composerVisible ? 'bg-primary text-black border-primary shadow-[0_0_10px_var(--vox-amber-glow)]' : 'bg-machine text-brass border-border hover:border-copper'}`}
                  >
                      <Sparkles size={12} className={composerVisible ? 'text-black' : 'text-primary'} /> {composerVisible ? 'COMPOSER ACTIVE' : 'COMPOSER'}
                  </button>
                </div>
            </div>

            {composerVisible && composerState && (
                <div className="shrink-0 h-[300px] border border-cyan rounded-xl overflow-hidden relative shadow-[0_0_15px_var(--vox-cyan-glow)] z-20 bg-surface">
                   <ComposerPanel composerState={composerState} />
                </div>
            )}

            <div className="rounded-xl border border-border p-3 bg-machine shadow-[inset_0_4px_10px_rgba(0,0,0,0.5)] shrink-0">
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className="rounded border border-border bg-void p-1 px-2 text-xs text-foreground outline-none w-32 focus:border-cyan focus:shadow-[0_0_5px_var(--vox-cyan-glow)] transition-all font-mono"
                  value={chatSessionId}
                  onChange={(event) => setChatSessionId(event.target.value)}
                  placeholder="session id"
                />
                <select
                  className="rounded border border-border bg-void p-1 px-2 text-xs text-brass outline-none focus:border-cyan uppercase tracking-wider font-bold"
                  value={chatProfile}
                  onChange={(event) => setChatProfile(event.target.value as 'fast' | 'reasoning' | 'creative')}
                >
                  <option value="fast">FAST</option>
                  <option value="reasoning">REASONING</option>
                  <option value="creative">CREATIVE</option>
                </select>
                <span className="text-[10px] text-cyan font-mono px-2 py-0.5 rounded border border-cyan border-opacity-30 bg-cyan border-opacity-10 ml-auto flex items-center gap-2">
                  <span className="w-1.5 h-1.5 rounded-full bg-cyan animate-pulse"></span>
                  {chatMeta?.model_used ? chatMeta.model_used : 'MENS: AUTO'}
                  {typeof chatMeta?.tokens === 'number' ? ` · ${chatMeta.tokens} tok` : ''}
                </span>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {(workspaceContext?.openFiles ?? []).map((file: string) => (
                  <button
                    key={file}
                    type="button"
                    className={`px-2 py-1 rounded border text-[10px] font-mono tracking-wide transition-colors ${
                      pinnedFiles.includes(file)
                        ? 'bg-cyan bg-opacity-10 border-cyan text-cyan hover:bg-opacity-20 shadow-[0_0_5px_var(--vox-cyan-glow)]'
                        : 'bg-void border-border text-steel hover:text-foreground hover:bg-surface'
                    }`}
                    onClick={() => togglePinnedFile(file)}
                  >
                    {file.split(/[/\\]/).pop()}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex-1 overflow-y-auto space-y-4 min-h-0 pr-2 pb-2 custom-scrollbar">
              {planAdequacyQuestions.length > 0 && (
                <div className="p-4 rounded-lg border border-copper bg-copper bg-opacity-10">
                  <h3 className="text-primary font-rajdhani text-sm uppercase tracking-widest mb-2 flex items-center gap-2">
                    <span className="w-2 h-2 bg-primary animate-pulse" /> CLARIFICATION REQUIRED
                  </h3>
                  <ul className="list-disc pl-5 text-[11px] text-zinc-300 space-y-1 font-mono">
                    {planAdequacyQuestions.map((q, i) => (
                      <li key={i}>{q}</li>
                    ))}
                  </ul>
                </div>
              )}
              {chatMessages.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full opacity-30 gap-4 group">
                     <MessageSquare size={48} className="text-steel group-hover:text-primary transition-colors" />
                     <p className="text-xs uppercase font-rajdhani tracking-widest text-steel group-hover:text-brass transition-colors">Awaiting Directives</p>
                </div>
              ) : (
                chatMessages.map((message: any) => {
                  const streaming = Boolean(message.is_streaming || message.isStreaming);
                  const body = typeof message.content === 'string' && message.content.length > 0 ? message.content : streaming ? '...' : '';
                  return (
                    <div key={String(message.id)} className="rounded-xl border border-border p-4 bg-surface text-sm shadow-md">
                      <div className="text-[10px] uppercase font-bold tracking-widest text-brass mb-3 flex items-center justify-between border-b border-border border-opacity-50 pb-2">
                        <span className="flex items-center gap-2"><Terminal size={12} className="text-steel"/> {message.role}</span>
                        {Array.isArray(message.context_files) && message.context_files.length > 0 ? (
                          <span className="text-steel font-mono">{message.context_files.length} ctx</span>
                        ) : null}
                      </div>
                      <div className="chat-markdown prose prose-invert prose-sm max-w-none text-zinc-300">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ className, children, ...props }) {
                              const lang = /language-(\w+)/.exec(className ?? '')?.[1] ?? 'text';
                              const text = String(children);
                              if (text.includes('\n') || className?.startsWith('language-')) {
                                return <div className="border border-border/50 rounded-md overflow-hidden my-2"><CodeBlock code={text.trimEnd()} lang={lang} /></div>;
                              }
                              return <code className={`px-1.5 py-0.5 rounded bg-machine border border-border/50 text-cyan font-mono text-[11px] ${className}`} {...props}>{children}</code>;
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

            <div className="shrink-0 pt-2 shrink-0">
              <form
                className="flex flex-col gap-2 rounded-xl border border-border bg-machine p-2 shadow-[inset_0_2px_4px_rgba(0,0,0,0.5)] focus-within:border-primary focus-within:shadow-[0_0_8px_var(--vox-amber-glow)] transition-all"
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
                  className="w-full min-h-[64px] border-none bg-transparent p-2 text-sm resize-y outline-none text-foreground placeholder-steel opacity-80 focus:opacity-100"
                  placeholder="Initiate vox interaction sequence..."
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
                  className="px-6 py-2 rounded bg-primary text-black font-rajdhani text-sm font-bold uppercase tracking-widest shrink-0 self-end hover:bg-amber-400 border border-transparent hover:border-black shadow-md shadow-[0_0_5px_var(--vox-amber-glow)] transition-all"
                >
                  EXECUTE
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
      { id: 'speak' as const, label: 'SPEAK', subtitle: 'Loquela', icon: <MessageSquare size={18} /> },
      { id: 'command' as const, label: 'COMMAND', subtitle: 'Imperium', icon: <Crown size={18} /> },
      { id: 'network' as const, label: 'NETWORK', subtitle: 'Rete', icon: <Network size={18} /> },
      { id: 'forge' as const, label: 'FORGE', subtitle: 'Fabrica', icon: <Hammer size={18} /> },
    ],
    [],
  );

  return (
    <div className="flex h-screen w-screen overflow-hidden vox-root bg-background text-foreground">
      <aside className="vox-nav-rail w-[72px] shrink-0 border-r border-border border-opacity-30 flex flex-col items-center py-4 gap-4 z-50 bg-secondary">
        <div className="flex flex-col gap-6 w-full px-2">
          {tabs.map((tab) => (
            <NavIcon key={tab.id} icon={tab.icon} label={tab.label} subtitle={tab.subtitle} active={activeTab === tab.id} onClick={() => setActiveTab(tab.id)} />
          ))}
        </div>
        <div className="mt-auto flex flex-col items-center gap-6 mb-4">
          <button 
            onClick={() => vscode.postMessage({ type: 'pickModel' })}
            className="text-steel opacity-60 hover:opacity-100 transition-opacity flex flex-col items-center gap-1"
            title="Settings / Praecepta"
          >
            <Settings2 size={20} />
          </button>
          
          <div className="flex flex-col items-center gap-1 cursor-pointer group" title={ludusSnapshot?.kpi ? `Level ${ludusSnapshot.kpi.level_number || 1} — ${ludusSnapshot.kpi.total_xp || 0} XP` : "Genius Tracker"}>
            <div className="relative w-10 h-10 rounded-full flex items-center justify-center bg-machine border border-copper shadow-[0_0_15px_var(--vox-amber-glow)] group-hover:scale-110 transition-transform">
              <div className="absolute inset-0 rounded-full border-2 border-primary border-t-transparent animate-spin-slow opacity-70" style={{ animationDuration: '3s' }} />
              {ludusSnapshot?.kpi ? (
                 <span className="text-primary font-rajdhani font-bold text-lg tracking-wider drop-shadow-[0_0_5px_var(--vox-amber-glow)]">{ludusSnapshot.kpi.level_number || 'I'}</span>
              ) : (
                 <span className="text-primary font-rajdhani font-bold text-sm tracking-wider">V</span>
              )}
            </div>
          </div>
        </div>
      </aside>

      <main className="flex-1 relative overflow-hidden flex flex-col min-w-0 bg-background bg-opacity-50">
        <div
          role="status"
          aria-live="polite"
          className="vox-exec-hint text-[10px] px-3 py-1 font-mono border-b border-border border-opacity-30 shrink-0 text-steel opacity-80 bg-background"
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
            initial={{ opacity: 0, scale: 0.98 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.98 }}
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

const NavIcon = ({ icon, label, subtitle, active, onClick }: any) => (
  <button
    onClick={onClick}
    className={`w-full flex flex-col items-center justify-center p-2 rounded transition-all relative group ${
      active 
        ? 'text-primary bg-primary bg-opacity-10 border border-copper shadow-[inset_0_0_8px_var(--vox-amber-glow)]' 
        : 'text-steel border border-transparent hover:text-foreground hover:bg-white hover:bg-opacity-5'
    }`}
  >
    {icon}
    <span className={`text-[9px] font-bold mt-1 tracking-widest ${active ? 'text-primary' : 'text-steel'}`}>
      {label}
    </span>
    <span className={`text-[8px] italic font-rajdhani leading-tight ${active ? 'text-brass' : 'text-steel opacity-60'}`}>
      {subtitle}
    </span>
  </button>
);

const rootElement = document.getElementById('root');
if (rootElement) {
  createRoot(rootElement).render(<App />);
}
