/**
 * Bi-directional Human <-> LLM Task Ledger CAS Revision & Delta Synchronization Bus.
 *
 * Bridges `ChatExecutionRail`, `PlanPanel`, and `Loquela` so that user task mutations
 * (add, rename, status toggle, priority reorder, remove, doubt, force-verify, approve)
 * increment a monotonic CAS revision (`rev`) and emit a structured `<task_ledger_delta>`
 * envelope for the LLM's next turn without clobbering concurrent agent updates.
 */

export type TaskLedgerOpKind =
  | 'add'
  | 'edit'
  | 'rename'
  | 'status'
  | 'priority'
  | 'remove'
  | 'doubt'
  | 'verify'
  | 'approve';

export interface TaskLedgerDeltaEntry {
  rev: number;
  tsMs: number;
  kind: TaskLedgerOpKind;
  taskId: string;
  summary: string;
  title?: string;
  status?: string;
  priority?: number;
  reason?: string;
  actor?: 'human' | 'llm';
}

export interface TaskLedgerSnapshot {
  rev: number;
  lastAckedRev: number;
  pendingDeltas: TaskLedgerDeltaEntry[];
}

export interface TaskLedgerDeltaEventDetail {
  delta: TaskLedgerDeltaEntry | null;
  entry: TaskLedgerDeltaEntry | null;
  snapshot: TaskLedgerSnapshot;
}

export const TASK_LEDGER_DELTA_EVENT = 'vox:task-ledger-delta';

let currentRev = 1;
let lastAckedRev = 1;
let pendingDeltas: TaskLedgerDeltaEntry[] = [];

export function getTaskLedgerSnapshot(): TaskLedgerSnapshot {
  return {
    rev: currentRev,
    lastAckedRev,
    pendingDeltas: [...pendingDeltas],
  };
}

export function recordTaskLedgerOp(
  op: Omit<TaskLedgerDeltaEntry, 'rev' | 'tsMs' | 'summary'> & { summary?: string },
): TaskLedgerDeltaEntry {
  currentRev += 1;
  const computedSummary =
    op.summary ??
    `${op.kind} ${op.taskId}${op.title ? ` "${op.title}"` : ''}${op.status ? ` -> ${op.status}` : ''}${op.priority != null ? ` (P${op.priority})` : ''}`;
  const entry: TaskLedgerDeltaEntry = {
    ...op,
    summary: computedSummary,
    rev: currentRev,
    tsMs: Date.now(),
  };
  pendingDeltas = [...pendingDeltas.slice(-24), entry];
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent<TaskLedgerDeltaEventDetail>(TASK_LEDGER_DELTA_EVENT, {
        detail: {
          delta: entry,
          entry,
          snapshot: getTaskLedgerSnapshot(),
        },
      }),
    );
  }
  return entry;
}

export function formatTaskLedgerDeltaBlock(snapshot = getTaskLedgerSnapshot()): string {
  if (snapshot.pendingDeltas.length === 0) {
    return `<task_ledger_delta base_rev="${snapshot.lastAckedRev}" head_rev="${snapshot.rev}" status="synced" />`;
  }
  const lines = snapshot.pendingDeltas.map(
    (d) => `  - [rev:${d.rev}] ${d.kind.toUpperCase()} (${d.taskId}): ${d.summary}`,
  );
  return `<task_ledger_delta base_rev="${snapshot.lastAckedRev}" head_rev="${snapshot.rev}">\n${lines.join('\n')}\n</task_ledger_delta>`;
}

export function acknowledgeTaskLedgerDeltas(): TaskLedgerSnapshot {
  lastAckedRev = currentRev;
  pendingDeltas = [];
  const snap = getTaskLedgerSnapshot();
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent<TaskLedgerDeltaEventDetail>(TASK_LEDGER_DELTA_EVENT, {
        detail: {
          delta: null,
          entry: null,
          snapshot: snap,
        },
      }),
    );
  }
  return snap;
}

export function __resetTaskLedgerForTests(): void {
  currentRev = 1;
  lastAckedRev = 1;
  pendingDeltas = [];
}
