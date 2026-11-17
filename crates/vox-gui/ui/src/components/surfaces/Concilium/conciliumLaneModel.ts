import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { AgentEventFrame } from '../../../transport';
import type {
  ComputeTier,
  ConciliumAgentLane,
  ConciliumPilotActionInput,
  ConciliumSnapshotDto,
  SequenceBoardEntry,
  SwarmWorkspaceMode,
} from '../../../types/concilium';
import type { Agent } from '../../../types/dashboard';
import type { SubAgentNode } from '../SubAgents/types';

export { DEFAULT_SNAPSHOT } from './conciliumDefaultSnapshot';

function flattenSubAgents(nodes: SubAgentNode[]): SubAgentNode[] {
  const out: SubAgentNode[] = [];
  const walk = (list: SubAgentNode[]) => {
    for (const n of list) {
      out.push(n);
      walk(n.children);
    }
  };
  walk(nodes);
  return out;
}

export function buildLanesFromLegacy(
  legacyAgents?: Agent[],
  subAgentNodes?: SubAgentNode[],
): ConciliumAgentLane[] | null {
  if (subAgentNodes && subAgentNodes.length > 0) {
    return flattenSubAgents(subAgentNodes).map((n, idx) => ({
      agent_id: n.windowId,
      roster_name: n.title,
      role: n.skill?.toUpperCase() ?? 'SUB-AGENT',
      status: n.status === 'running' ? 'working' : 'parked',
      compute_tier: idx % 2 === 0 ? 'cloud_bandit' : 'local_mens_gpu',
      node_id: `window-${n.windowId}`,
      model_id: n.model.id,
      workspace_mode: 'share',
      file_lock_prefix: `crates/vox-orchestrator/window/${n.windowId}`,
      parked_since_seq: n.status === 'running' ? null : 115 + idx,
      tok_per_sec: n.status === 'running' ? 160 : 0,
      used_tokens: n.usedTokens,
      max_tokens: n.model.maxTokens || 200000,
      kv_cache_hit_pct: 89,
      current_action: `${n.title} (${n.status})`,
      spans: [
        {
          span_id: `sp-${n.windowId}`,
          traceparent: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b${idx % 9}-01`,
          kind: n.status === 'running' ? 'tool_call' : 'parked',
          label: n.title,
          detail: n.skill,
          start_offset_ms: 600,
          duration_ms: 5400,
          file_lock_prefix: `crates/vox-orchestrator/window/${n.windowId}`,
          woken_by_seq: 118,
        },
      ],
    }));
  }
  if (legacyAgents && legacyAgents.length > 0) {
    const tiers: ComputeTier[] = ['cloud_bandit', 'local_mens_gpu', 'populi_quic_peer'];
    return legacyAgents.map((a, idx) => ({
      agent_id: a.id,
      roster_name: a.codename,
      role: a.phase.toUpperCase(),
      status: a.phase === 'Paused' ? 'parked' : 'working',
      compute_tier: tiers[idx % tiers.length]!,
      node_id: `node-${a.id}`,
      model_id: a.skill ?? 'anthropic/claude-sonnet',
      workspace_mode: 'share',
      file_lock_prefix: 'crates/vox-orchestrator/src/router.rs',
      parked_since_seq: a.phase === 'Paused' ? 116 : null,
      tok_per_sec: a.phase === 'Paused' ? 0 : 184,
      used_tokens: Math.round((a.progress ?? 0.5) * 64000),
      max_tokens: 128000,
      kv_cache_hit_pct: 92,
      current_action: a.task,
      spans: [
        {
          span_id: `sp-${a.id}`,
          traceparent: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902a${idx % 9}-01`,
          kind: a.phase === 'Paused' ? 'parked' : 'thinking',
          label: a.task || a.codename,
          detail: a.phase,
          start_offset_ms: 500,
          duration_ms: 6200,
          file_lock_prefix: 'crates/vox-orchestrator/src/router.rs',
          woken_by_seq: 119,
        },
      ],
    }));
  }
  return null;
}

function asComputeTier(raw: unknown): ComputeTier {
  if (
    raw === 'cloud_bandit' ||
    raw === 'local_mens_gpu' ||
    raw === 'populi_quic_peer' ||
    raw === 'inline_cpu'
  ) {
    return raw;
  }
  return 'cloud_bandit';
}

function asWorkspaceMode(raw: unknown): SwarmWorkspaceMode {
  if (raw === 'inherit' || raw === 'share' || raw === 'branch') return raw;
  return 'share';
}

export function mergeBoardTail(
  existing: SequenceBoardEntry[],
  incoming: SequenceBoardEntry[],
): SequenceBoardEntry[] {
  const bySeq = new Map<number, SequenceBoardEntry>();
  for (const e of existing) bySeq.set(e.seq, e);
  for (const e of incoming) bySeq.set(e.seq, e);
  return [...bySeq.values()].sort((a, b) => a.seq - b.seq).slice(-50);
}

