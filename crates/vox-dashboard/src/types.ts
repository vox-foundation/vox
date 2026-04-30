// ── Workflow / mesh runtime ───────────────────────────────────────────────────

export interface VoxWorkflowStatus {
  name: string;
  overall_status: string;
  elapsed_ms: number;
  steps: WorkflowStep[];
  actors: WorkflowActor[];
}
export interface WorkflowStep {
  id: number; name: string; status: 'success' | 'failed' | 'pending';
  time_iso: string; output_data?: string; error_msg?: string;
  retry_count?: number; max_retries?: number; backoff_label?: string; timeout_label?: string;
}
export interface WorkflowActor {
  name: string; state_bytes: string; inbox_depth: number;
}
export interface IntentionEntry {
  id: string; goal: string; confidence: number; active: boolean;
  agent_reliability: number; speculative_branch: string; prompt_trace?: string;
}
export interface VoxMeshTopology {
  nodes: MeshNodeData[]; edges: MeshEdgeData[];
  active_migrations: { actor: string; from_node: string; to_node: string }[];
}
export interface MeshNodeData {
  id: string; node_type: 'primary' | 'edge'; region: string;
  latency_ms: number; cpu_pct: number; resident_actors: string[];
  position?: { x: number; y: number };
}
export interface MeshEdgeData {
  id: string; from: string; to: string; status: 'ws' | 'poll';
}
export interface BudgetBucket { time: string; cost: number; }

// ── Chat / session ────────────────────────────────────────────────────────────

export interface ChatSessionMeta {
  model_used?: string;
  tokens?: number;
  socrates?: {
    risk_decision?: string;
  };
}

// ── Composer ──────────────────────────────────────────────────────────────────

export interface ComposerDraft {
  path: string;
  /** Full original file content before the proposed edit. */
  original: string;
  /** Proposed replacement content. */
  proposed: string;
  model_used?: string;
  tokens?: number;
  explanation?: string;
}

export interface ComposerState {
  availableFiles?: string[];
  drafts?: ComposerDraft[];
  isGenerating?: boolean;
  lastError?: string;
  snapshotRequested?: boolean;
}

// ── Workspace inspector ───────────────────────────────────────────────────────

export interface ActiveEditorDiagnostic {
  severity: string;
  line: number;
  message: string;
}

export interface ActiveEditorState {
  filePath?: string;
  languageId?: string;
  line?: number;
  selectedText?: string;
  diagnostics?: ActiveEditorDiagnostic[];
}

export interface LastPlanState {
  plan_adequacy_score?: number;
  plan_too_thin?: boolean;
  tasks?: unknown[];
  adequacy_reason_codes?: string[];
}

export interface WorkspaceInspectorState {
  openFiles?: string[];
  activeEditor?: ActiveEditorState;
  lastChatMeta?: {
    socrates?: {
      risk_decision?: string;
      confidence_estimate?: number;
      contradiction_ratio?: number;
    };
    retrieval?: {
      retrieval_tier?: string;
      evidence_count?: number;
    };
  };
  repoIndexStatus?: unknown;
  lastPlan?: LastPlanState;
  capabilityManifest?: unknown;
  repoQueryResult?: unknown;
  /** Keys present in the agent context store (for the context-store panel). */
  contextKeys?: string[];
  /** Last fetched context-store value (raw, displayed via pretty-print). */
  contextValue?: unknown;
  /** Snapshot of the browser tool state (url, title, etc.). */
  browserState?: unknown;
}

// ── Attention budgeting ───────────────────────────────────────────────────────

export interface AttentionStatusPayload {
  enabled?: boolean;
  max_ms?: number;
  spent_ms?: number;
  exhausted?: boolean;
  alert_threshold?: number;
  focus_depth?: string;
  interrupt_freq_per_hour?: number;
  auto_approve_ratio?: number;
  inbox_suppressed_count?: number;
}

/** Reserved for future alert payload; currently unused by the UI. */
export interface AttentionAlert {
  kind?: string;
  message?: string;
}

// ── Transport event types (TASK-0.6) ─────────────────────────────────────────

/** Discriminated union emitted as `connection_status` by VoxTransport. */
export type ConnectionStatusPayload =
  | { status: 'connected' }
  | { status: 'connecting'; attempt?: number }
  | { status: 'disconnected'; code?: number; attempt?: number }
  | { status: 'error'; error?: string }
  | { status: 'failed_permanently' };

/** Literal union emitted as `authStatus` by VoxTransport. */
export type AuthStatusEvent = 'authorized' | 'unauthorized' | 'no_token';

/** Known typed events emitted by VoxTransport. */
export interface VoxTransportEventMap {
  connection_status: ConnectionStatusPayload;
  authStatus: AuthStatusEvent;
}
