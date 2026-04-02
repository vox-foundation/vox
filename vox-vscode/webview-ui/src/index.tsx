import React, { useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  LayoutDashboard,
  Network,
  Blocks,
  Activity as ActivityIcon,
  Code2,
  Settings2,
  BrainCircuit,
  RotateCcw,
  Globe2,
  Trophy,
  MessageSquare,
  Sparkles,
  ScanSearch,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import './index.css';
import { getVsCodeApi } from './utils/vscode';
import { parseHostToWebviewMessage } from '../../src/protocol/hostToWebviewMessages';
import type { ChatSessionMeta, ComposerState, WorkspaceInspectorState } from '../../src/types';
import { Dashboard } from './components/Dashboard';
import { AgentFlow } from './components/AgentFlow';
import { PipelineView } from './components/PipelineView';
import { AstView } from './components/AstView';
import { FinancialDashboard } from './components/FinancialDashboard';
import { IntentionMatrix } from './components/IntentionMatrix';
import { WorkflowScrubber } from './components/WorkflowScrubber';
import { MeshTopology } from './components/MeshTopology';
import { CompanionHUD } from './components/CompanionHUD';
import { LudusPanel } from './components/LudusPanel';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ComposerPanel } from './components/ComposerPanel';
import { ContextExplorer } from './components/ContextExplorer';
import { CodeBlock } from './components/CodeBlock';

const vscode = getVsCodeApi();

