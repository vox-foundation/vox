/**
 * Canonical wire DTOs for Mode 1: Vox Concilium (Causal Swimlanes, Shared #seq Board, Live Agent Lens)
 * and Mode 2: Vox Populi (Hybrid Compute Placement Map & FileAffinityMap Lock Matrix).
 * Mirrors Rust `vox_orchestrator_types::concilium` (`CONC-01..08`).
 */

export type ComputeTier =
  | 'cloud_bandit'
  | 'local_mens_gpu'
  | 'populi_quic_peer'
  | 'inline_cpu';

export type SwarmWorkspaceMode = 'inherit' | 'share' | 'branch';

export interface CausalSwarmStamp {
  swarm_seq: number;
  channel: string;
  traceparent: string;
  span_id: string;
  compute_tier: ComputeTier;
  file_lock_prefix?: string | null;
  workspace_mode: SwarmWorkspaceMode;
  parked_since_seq?: number | null;
}

export type ConciliumSpanKind =
  | 'thinking'
  | 'tool_call'
  | 'writing'
  | 'parked'
  | 'lock_wait'
  | 'trip';

export interface ConciliumSpanBar {
  span_id: string;
  traceparent: string;
  kind: ConciliumSpanKind;
  label: string;
  detail?: string | null;
  start_offset_ms: number;
  duration_ms: number;
  file_lock_prefix?: string | null;
  woken_by_seq?: number | null;
}

export interface ConciliumWakeupArc {
  from_seq: number;
  target_agent: string;
  target_span_id: string;
  mention: string;
}

export interface ConciliumAgentLane {
  agent_id: string;
  roster_name: string;
  role: string;
  status: 'working' | 'waiting' | 'parked' | 'tripped' | 'idle';
  compute_tier: ComputeTier;
  node_id: string;
  model_id: string;
  workspace_mode: SwarmWorkspaceMode;
  file_lock_prefix?: string | null;
  parked_since_seq?: number | null;
  tok_per_sec: number;
  used_tokens: number;
  max_tokens: number;
  kv_cache_hit_pct?: number | null;
  current_action: string;
  spans: ConciliumSpanBar[];
}

export interface SequenceBoardEntry {
  seq: number;
  channel: string;
  sender: string;
  body: string;
  mentions: string[];
  timestamp_ms: number;
  traceparent: string;
  span_id: string;
  compute_tier: ComputeTier;
  file_lock_prefix?: string | null;
  workspace_mode: SwarmWorkspaceMode;
  parked_since_seq?: number | null;
  receipt_badge?: string | null;
  is_watchdog_alert?: boolean;
}

export interface ConciliumSnapshotDto {
  head_seq: number;
  window_duration_ms: number;
  active_workers: number;
  parked_workers: number;
  active_locks: number;
  now_playing_ticker: string;
  lanes: ConciliumAgentLane[];
  wakeup_arcs: ConciliumWakeupArc[];
  board_tail: SequenceBoardEntry[];
}

export interface DockedSwarmWorkerPill {
  agent_id: string;
  roster_name: string;
  status: 'working' | 'waiting' | 'parked' | 'tripped' | 'idle';
  tok_per_sec: number;
  workspace_mode: SwarmWorkspaceMode;
  file_lock_prefix?: string | null;
}

export interface HybridComputeNodeCardDto {
  node_id: string;
  display_name: string;
  tier: ComputeTier;
  status: 'online' | 'maintenance' | 'quarantined' | 'degraded';
  host_triple?: string | null;
  gpu_summary?: string | null;
  vram_used_mb?: number | null;
  vram_total_mb?: number | null;
  kv_cache_warm_prefixes?: number | null;
  rtt_ms?: number | null;
  bandit_weight?: number | null;
  advertised_models: string[];
  docked_workers: DockedSwarmWorkerPill[];
  last_seen_unix_ms?: number | null;
}

export interface FileLockCellDto {
  prefix: string;
  holder_agent: string;
  exclusive: boolean;
  compute_tier: ComputeTier;
  workspace_mode: SwarmWorkspaceMode;
  waiters: string[];
  contended: boolean;
  acquired_seq: number;
}

export interface PopuliPlacementDto {
  generated_at_ms: number;
  head_seq: number;
  gossip_cadence_ms: number;
  source: string;
  queue_depth: number;
  nodes: HybridComputeNodeCardDto[];
  lock_matrix: FileLockCellDto[];
}

export type ConciliumPilotActionKind =
  | 'park_agent'
  | 'steer_agent'
  | 'fork_agent'
  | 'swap_agent_compute';

export interface ConciliumPilotActionInput {
  action: ConciliumPilotActionKind;
  agent_id: string;
  channel?: string | null;
  directive?: string | null;
  fork_name?: string | null;
  workspace_mode?: SwarmWorkspaceMode | null;
  target_tier?: ComputeTier | null;
  target_node_id?: string | null;
  target_model_id?: string | null;
}

export interface ConciliumPilotActionResult {
  ok: boolean;
  seq: number;
  message: string;
  forked_agent_id?: string | null;
}
