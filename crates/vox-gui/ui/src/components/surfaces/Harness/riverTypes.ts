import type { ClutchId, RiskId } from '../../../lib/driveConsole';
import type { RailRouting } from '../../../lib/routingSummary';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { ChatExecutionTask } from '../Chat/ChatExecutionRail';

export type HarnessTierId = 'intercept' | 'fanout' | 'verifier';

export type AgentRoleLane = 'INTERCEPT' | 'FAN_OUT' | 'VERIFY' | 'SANDBOX';

export interface TierCascadeAssignment {
  tierId: HarnessTierId;
  tierNumber: 1 | 2 | 3;
  title: string;
  modelLabel: string;
  badge: string;
  latencySlo: string;
  tokenDeltaLabel: string;
}

export interface RiverStageProgress {
  id: string;
  label: string;
  pct: number;
  active: boolean;
}

export interface FunnelStage {
  stageId: 'candidates' | 'ast_lint' | 'cold_tests' | 'adversarial_pass';
  label: string;
  count: number;
  retentionPct: number;
}

export interface FailureAttributionRow {
  code: string;
  label: string;
  interceptedCount: number;
  sharePct: number;
}

export interface AgentTrajectoryRow {
  agentId: string;
  label: string;
  /** 8 historical slices, each mapping to lane index 0..3 ('INTERCEPT' | 'FAN_OUT' | 'VERIFY' | 'SANDBOX'). */
  slices: number[];
  activeLane: AgentRoleLane;
}

export interface RiverStrand {
  id: string;
  sourceSlot: number; // 0..1 normalized X at top
  targetSlot: number; // 0..1 normalized X at bottom
  intensity: number;  // 0.2..1.0
  status: 'cached' | 'streaming' | 'verified' | 'doubted';
  hashPrefix: string;
}

export interface LudusHarnessYield {
  xpEarned: number;
  crystalsEarned: number;
  thriftRankLabel: string;
}

export interface RiverSnapshot {
  tiers: TierCascadeAssignment[];
  throughputTokPerSec: number;
  cacheHitPct: number;
  tokenSavingsPct: number;
  savedUsd: number;
  activeAgents: number;
  activePhaseBanner: string;
  activePhaseSubtext: string;
  waveformBars: number[]; // 24 normalized bars 0.1..1.0
  stages: RiverStageProgress[];
  strands: RiverStrand[];
  trajectories: AgentTrajectoryRow[];
  funnel: FunnelStage[];
  failures: FailureAttributionRow[];
  ludusYield: LudusHarnessYield;
}

export interface TierCascadeInput {
  clutch: ClutchId;
  risk: RiskId;
  chatModelOverride?: string | null;
  routing?: RailRouting | null;
}

export interface RiverCorrelationInput extends TierCascadeInput {
  tasks: ChatExecutionTask[];
  messages: ChatMessage[];
  sessionSpentUsd?: number | null;
}
