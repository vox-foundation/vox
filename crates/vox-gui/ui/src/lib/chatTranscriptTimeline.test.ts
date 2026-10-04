import { describe, it, expect } from 'vitest';
import {
  buildChatOnlyTimeline,
  isTokenStreamEvent,
} from './chatTranscriptTimeline';
import type { ChatMessage } from './chatCorrelation';
import type { StreamItem } from '../types/dashboard';

function agentItem(
  id: string,
  eventType: string,
  timestampMs: number,
  extra: Partial<StreamItem> = {},
): StreamItem {
  return {
    id,
    kind: 'agent',
    tag: eventType === 'token_streamed' ? 'TOKEN' : 'TASK',
    title: eventType,
    body: '',
    ts: '12:00',
    metadata: { eventType, timestampMs, agentId: 'agent-1' },
    ...extra,
  };
}

describe('isTokenStreamEvent', () => {
  it('returns true for token_streamed metadata', () => {
    expect(isTokenStreamEvent(agentItem('1', 'token_streamed', 100))).toBe(true);
  });

  it('returns false for task_started', () => {
    expect(isTokenStreamEvent(agentItem('2', 'task_started', 200))).toBe(false);
  });
});


function msg(id: string, role: ChatMessage['role'], status: ChatMessage['status'] = 'done'): ChatMessage {
  return { id, role, text: 'hi', status } as ChatMessage;
}

function evt(id: string, tag: string, eventType: string, extra: Record<string, any> = {}): StreamItem {
  return {
    id,
    kind: 'agent',
    tag,
    title: tag,
    body: '',
    ts: 'now',
    metadata: { eventType, timestampMs: Number(id), ...extra },
  };
}

describe('buildChatOnlyTimeline', () => {
  it('excludes all raw agent event rows (CHECKPOINT/TASK/PHASE/COST/TOKEN) from the chat-only list', () => {
    const messages = [msg('m1', 'user'), msg('m2', 'assistant')];
    const events = [
      evt('1', 'CHECKPOINT', 'snapshot_captured'),
      evt('2', 'TASK', 'task_started'),
      evt('3', 'PHASE', 'task_phase_changed'),
      evt('4', 'COST', 'cost_incurred'),
      evt('5', 'TOKEN', 'token_streamed'),
    ];
    const rows = buildChatOnlyTimeline(messages, events);
    expect(rows.every((r) => r.kind === 'message' || r.kind === 'status')).toBe(true);
  });

  it('produces exactly one live status row while a task is in-flight, with phase and elapsed time', () => {
    const messages = [msg('m1', 'user')];
    const events = [
      evt('1', 'TASK', 'task_started', { taskId: 7 }),
      evt('2', 'PHASE', 'task_phase_changed', { taskId: 7, phase: 'Verify' }),
    ];
    const rows = buildChatOnlyTimeline(messages, events, { nowMs: 12_000 });
    const statusRows = rows.filter((r) => r.kind === 'status');
    expect(statusRows).toHaveLength(1);
    expect(statusRows[0]).toMatchObject({ kind: 'status', phase: 'Verify', taskId: 7 });
    // task_started's event id is '1', so its timestampMs (via Number(id)) is 1.
    expect(statusRows[0].elapsedMs).toBe(12_000 - 1);
  });

  it('removes the status row once the task completes', () => {
    const messages = [msg('m1', 'user'), msg('m2', 'assistant')];
    const events = [
      evt('1', 'TASK', 'task_started', { taskId: 7 }),
      evt('2', 'PHASE', 'task_phase_changed', { taskId: 7, phase: 'Verify' }),
      evt('3', 'TASK', 'task_completed', { taskId: 7 }),
    ];
    const rows = buildChatOnlyTimeline(messages, events);
    expect(rows.some((r) => r.kind === 'status')).toBe(false);
  });

  it('normal verbosity adds a done-summary row after a task completes, using its cost_incurred data', () => {
    const messages = [msg('m1', 'user'), msg('m2', 'assistant')];
    const events = [
      evt('1', 'TASK', 'task_started', { taskId: 7 }),
      evt('2', 'COST', 'cost_incurred', { taskId: 7, costUsd: 0.003 }),
      evt('3', 'TASK', 'task_completed', { taskId: 7 }),
    ];
    const rows = buildChatOnlyTimeline(messages, events, { verbosity: 'normal' });
    const summary = rows.find((r) => r.kind === 'summary');
    expect(summary).toMatchObject({ kind: 'summary', taskId: 7, costUsd: 0.003 });
  });

  it('quiet verbosity omits the summary row even after completion', () => {
    const messages = [msg('m1', 'user'), msg('m2', 'assistant')];
    const events = [
      evt('1', 'TASK', 'task_started', { taskId: 7 }),
      evt('2', 'COST', 'cost_incurred', { taskId: 7, costUsd: 0.003 }),
      evt('3', 'TASK', 'task_completed', { taskId: 7 }),
    ];
    const rows = buildChatOnlyTimeline(messages, events, { verbosity: 'quiet' });
    expect(rows.some((r) => r.kind === 'summary')).toBe(false);
  });
});

import { mapAgentEvent } from './mapAgentEvent';

describe('buildChatOnlyTimeline through the real event mapper', () => {
  // The existing tests hand-build metadata; these go through mapAgentEvent, the producer the app uses.
  const frame = (id: number, type: string, extra: Record<string, unknown>) =>
    mapAgentEvent({ id, timestamp_ms: id * 1000, kind: { type, ...extra } });

  it('shows the phase the engine reported', () => {
    const rows = buildChatOnlyTimeline([], [
      frame(1, 'task_started', { task_id: 7, agent_id: 3 }),
      frame(2, 'task_phase_changed', { task_id: 7, agent_id: 3, phase: 'Act' }),
    ], { nowMs: 5000 });
    expect(rows.find(r => r.kind === 'status')).toMatchObject({ taskId: 7, phase: 'Act' });
  });

  it('attributes each agent cost to that agent task and shows the total when the task completes', () => {
    const rows = buildChatOnlyTimeline([], [
      frame(1, 'task_started', { task_id: 7, agent_id: 3 }),
      frame(2, 'cost_incurred', { agent_id: 3, cost_usd: 0.25, provider: 'acme', model: 'acme/widget' }),
      frame(3, 'cost_incurred', { agent_id: 3, cost_usd: 0.5, provider: 'acme', model: 'acme/widget' }),
      frame(4, 'task_completed', { task_id: 7, agent_id: 3 }),
    ], { nowMs: 5000 });
    expect(rows.find(r => r.kind === 'summary')).toMatchObject({ taskId: 7, costUsd: 0.75 });
  });
});

