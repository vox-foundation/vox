import type { ChatMessage } from './chatCorrelation';
import type { StreamItem } from '../types/dashboard';

export type TranscriptMessageRow = {
  kind: 'message';
  id: string;
  atMs: number;
  message: ChatMessage;
};

export function isTokenStreamEvent(item: StreamItem): boolean {
  const eventType = item.metadata?.eventType;
  if (typeof eventType === 'string') return eventType === 'token_streamed';
  return item.tag === 'TOKEN';
}

export type TranscriptStatusRow = {
  kind: 'status';
  id: string;
  atMs: number;
  taskId: number;
  phase: string;
  elapsedMs: number;
};

export type TranscriptSummaryRow = {
  kind: 'summary';
  id: string;
  atMs: number;
  taskId: number;
  costUsd: number;
};

export type ChatOnlyTimelineRow = TranscriptMessageRow | TranscriptStatusRow | TranscriptSummaryRow;

const IN_FLIGHT_EVENT_TYPES = new Set(['task_started', 'task_phase_changed']);
const TASK_END_EVENT_TYPES = new Set(['task_completed', 'task_failed']);

/**
 * Chat-feed-only view of the timeline: real messages plus at most one live
 * status row per in-flight task (phase + elapsed time), plus an optional
 * done-summary row once a task completes (verbosity-gated). Every raw agent
 * event (CHECKPOINT/TASK/PHASE/COST/TOKEN) is excluded here.
 */
export function buildChatOnlyTimeline(
  messages: ChatMessage[],
  agentItems: StreamItem[],
  options?: { messageStepMs?: number; nowMs?: number; verbosity?: 'quiet' | 'normal' | 'verbose' },
): ChatOnlyTimelineRow[] {
  const messageStepMs = options?.messageStepMs ?? 1000;
  const nowMs = options?.nowMs ?? Date.now();
  const verbosity = options?.verbosity ?? 'normal';

  const rows: ChatOnlyTimelineRow[] = messages.map((message, index) => ({
    kind: 'message' as const,
    id: message.id,
    atMs: index * messageStepMs,
    message,
  }));

  // Track the latest in-flight task per taskId, in arrival order, and drop
  // any task that has since completed/failed. Also track each task's total
  // cost and whether it completed, for the optional summary row.
  const inFlight = new Map<number, { phase: string; startedAtMs: number }>();
  const costByTask = new Map<number, number>();
  const completedTasks = new Set<number>();
  const taskByAgent = new Map<string, number>();

  for (const item of agentItems) {
    const eventType = item.metadata?.eventType;
    const agentId = item.metadata?.agentId as string | undefined;
    let taskId = item.taskId ?? (item.metadata?.taskId as number | undefined);
    if (taskId != null && agentId) taskByAgent.set(agentId, taskId);
    // Cost events carry only the agent; attribute them to that agent's current task.
    if (taskId == null && eventType === 'cost_incurred' && agentId) taskId = taskByAgent.get(agentId);
    if (taskId == null) continue;

    if (typeof eventType === 'string' && eventType === 'cost_incurred') {
      const costUsd = item.metadata?.costUsd;
      if (typeof costUsd === 'number') costByTask.set(taskId, (costByTask.get(taskId) ?? 0) + costUsd);
      continue;
    }
    if (typeof eventType === 'string' && TASK_END_EVENT_TYPES.has(eventType)) {
      inFlight.delete(taskId);
      if (eventType === 'task_completed') completedTasks.add(taskId);
      continue;
    }
    if (typeof eventType === 'string' && IN_FLIGHT_EVENT_TYPES.has(eventType)) {
      const ts = typeof item.metadata?.timestampMs === 'number' ? item.metadata.timestampMs : 0;
      const existing = inFlight.get(taskId);
      const startedAtMs = eventType === 'task_started' ? ts : (existing?.startedAtMs ?? ts);
      const phase =
        eventType === 'task_phase_changed' && typeof item.metadata?.phase === 'string'
          ? item.metadata.phase
          : (existing?.phase ?? 'Working');
      inFlight.set(taskId, { phase, startedAtMs });
    }
  }

  for (const [taskId, { phase, startedAtMs }] of inFlight) {
    rows.push({
      kind: 'status',
      id: `status-${taskId}`,
      atMs: nowMs,
      taskId,
      phase,
      elapsedMs: Math.max(0, nowMs - startedAtMs),
    });
  }

  if (verbosity !== 'quiet') {
    for (const taskId of completedTasks) {
      const costUsd = costByTask.get(taskId);
      if (costUsd == null) continue;
      rows.push({ kind: 'summary', id: `summary-${taskId}`, atMs: nowMs, taskId, costUsd });
    }
  }

  return rows;
}