// Tab ids are persisted identifiers: keep `telemetry` stable even if the nav icon label changes.
// User-facing disclosure expectations: docs/src/architecture/telemetry-client-disclosure-ssot.md
type TabId =
  | 'dashboard'
  | 'flow'
  | 'chat'
  | 'composer'
  | 'context'
  | 'pipeline'
  | 'ast'
  | 'telemetry'
  | 'intentions'
  | 'scrubber'
  | 'mesh'
  | 'ludus';

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('dashboard');
  const [voxStatus, setVoxStatus] = useState<any>(null);
  const [gamify, setGamify] = useState<any>(null);
  const [_languageSurface, setLanguageSurface] = useState<any>(null);
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
  const [chatInput, setChatInput] = useState<string>('');
  const [chatSessionId, setChatSessionId] = useState<string>('vscode-sidebar');
  const [chatProfile, setChatProfile] = useState<'fast' | 'reasoning' | 'creative'>('reasoning');
  const [workspaceContext, setWorkspaceContext] = useState<any>(null);
  const [composerState, setComposerState] = useState<ComposerState | null>(null);
  const [inspectorState, setInspectorState] = useState<WorkspaceInspectorState | null>(null);
  const [pinnedFiles, setPinnedFiles] = useState<string[]>([]);
  const [planAdequacyQuestions, setPlanAdequacyQuestions] = useState<string[]>([]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const parsed = parseHostToWebviewMessage(event.data);
      if (!parsed) return;
      switch (parsed.type) {
        case 'voxStatus':
          setVoxStatus(parsed.value);
          break;
        case 'gamifyUpdate':
          setGamify(parsed.value);
          break;
        case 'languageSurface':
          setLanguageSurface(parsed.value);
          break;
        case 'astResult':
          setAst(parsed.value);
          break;
        case 'pipelineStatus':
          setPipeline(parsed.value);
          break;
        case 'activeEditorChanged':
          setActiveFile(String(parsed.value ?? ''));
          break;
        case 'a2aTasks':
          setTasks(Array.isArray(parsed.value) ? parsed.value : []);
          break;
        case 'budgetHistory':
          if (parsed.value) setBudgetHistory(parsed.value as any[]);
          break;
        case 'modelList':
          if (parsed.value) setModelList(parsed.value as any[]);
          break;
        case 'workflowStatus':
          setWorkflowStatus(parsed.value);
          break;
        case 'meshStatus':
          setMeshStatus(parsed.value);
          break;
        case 'intentionMatrix':
          setIntentionMatrix(parsed.value);
          break;
        case 'oplog':
          if (parsed.value) setOplog(parsed.value as any[]);
          break;
        case 'agentsUpdate':
          if (parsed.value) setAgents(parsed.value as any[]);
          break;
        case 'capabilitiesUpdate':
          setCapabilities(parsed.value);
          break;
        case 'ludusProgressSnapshot':
          if (parsed.value && typeof parsed.value === 'object' && !Array.isArray(parsed.value)) {
            setLudusSnapshot(parsed.value as Record<string, unknown>);
          }
          break;
        case 'chatHistory':
          setChatMessages(Array.isArray(parsed.value) ? parsed.value : []);
          break;
        case 'chatMeta':
          setChatMeta((parsed.value as ChatSessionMeta) ?? null);
          break;
        case 'workspaceContext':
          setWorkspaceContext(parsed.value);
          break;
        case 'composerState':
          setComposerState((parsed.value as ComposerState) ?? null);
          break;
        case 'inspectorState':
          setInspectorState((parsed.value as WorkspaceInspectorState) ?? null);
          break;
        case 'planUpdate':
          break;
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
  const agentCount =
    typeof orch?.agent_count === 'number'
      ? orch.agent_count
      : agents.filter((agent) => agent.status === 'working').length;
  const dashboardStats = {
    activeAgents: String(agentCount),
    queueDepth: tasks.length.toString(),
    latency: voxStatus?.avg_latency_ms ? `${voxStatus.avg_latency_ms}ms` : '--',
    budget:
      (voxStatus as { total_cost_usd?: number } | null)?.total_cost_usd != null
        ? `$${(voxStatus as { total_cost_usd: number }).total_cost_usd.toFixed(2)}`
        : '--',
  };

  const taskFallback = tasks.map((task: any, index: number) => ({
    id: String(task.id ?? task.task_id ?? index),
    description:
      typeof task.description === 'string' && task.description.length > 30
        ? `${task.description.substring(0, 27)}...`
        : task.description ?? 'task',
    agent_id: task.agent_id ?? '--',
    status: task.status === 'InProgress' ? 'Running' : task.status ?? 'Queued',
    duration_ms: undefined as number | undefined,
  }));
  const opRows = Array.isArray(oplog) && oplog.length > 0 ? oplog : taskFallback;

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
        return <Dashboard stats={dashboardStats} ops={opRows} pipeline={pipeline} />;
      case 'flow':
        return <AgentFlow tasks={tasks} capabilities={capabilities} />;
      case 'chat':
        return (
          <div className="flex flex-col h-full min-h-0 p-4 gap-3">
            <div className="glass rounded-2xl border border-white/10 p-3">
              <div className="flex flex-wrap items-center gap-2">
                <input
                  className="rounded-lg border border-white/10 bg-black/20 p-2 text-xs text-zinc-200 outline-none"
                  value={chatSessionId}
                  onChange={(event) => setChatSessionId(event.target.value)}
                  placeholder="session id"
                />
                <select
                  className="rounded-lg border border-white/10 bg-black/20 p-2 text-xs text-zinc-200 outline-none"
                  value={chatProfile}
                  onChange={(event) => setChatProfile(event.target.value as 'fast' | 'reasoning' | 'creative')}
                >
                  <option value="fast">Fast</option>
                  <option value="reasoning">Reasoning</option>
                  <option value="creative">Creative</option>
                </select>
                <span className="text-[11px] text-zinc-500">
                  {chatMeta?.model_used ? `model ${chatMeta.model_used}` : 'model auto'}
                  {typeof chatMeta?.tokens === 'number' ? ` · ${chatMeta.tokens} tok` : ''}
                </span>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {(workspaceContext?.openFiles ?? []).map((file: string) => (
                  <button
                    key={file}
                    type="button"
                    className={`px-2 py-1 rounded-full border text-[11px] ${
                      pinnedFiles.includes(file)
                        ? 'bg-blue-500/15 border-blue-500/30 text-blue-200'
                        : 'bg-black/20 border-white/10 text-zinc-500'
                    }`}
                    onClick={() => togglePinnedFile(file)}
                  >
                    {file}
                  </button>
                ))}
              </div>
            </div>

            <div className="flex-1 overflow-y-auto space-y-3 min-h-0 pr-1">
              {planAdequacyQuestions.length > 0 && (
                <div className="p-4 rounded-xl border border-amber-500/30 bg-amber-500/10">
                  <h3 className="text-amber-400 font-medium text-sm mb-2 flex items-center gap-2">
                    Wait, consider these clarifying questions before proceeding:
                  </h3>
                  <ul className="list-disc pl-5 text-amber-200/80 text-sm space-y-1">
                    {planAdequacyQuestions.map((q, i) => (
                      <li key={i}>{q}</li>
                    ))}
                  </ul>
                </div>
              )}
              {chatMessages.length === 0 ? (
                <p className="opacity-50 text-xs">No messages yet. Send a prompt below.</p>
              ) : (
                chatMessages.map((message: any) => {
                  const streaming = Boolean(message.is_streaming || message.isStreaming);
                  const body =
                    typeof message.content === 'string' && message.content.length > 0
                      ? message.content
                      : streaming
                        ? '...'
                        : '';
                  return (
                    <div key={String(message.id)} className="glass rounded-2xl border border-white/10 p-3">
                      <div className="text-[10px] uppercase tracking-wide opacity-50 mb-2 flex items-center justify-between">
                        <span>{message.role}</span>
                        {Array.isArray(message.context_files) && message.context_files.length > 0 ? (
                          <span>{message.context_files.length} context files</span>
                        ) : null}
                      </div>
                      <div className="chat-markdown">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ className, children, ...props }) {
                              const lang = /language-(\w+)/.exec(className ?? '')?.[1] ?? 'text';
                              const text = String(children);
                              // Block code: has a newline or came from a fenced block
                              if (text.includes('\n') || className?.startsWith('language-')) {
                                return <CodeBlock code={text.trimEnd()} lang={lang} />;
                              }
                              // Inline code
                              return <code className={className} {...props}>{children}</code>;
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

            <div className="glass rounded-2xl border border-white/10 p-3">
              <form
                className="flex flex-col gap-2 shrink-0"
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
                  className="w-full min-h-[88px] rounded-lg border border-white/10 bg-black/20 p-3 text-sm resize-y outline-none"
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
                  className="px-3 py-2 rounded-lg bg-blue-600 text-white text-xs font-semibold shrink-0 self-start"
                >
                  Send
                </button>
              </form>
            </div>
          </div>
        );
      case 'composer':
        return <ComposerPanel composerState={composerState} />;
      case 'context':
        return <ContextExplorer inspector={inspectorState} />;
      case 'pipeline':
        return <PipelineView status={pipeline} />;
      case 'ast':
        return <AstView ast={ast} activeFile={activeFile} />;
      case 'telemetry':
        return <FinancialDashboard stats={voxStatus} budgetHistory={budgetHistory} modelList={modelList} />;
      case 'intentions':
        return <IntentionMatrix intents={intentionMatrix} socratesStatus={voxStatus} />;
      case 'scrubber':
        return <WorkflowScrubber snapshots={workflowStatus} />;
      case 'mesh':
        return <MeshTopology topology={meshStatus} />;
      case 'ludus':
        return <LudusPanel snapshot={ludusSnapshot} />;
      default:
        return <Dashboard stats={dashboardStats} ops={opRows} pipeline={pipeline} />;
    }
  };

  const tabs = useMemo(
    () => [
      { id: 'dashboard' as const, icon: <LayoutDashboard size={20} /> },
      { id: 'flow' as const, icon: <Network size={20} /> },
      { id: 'chat' as const, icon: <MessageSquare size={20} /> },
      { id: 'composer' as const, icon: <Sparkles size={20} /> },
      { id: 'context' as const, icon: <ScanSearch size={20} /> },
      { id: 'scrubber' as const, icon: <RotateCcw size={20} /> },
      { id: 'intentions' as const, icon: <BrainCircuit size={20} /> },
      { id: 'mesh' as const, icon: <Globe2 size={20} /> },
      { id: 'pipeline' as const, icon: <Blocks size={20} /> },
      { id: 'ast' as const, icon: <Code2 size={20} /> },
      { id: 'telemetry' as const, icon: <ActivityIcon size={20} /> },
      { id: 'ludus' as const, icon: <Trophy size={20} /> },
    ],
    [],
  );

  return (
    <div className="flex h-screen w-screen overflow-hidden vox-root">
      <aside className="vox-nav-rail w-16 border-r flex flex-col items-center py-6 gap-4 z-50">
        {tabs.map((tab) => (
          <NavIcon key={tab.id} icon={tab.icon} active={activeTab === tab.id} onClick={() => setActiveTab(tab.id)} />
        ))}
        <div className="mt-auto flex flex-col gap-4 mb-2">
          <NavIcon icon={<Settings2 size={18} />} onClick={() => vscode.postMessage({ type: 'pickModel' })} />
          <div className="w-8 h-8 rounded-full bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-500 text-[10px] font-bold">V</div>
        </div>
      </aside>

      <main className="flex-1 relative overflow-hidden flex flex-col">
        <div
          role="status"
          aria-live="polite"
          className="vox-exec-hint text-[10px] px-3 py-1 font-mono border-b shrink-0 leading-relaxed"
        >
          {execHint}
          {capabilities?.db_configured === false ? ' · events: transient' : ''}
          {typeof capabilities?.toolCount === 'number' ? ` · MCP tools: ${capabilities.toolCount}` : ''}
          {capabilities?.schemaFingerprint ? ` · cap fp: ${capabilities.schemaFingerprint}` : ''}
          {chatMeta?.socrates?.risk_decision ? ` · Socrates: ${String(chatMeta.socrates.risk_decision)}` : ''}
          {capabilities?.lastMcpError ? ` · MCP error: ${String(capabilities.lastMcpError).slice(0, 120)}` : ''}
        </div>
        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, x: 10 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -10 }}
            transition={{ duration: 0.2, ease: 'easeOut' }}
            className="h-full w-full min-h-0"
          >
            <ErrorBoundary>{renderContent()}</ErrorBoundary>
          </motion.div>
        </AnimatePresence>
        <CompanionHUD gamify={gamify} />
      </main>
    </div>
  );
}

const NavIcon = ({ icon, active, onClick }: any) => (
  <button
    onClick={onClick}
    className={`w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-300 group relative ${
      active ? 'bg-blue-600 text-white shadow-[0_0_20px_rgba(59,130,246,0.3)]' : 'text-zinc-500 hover:text-zinc-300 hover:bg-white/5'
    }`}
  >
    {icon}
    {active && <motion.div layoutId="nav-glow" className="absolute inset-0 rounded-xl bg-blue-400/20 blur-xl -z-10" />}
  </button>
);

const rootElement = document.getElementById('root');
if (rootElement) {
  createRoot(rootElement).render(<App />);
}
