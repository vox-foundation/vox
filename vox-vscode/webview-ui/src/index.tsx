import React, { useState, useEffect, useMemo } from "react";
import { createRoot } from "react-dom/client";
import { 
  LayoutDashboard, 
  Network, 
  Blocks, 
  Activity as ActivityIcon, 
  Code2, 
  Settings2,
  Cpu,
  Zap
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

import "./index.css";
import { getVsCodeApi } from "./utils/vscode";
import { parseHostToWebviewMessage } from "../../src/protocol/hostToWebviewMessages";
import { Dashboard } from "./components/Dashboard";
import { AgentFlow } from "./components/AgentFlow";
import { PipelineView } from "./components/PipelineView";
import { AstView } from "./components/AstView";
import { FinancialDashboard } from "./components/FinancialDashboard";
import { IntentionMatrix } from "./components/IntentionMatrix";
import { WorkflowScrubber } from "./components/WorkflowScrubber";
import { MeshTopology } from "./components/MeshTopology";
import { CompanionHUD } from "./components/CompanionHUD";
import { LudusPanel } from "./components/LudusPanel";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { BrainCircuit, RotateCcw, Globe2, AlertCircle } from "lucide-react";

const vscode = getVsCodeApi();

function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'flow' | 'pipeline' | 'ast' | 'telemetry' | 'intentions' | 'scrubber' | 'mesh'>('dashboard');
  const [voxStatus, setVoxStatus] = useState<any>(null);
  const [gamify, setGamify] = useState<any>(null);
  const [languageSurface, setLanguageSurface] = useState<any>(null);
  const [ast, setAst] = useState<any>(null);
  const [pipeline, setPipeline] = useState<any>(null);
  const [activeFile, setActiveFile] = useState<string>("");
  const [tasks, setTasks] = useState<any[]>([]);

  // SDUI States
  const [workflowStatus, setWorkflowStatus] = useState<any>(null);
  const [meshStatus, setMeshStatus] = useState<any>(null);
  const [intentionMatrix, setIntentionMatrix] = useState<any>(null);
  const [oplog, setOplog] = useState<any[]>([]);
  const [budgetHistory, setBudgetHistory] = useState<any[]>([]);
  const [modelList, setModelList] = useState<any[]>([]);
  const [agents, setAgents] = useState<any[]>([]);
  const [capabilities, setCapabilities] = useState<any>(null);

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
          setActiveFile(parsed.value);
          break;
        case 'a2aTasks':
          setTasks(Array.isArray(parsed.value) ? parsed.value : []);
          break;
        case 'budgetHistory':
          if (parsed.value) setBudgetHistory(parsed.value);
          break;
        case 'modelList':
          if (parsed.value) setModelList(parsed.value);
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
          if (parsed.value) setOplog(parsed.value);
          break;
        case 'agentsUpdate':
          if (parsed.value) setAgents(parsed.value);
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
        case 'planUpdate':
          break;
      }
    };

    window.addEventListener('message', handler);
    vscode.postMessage({ type: 'getInitialData' });

    return () => window.removeEventListener('message', handler);
  }, []);

  const renderContent = () => {
    const orch = gamify as Record<string, unknown> | null;
    const agentCount =
      typeof orch?.agent_count === 'number'
        ? orch.agent_count
        : agents.filter((a) => a.status === 'working').length;
    const dashboardStats = {
      activeAgents: String(agentCount),
      queueDepth: tasks.length.toString(),
      latency: voxStatus?.avg_latency_ms ? `${voxStatus.avg_latency_ms}ms` : '--',
      budget:
        (voxStatus as { total_cost_usd?: number } | null)?.total_cost_usd != null
          ? `$${(voxStatus as { total_cost_usd: number }).total_cost_usd.toFixed(2)}`
          : '--',
    };

    const taskFallback = tasks.map((t: any, i: number) => ({
      id: String(t.id ?? t.task_id ?? i),
      description:
        typeof t.description === 'string' && t.description.length > 30
          ? t.description.substring(0, 27) + '...'
          : t.description ?? 'task',
      agent_id: t.agent_id ?? '--',
      status: t.status === 'InProgress' ? 'Running' : t.status ?? 'Queued',
      duration_ms: undefined as number | undefined,
    }));
    const opRows = Array.isArray(oplog) && oplog.length > 0 ? oplog : taskFallback;

    switch (activeTab) {
      case 'dashboard':
        return <Dashboard stats={dashboardStats} ops={opRows} pipeline={pipeline} />;
      case 'flow':
        return <AgentFlow tasks={tasks} capabilities={capabilities} />;
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

  return (
    <div
      className="flex h-screen w-screen overflow-hidden vox-root"
      style={{
        background: 'var(--vscode-sideBar-background, #09090b)',
        color: 'var(--vscode-sideBar-foreground, #e4e4e7)',
      }}
    >
      {/* Mini Sidebar Nav */}
      <aside
        className="w-16 border-r flex flex-col items-center py-6 gap-6 z-50"
        style={{
          borderColor: 'var(--vscode-panel-border, rgba(255,255,255,0.06))',
          background: 'var(--vscode-sideBar-background, rgba(255,255,255,0.02))',
        }}
      >
        <NavIcon icon={<LayoutDashboard size={20} />} active={activeTab === 'dashboard'} onClick={() => setActiveTab('dashboard')} />
        <NavIcon icon={<Network size={20} />} active={activeTab === 'flow'} onClick={() => setActiveTab('flow')} />
        <NavIcon icon={<RotateCcw size={20} />} active={activeTab === 'scrubber'} onClick={() => setActiveTab('scrubber')} />
        <NavIcon icon={<BrainCircuit size={20} />} active={activeTab === 'intentions'} onClick={() => setActiveTab('intentions')} />
        <NavIcon icon={<Globe2 size={20} />} active={activeTab === 'mesh'} onClick={() => setActiveTab('mesh')} />
        <NavIcon icon={<Blocks size={20} />} active={activeTab === 'pipeline'} onClick={() => setActiveTab('pipeline')} />
        <NavIcon icon={<Code2 size={20} />} active={activeTab === 'ast'} onClick={() => setActiveTab('ast')} />
        <NavIcon icon={<ActivityIcon size={20} />} active={activeTab === 'telemetry'} onClick={() => setActiveTab('telemetry')} />
        <NavIcon icon={<Trophy size={20} />} active={activeTab === 'ludus'} onClick={() => setActiveTab('ludus')} />
        
        <div className="mt-auto flex flex-col gap-4 mb-2">
           <NavIcon icon={<Settings2 size={18} />} onClick={() => vscode.postMessage({ type: 'pickModel' })} />
           <div className="w-8 h-8 rounded-full bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-500 text-[10px] font-bold">V</div>
        </div>
      </aside>

      {/* Main content area */}
      <main className="flex-1 relative overflow-hidden flex flex-col">
        <div
          role="status"
          aria-live="polite"
          className="text-[10px] px-3 py-1 font-mono border-b shrink-0 leading-relaxed"
          style={{
            borderColor: 'var(--vscode-panel-border)',
            background: 'var(--vscode-editor-inactiveSelectionBackground, rgba(100,100,100,0.15))',
            color: 'var(--vscode-descriptionForeground, inherit)',
          }}
        >
          {execHint}
          {capabilities?.db_configured === false ? ' · events: transient' : ''}
          {typeof capabilities?.toolCount === 'number' ? ` · MCP tools: ${capabilities.toolCount}` : ''}
          {capabilities?.schemaFingerprint ? ` · cap fp: ${capabilities.schemaFingerprint}` : ''}
          {capabilities?.lastMcpError
            ? ` · MCP error: ${String(capabilities.lastMcpError).slice(0, 120)}`
            : ''}
        </div>
        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, x: 10 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -10 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            className="h-full w-full min-h-0"
          >
            <ErrorBoundary>
              {renderContent()}
            </ErrorBoundary>
          </motion.div>
        </AnimatePresence>
        
        {/* Companion HUD overlay */}
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

function TelemetryView({ stats }: any) {
    return (
        <div className="p-10 bg-[#09090b] h-full overflow-y-auto">
            <h2 className="text-3xl font-black text-white mb-8 tracking-tighter uppercase">Fleet <span className="text-blue-500">Telemetry</span></h2>
            <div className="grid grid-cols-2 gap-8">
                <div className="glass p-8 rounded-[2rem] border border-white/5">
                    <h3 className="text-sm font-bold text-zinc-500 uppercase tracking-widest mb-6">Token Usage (Life)</h3>
                    <div className="flex flex-col items-center justify-center h-40">
                        <div className="text-5xl font-black text-white tracking-tighter mb-2">
                            {stats?.total_tokens_used?.toLocaleString() || "0"}
                        </div>
                        <div className="text-[10px] font-bold text-blue-500 uppercase tracking-widest">Total tokens (session)</div>
                    </div>
                </div>
                <div className="glass p-8 rounded-[2rem] border border-white/5">
                    <h3 className="text-sm font-bold text-zinc-500 uppercase tracking-widest mb-6">Estimated Cost</h3>
                    <div className="flex flex-col items-center justify-center h-40">
                        <div className="text-5xl font-black text-white tracking-tighter mb-2">
                            ${stats?.total_cost_usd?.toFixed(2) || "0.00"}
                        </div>
                        <div className="text-[10px] font-bold text-emerald-500 uppercase tracking-widest">USD Invested in Intelligence</div>
                    </div>
                </div>
            </div>
        </div>
    );
}

const LatencyBar = ({ label, value, pct }: any) => (
    <div>
        <div className="flex justify-between text-[11px] font-bold text-zinc-400 mb-2 uppercase tracking-widest">
            <span>{label}</span>
            <span className="text-zinc-500 font-mono">{value}</span>
        </div>
        <div className="h-1.5 bg-white/5 rounded-full overflow-hidden border border-white/5">
            <motion.div initial={{ width: 0 }} animate={{ width: `${pct}%` }} className="h-full bg-blue-500" />
        </div>
    </div>
);

const rootElement = document.getElementById("root");
if (rootElement) {
  createRoot(rootElement).render(<App />);
}
