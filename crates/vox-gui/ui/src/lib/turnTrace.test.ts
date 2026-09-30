import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { TurnEventDto } from '../types/dashboard';
import { buildTurnTrace, isInterrupt, summaryText } from './turnTrace';

const CONTRACT: { kinds: Array<{ kind: string; example: TurnEventDto }> } = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), '../../../../../contracts/gui/turn-event-kinds.v1.json'),
    'utf8',
  ),
);

function example(kind: string): TurnEventDto {
  const entry = CONTRACT.kinds.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

const EMPTY = {
  modelLabel: null,
  tools: 0,
  receiptsVerified: 0,
  receiptsUnverified: 0,
  delegations: 0,
  researchWaves: 0,
  durationMs: null,
};

describe('buildTurnTrace', () => {
  it('summarises a clean turn from the contract examples', () => {
    const t = buildTurnTrace(
      [example('routing_decision'), example('tool_receipt'), example('delegation_spawned'), example('research_milestone')],
      'normal',
      { latencyMs: 4100 },
    );
    expect(t.summary).toEqual({
      modelLabel: 'acme/widget-5.5',
      tools: 1,
      receiptsVerified: 1,
      receiptsUnverified: 0,
      delegations: 1,
      researchWaves: 3,
      durationMs: 4100,
    });
    expect(t.steps.map((s) => s.event.kind)).toEqual([
      'routing_decision',
      'tool_receipt',
      'delegation_spawned',
      'research_milestone',
    ]);
    expect(t.interrupts).toEqual([]);
    expect(t.defaultExpanded).toBe(false);
    expect(summaryText(t.summary)).toBe(
      'acme/widget-5.5 · 1 tool · 1 receipt ✓ · 1 delegated · 3 research waves · 4.1s',
    );
  });

  it('shows the family marked offline for a bootstrap pick and the local id for a local pick', () => {
    const offline = buildTurnTrace([{ ...example('routing_decision'), resolved_from: 'bootstrap' }], 'normal');
    expect(offline.summary.modelLabel).toBe('acme/widget (offline)');
    const local = buildTurnTrace(
      [{ ...example('routing_decision'), resolved_from: 'local', resolved_id: 'mens/run-7' }],
      'normal',
    );
    expect(local.summary.modelLabel).toBe('mens/run-7 (local)');
  });

  it('falls back to the message modelId, plainly, only when no routing decision exists', () => {
    const hydrated = buildTurnTrace(undefined, 'normal', { modelId: 'mens/run-7', latencyMs: 1200 });
    expect(hydrated.summary.modelLabel).toBe('mens/run-7');
    expect(summaryText(hydrated.summary)).toBe('mens/run-7 · 1.2s');
    const both = buildTurnTrace([{ ...example('routing_decision'), resolved_from: 'bootstrap' }], 'normal', {
      modelId: 'acme/widget-5.5',
    });
    expect(both.summary.modelLabel).toBe('acme/widget (offline)');
  });

  it('keeps skill activation inline and makes a flagged claims verdict an interrupt, not a step', () => {
    const t = buildTurnTrace([example('skill_activated'), example('receipt_claims'), example('tool_receipt')], 'quiet');
    expect(t.inline.map((e) => e.kind)).toEqual(['skill_activated']);
    expect(t.interrupts.map((i) => i.event.kind)).toEqual(['receipt_claims']);
    expect(t.steps.map((s) => s.event.kind)).toEqual(['tool_receipt']);
    expect(t.defaultExpanded).toBe(false);
  });

  it('a clean claims verdict is a step, not an interrupt', () => {
    const t = buildTurnTrace([{ ...example('receipt_claims'), fabricated: 0, unverified: 0 }], 'normal');
    expect(t.interrupts).toEqual([]);
    expect(t.steps).toHaveLength(1);
    expect(t.steps[0].status).toBe('ok');
  });

  it('an unverified receipt without a claims verdict is a failed step, not an interrupt', () => {
    const t = buildTurnTrace([{ ...example('tool_receipt'), verified: false }], 'normal');
    expect(t.interrupts).toEqual([]);
    expect(t.steps[0].status).toBe('failed');
    expect(t.summary.receiptsUnverified).toBe(1);
    expect(t.defaultExpanded).toBe(true);
  });

  it('coalesces identical consecutive events into one step with a count', () => {
    const r = example('research_milestone');
    const t = buildTurnTrace([r, { ...r }, { ...r }], 'quiet');
    expect(t.steps).toHaveLength(1);
    expect(t.steps[0]).toMatchObject({ count: 3, status: 'ok' });
    expect(t.summary.researchWaves).toBe(9);
  });

  it('does not coalesce events that differ or are not adjacent', () => {
    const r = example('research_milestone');
    const t = buildTurnTrace([r, { ...r, waves_executed: 4 }, r], 'normal');
    expect(t.steps.map((s) => s.count)).toEqual([1, 1, 1]);
  });

  it('drops unknown kinds and events missing a required field', () => {
    const noReceiptId: TurnEventDto = { ...example('tool_receipt') };
    delete noReceiptId.receipt_id;
    const t = buildTurnTrace([{ kind: 'from_the_future' }, noReceiptId, example('routing_decision')], 'normal');
    expect(t.steps.map((s) => s.event.kind)).toEqual(['routing_decision']);
    expect(t.summary.tools).toBe(0);
  });

  it('default expansion follows verbosity', () => {
    const clean = [example('routing_decision'), example('tool_receipt')];
    const failed = [example('routing_decision'), { ...example('tool_receipt'), verified: false }];
    const interrupted = [...clean, example('receipt_claims')];
    expect(['quiet', 'normal', 'verbose'].map((v) => buildTurnTrace(clean, v as 'quiet').defaultExpanded)).toEqual([
      false,
      false,
      true,
    ]);
    expect(buildTurnTrace(failed, 'quiet').defaultExpanded).toBe(false);
    expect(buildTurnTrace(failed, 'normal').defaultExpanded).toBe(true);
    expect(buildTurnTrace(interrupted, 'normal').defaultExpanded).toBe(true);
  });

  it('treats an absent event list as an empty trace', () => {
    const t = buildTurnTrace(undefined, 'verbose');
    expect(t.steps).toEqual([]);
    expect(t.summary).toEqual(EMPTY);
  });
});

describe('isInterrupt', () => {
  it('is only a flagged claims verdict among live kinds', () => {
    expect(isInterrupt(example('receipt_claims'))).toBe(true);
    expect(isInterrupt({ ...example('receipt_claims'), fabricated: 0, unverified: 2 })).toBe(true);
    expect(isInterrupt({ ...example('receipt_claims'), fabricated: 0, unverified: 0 })).toBe(false);
    for (const kind of ['skill_activated', 'delegation_spawned', 'research_milestone', 'tool_receipt', 'routing_decision']) {
      expect(isInterrupt(example(kind)), kind).toBe(false);
    }
  });
});

describe('summaryText', () => {
  it('reports verified out of total when a receipt failed', () => {
    expect(summaryText({ ...EMPTY, tools: 3, receiptsVerified: 2, receiptsUnverified: 1 })).toBe(
      '3 tools · 2/3 receipts verified',
    );
  });

  it('is empty for an empty summary', () => {
    expect(summaryText(EMPTY)).toBe('');
  });
});
