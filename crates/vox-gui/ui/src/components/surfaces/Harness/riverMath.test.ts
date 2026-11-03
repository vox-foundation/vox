// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { deriveTierCascade, deriveRiverSnapshot } from './riverMath';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import type { ChatExecutionTask } from '../Chat/ChatExecutionRail';

describe('deriveTierCascade', () => {
  it('maps efficiency clutch + moderate risk + routing into 3 distinct harness tiers', () => {
    const tiers = deriveTierCascade({
      clutch: 'efficiency',
      risk: 'moderate',
      chatModelOverride: null,
      routing: {
        model: 'Sonnet 4.6',
        reason: 'balanced graph fan-out',
        state: 'catalog',
        alternatives: ['Opus 4.6', 'Haiku 4.5'],
      },
    });

    expect(tiers).toHaveLength(3);
    expect(tiers[0].tierId).toBe('intercept');
    expect(tiers[0].modelLabel).toBe('Haiku 4.5');
    expect(tiers[1].tierId).toBe('fanout');
    expect(tiers[1].modelLabel).toBe('Auto → Sonnet 4.6');
    expect(tiers[2].tierId).toBe('verifier');
    expect(tiers[2].badge).toMatch(/COMPILE \+ TEST/i);
  });

  it('respects explicit chatModelOverride on Tier 2 Fan-Out and elevates Tier 3 on low risk (strict)', () => {
    const tiers = deriveTierCascade({
      clutch: 'genius',
      risk: 'low',
      chatModelOverride: 'local/gemini-argon',
      routing: null,
    });

    expect(tiers[1].modelLabel).toBe('Pinned → local/gemini-argon');
    expect(tiers[2].tierId).toBe('verifier');
    expect(tiers[2].badge).toMatch(/SOCRATIC RED-TEAM/i);
  });
});

describe('deriveRiverSnapshot', () => {
  it('computes BLAKE3 cache thrift, 4-stage funnel, and 8-slice agent trajectories from tasks and messages', () => {
    const tasks: ChatExecutionTask[] = [
      { id: 't-1', title: 'Implement BLAKE3 receipt check', status: 'in_progress' },
      { id: 't-2', title: 'Verify cold nextest suite', status: 'completed' },
    ];
    const messages: ChatMessage[] = [
      {
        id: 'm-1',
        role: 'assistant',
        text: 'Patched module and verified tests.',
        status: 'done',
        runId: 'r-1',
        taskId: 't-2',
        latencyMs: 1420,
        events: [
          { kind: 'tool_called', name: 'vox_read_file' },
          { kind: 'verify_passed', summary: 'cargo test passed' },
        ],
      },
    ];

    const snap = deriveRiverSnapshot({
      tasks,
      messages,
      sessionSpentUsd: 0.14,
      clutch: 'efficiency',
      risk: 'low',
      chatModelOverride: null,
      routing: null,
    });

    expect(snap.activeAgents).toBe(1);
    expect(snap.cacheHitPct).toBeGreaterThan(50);
    expect(snap.tokenSavingsPct).toBeGreaterThan(50);
    expect(snap.funnel).toHaveLength(4);
    expect(snap.funnel[0].stageId).toBe('candidates');
    expect(snap.funnel[3].stageId).toBe('adversarial_pass');
    expect(snap.funnel[0].count).toBeGreaterThanOrEqual(snap.funnel[3].count);
    expect(snap.trajectories.length).toBeGreaterThan(0);
    expect(snap.trajectories[0].slices).toHaveLength(8);
    expect(snap.ludusYield.crystalsEarned).toBeGreaterThanOrEqual(1);
  });

  it('accurately counts multi-agent fan-out and gracefully handles missing task titles', () => {
    const tasks: ChatExecutionTask[] = [
      { id: 't-1', title: '', status: 'in_progress' },
      { id: 't-2', title: 'Task Two', status: 'running' },
      { id: 't-3', title: 'Task Three', status: 'queued' },
      { id: 't-4', title: 'Task Four', status: 'completed' },
    ];
    const messages: ChatMessage[] = [
      { id: 'm-1', role: 'assistant', text: 'Streaming response...', status: 'streaming' },
    ];

    const snap = deriveRiverSnapshot({
      tasks,
      messages,
      sessionSpentUsd: 0.20,
      clutch: 'genius',
      risk: 'low',
      chatModelOverride: null,
      routing: null,
    });

    // 3 in-progress/running/queued + 1 streaming = 4 active agents
    expect(snap.activeAgents).toBe(4);
    // Trajectory for task without title falls back to task id
    expect(snap.trajectories[0].label).toBe('t-1');
  });
});
