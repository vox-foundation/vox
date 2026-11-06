import { describe, it, expect, beforeEach } from 'vitest';
import {
  __resetTaskLedgerForTests,
  acknowledgeTaskLedgerDeltas,
  formatTaskLedgerDeltaBlock,
  getTaskLedgerSnapshot,
  recordTaskLedgerOp,
} from './taskLedgerSync';

describe('taskLedgerSync (Human <-> LLM CAS Ledger)', () => {
  beforeEach(() => {
    __resetTaskLedgerForTests();
  });

  it('increments monotonic CAS rev and formats <task_ledger_delta> preamble', () => {
    expect(getTaskLedgerSnapshot().rev).toBe(1);
    expect(formatTaskLedgerDeltaBlock()).toBe(
      '<task_ledger_delta base_rev="1" head_rev="1" status="synced" />',
    );

    recordTaskLedgerOp({
      kind: 'add',
      taskId: 't-101',
      title: 'Wire BPE slice breakdown',
      priority: 1,
      actor: 'human',
    });
    recordTaskLedgerOp({
      kind: 'status',
      taskId: 't-101',
      status: 'completed',
      actor: 'human',
    });

    const snap = getTaskLedgerSnapshot();
    expect(snap.rev).toBe(3);
    expect(snap.pendingDeltas).toHaveLength(2);

    const block = formatTaskLedgerDeltaBlock(snap);
    expect(block).toContain('<task_ledger_delta base_rev="1" head_rev="3">');
    expect(block).toContain('[rev:2] ADD (t-101)');
    expect(block).toContain('[rev:3] STATUS (t-101)');

    const acked = acknowledgeTaskLedgerDeltas();
    expect(acked.lastAckedRev).toBe(3);
    expect(acked.pendingDeltas).toHaveLength(0);
    expect(formatTaskLedgerDeltaBlock(acked)).toBe(
      '<task_ledger_delta base_rev="3" head_rev="3" status="synced" />',
    );
  });
});