export function applyAgentEventFrame(
  prev: ConciliumSnapshotDto,
  frame: AgentEventFrame,
): ConciliumSnapshotDto {
  const eventType = typeof frame.kind?.type === 'string' ? frame.kind.type : 'agent_event';
  const rawSender = frame.kind?.agent_id ?? frame.kind?.sender;
  const sender =
    typeof rawSender === 'string' || typeof rawSender === 'number'
      ? `${rawSender}`
      : 'coordinator';
  const rawChannel = frame.kind?.channel;
  const channel = typeof rawChannel === 'string' && rawChannel.trim() ? rawChannel : '#main';
  const stampedSeq =
    typeof frame.kind?.swarm_seq === 'number' && Number.isFinite(frame.kind.swarm_seq)
      ? frame.kind.swarm_seq
      : 0;
  const nextSeq = Math.max(prev.head_seq + 1, stampedSeq);
  const rawMsg = frame.kind?.message;
  const safeBody =
    typeof rawMsg === 'string' && rawMsg.trim().length > 0
      ? sanitizeErrorForToast(rawMsg)
      : `${eventType} on ${sender}`;
  const rawTraceparent = frame.kind?.traceparent;
  const traceparent =
    typeof rawTraceparent === 'string' && rawTraceparent.trim().length > 0
      ? rawTraceparent
      : `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba9${nextSeq.toString().padStart(4, '0')}-01`;
  const rawSpanId = frame.kind?.span_id;
  const spanId =
    typeof rawSpanId === 'string' && rawSpanId.trim().length > 0
      ? rawSpanId
      : `sp-live-${nextSeq}`;
  const rawParked = frame.kind?.parked_since_seq;
  const parkedSinceSeq =
    typeof rawParked === 'number' && Number.isFinite(rawParked) ? rawParked : null;

  const entry: SequenceBoardEntry = {
    seq: nextSeq,
    channel,
    sender,
    body: safeBody,
    mentions: Array.isArray(frame.kind?.mentions)
      ? frame.kind.mentions.filter((m): m is string => typeof m === 'string')
      : [],
    timestamp_ms: frame.timestamp_ms || Date.now(),
    traceparent,
    span_id: spanId,
    compute_tier: asComputeTier(frame.kind?.compute_tier),
    file_lock_prefix:
      typeof frame.kind?.file_lock_prefix === 'string' ? frame.kind.file_lock_prefix : null,
    workspace_mode: asWorkspaceMode(frame.kind?.workspace_mode),
    parked_since_seq: parkedSinceSeq,
    receipt_badge: eventType,
    is_watchdog_alert: eventType.includes('alert') || eventType.includes('trip'),
  };

  return {
    ...prev,
    head_seq: nextSeq,
    now_playing_ticker: `@${sender} · ${eventType} · #seq-${nextSeq}`,
    board_tail: mergeBoardTail(prev.board_tail, [entry]),
  };
}

export function applyPilotActionToSnapshot(
  prev: ConciliumSnapshotDto,
  input: ConciliumPilotActionInput,
  seedLanes: ConciliumAgentLane[] | null,
  serverSeq?: number,
): ConciliumSnapshotDto {
  const nextSeq =
    typeof serverSeq === 'number' && Number.isFinite(serverSeq) && serverSeq > prev.head_seq
      ? serverSeq
      : prev.head_seq + 1;
  const baseLanes = seedLanes && seedLanes.length > 0 ? seedLanes : prev.lanes;
  let updatedLanes = baseLanes.map((l) => {
    if (l.agent_id !== input.agent_id) return l;
    if (input.action === 'park_agent') {
      return { ...l, status: 'parked' as const, tok_per_sec: 0, parked_since_seq: nextSeq };
    }
    if (input.action === 'swap_agent_compute' && input.target_tier) {
      return { ...l, compute_tier: input.target_tier };
    }
    if (input.action === 'steer_agent' && input.directive) {
      return { ...l, status: 'working' as const, current_action: `Steered: ${input.directive}` };
    }
    return l;
  });

  if (input.action === 'fork_agent') {
    const src = baseLanes.find((l) => l.agent_id === input.agent_id) ?? baseLanes[0];
    if (src) {
      const forkId = input.fork_name ?? `${src.roster_name}-fork`;
      updatedLanes = [
        ...updatedLanes,
        {
          ...src,
          agent_id: forkId,
          roster_name: forkId,
          workspace_mode: input.workspace_mode ?? 'share',
        },
      ];
    }
  }

  const nextBoard =
    input.action === 'steer_agent'
      ? mergeBoardTail(prev.board_tail, [
          {
            seq: nextSeq,
            channel: input.channel ?? '#main',
            sender: 'operator',
            body: `@${input.agent_id.replace(/^@/, '')} ${input.directive ?? ''}`.trim(),
            mentions: [`@${input.agent_id.replace(/^@/, '')}`],
            timestamp_ms: Date.now(),
            traceparent: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba9${nextSeq.toString().padStart(4, '0')}-01`,
            span_id: `sp-steer-${nextSeq}`,
            compute_tier: 'cloud_bandit',
            file_lock_prefix: null,
            workspace_mode: 'share',
            parked_since_seq: null,
            receipt_badge: 'pilot:steer',
            is_watchdog_alert: false,
          },
        ])
      : prev.board_tail;

  return { ...prev, head_seq: nextSeq, lanes: updatedLanes, board_tail: nextBoard };
}
