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
