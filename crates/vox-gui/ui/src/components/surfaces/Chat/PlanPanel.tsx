import React, { useEffect, useState } from 'react';
import { EmptyState } from '../../ui/EmptyState';
import {
  updatePlanNode,
  insertPlanNode,
  approvePlanNodes,
  hopperMarkDone,
  hopperCancel,
  hopperSubmit,
  type PlanNodeStatus,
  type PlanNodeDto,
} from '../../../transport';
import {
  getTaskLedgerSnapshot,
  recordTaskLedgerOp,
  TASK_LEDGER_DELTA_EVENT,
  type TaskLedgerDeltaEventDetail,
} from '../../../lib/taskLedgerSync';

export type { PlanNodeStatus } from '../../../transport';
export type PlanNodeView = PlanNodeDto;

interface PlanPanelProps {
  planSessionId: string | null | undefined;
  planVersion: number | null | undefined;
  nodes: PlanNodeView[];
  /** Called when the user discards a still-unapproved plan (Task F2). No
   *  backend primitive for "discard" exists — the blocked nodes simply never
   *  dispatch — so this is a local dismissal the parent wires to clearing its
   *  own `planSessionId`/`planVersion` state. */
  onDiscard?: () => void;
}

const STATUS_ICON: Record<PlanNodeStatus, string> = {
  pending: '○',
  queued: '◔',
  in_progress: '◑',
  completed: '●',
  failed: '✕',
  cancelled: '−',
  superseded: '»',
  blocked_on_approval: '⏸',
};

const STATUS_COLOR: Record<PlanNodeStatus, string> = {
  pending: 'text-text-muted',
  queued: 'text-(--color-status-info)',
  in_progress: 'text-brass',
  completed: 'text-(--color-status-pass)',
  failed: 'text-(--color-status-fail)',
  cancelled: 'text-text-muted line-through',
  superseded: 'text-text-muted line-through',
  blocked_on_approval: 'text-(--color-status-warn)',
};

const EDITABLE_STATUSES = new Set<PlanNodeStatus>(['pending', 'queued', 'blocked_on_approval']);

function PlanNodeRow({
  node,
  planSessionId,
  planVersion,
  onToggleStatus,
  onRemoveNode,
}: {
  node: PlanNodeView;
  planSessionId: string;
  planVersion: number;
  onToggleStatus?: (nodeId: string, nextStatus: PlanNodeStatus) => void;
  onRemoveNode?: (nodeId: string) => void;
}) {
  const defaultEditable = EDITABLE_STATUSES.has(node.status);
  const [explicitEditing, setExplicitEditing] = useState(false);
  const editable = defaultEditable || explicitEditing;
  const [value, setValue] = useState(node.description);

  useEffect(() => {
    setValue(node.description);
  }, [node.description]);

  const commit = () => {
    setExplicitEditing(false);
    if (value === node.description) return;
    recordTaskLedgerOp({ kind: 'edit', taskId: node.node_id, title: value, actor: 'human' });
    if (planSessionId) {
      void updatePlanNode(planSessionId, planVersion, node.node_id, value);
    }
  };

  return (
    <div
      className="group flex items-center gap-2 py-1 text-[12px]"
      data-testid={`plan-node-${node.node_id}`}
    >
      <button
        type="button"
        data-testid={`plan-node-toggle-${node.node_id}`}
        aria-label={`Toggle step status: ${node.description}`}
        onClick={() =>
          onToggleStatus?.(
            node.node_id,
            node.status === 'completed' ? 'pending' : 'completed',
          )
        }
        className={`w-4 shrink-0 text-center transition hover:brightness-125 ${STATUS_COLOR[node.status]}`}
      >
        {STATUS_ICON[node.status]}
      </button>
      {editable ? (
        <input
          autoFocus={explicitEditing}
          aria-label={`Edit step: ${node.description}`}
          className="flex-1 rounded-sm border border-transparent bg-transparent px-1 text-text-secondary hover:border-border-subtle focus:border-brass/40 focus:outline-hidden"
          value={value}
          onChange={e => setValue(e.target.value)}
          onBlur={commit}
          onKeyDown={e => {
            if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
            if (e.key === 'Escape') setExplicitEditing(false);
          }}
        />
      ) : (
        <span
          onDoubleClick={() => setExplicitEditing(true)}
          className={`flex-1 ${node.status === 'cancelled' || node.status === 'superseded' ? 'text-text-muted line-through' : 'text-text-secondary'}`}
        >
          {value}
        </span>
      )}
      {!defaultEditable && !explicitEditing && node.status === 'in_progress' && (
        <button
          type="button"
          data-testid={`plan-node-edit-${node.node_id}`}
          aria-label={`Edit in-progress step: ${node.description}`}
          onClick={() => setExplicitEditing(true)}
          className="shrink-0 rounded px-1 font-mono text-[11px] text-text-muted opacity-0 transition group-hover:opacity-100 hover:text-brass focus:opacity-100"
        >
          ✎
        </button>
      )}
      {onRemoveNode && (
        <button
          type="button"
          data-testid={`plan-node-remove-${node.node_id}`}
          aria-label={`Remove step: ${node.description}`}
          onClick={() => onRemoveNode(node.node_id)}
          className="shrink-0 rounded px-1 font-mono text-[11px] text-text-muted opacity-0 transition group-hover:opacity-100 hover:text-(--color-status-fail) focus:opacity-100"
        >
          ×
        </button>
      )}
    </div>
  );
}

export function PlanPanel({ planSessionId, planVersion, nodes, onDiscard }: PlanPanelProps) {
  const [adding, setAdding] = useState(false);
  const [newDescription, setNewDescription] = useState('');
  const [approving, setApproving] = useState(false);
  const [localAdded, setLocalAdded] = useState<PlanNodeView[]>([]);
  const [statusOverrides, setStatusOverrides] = useState<Record<string, PlanNodeStatus>>({});
  const [descOverrides, setDescOverrides] = useState<Record<string, string>>({});
  const [removedIds, setRemovedIds] = useState<Set<string>>(new Set());
  const [ledgerRev, setLedgerRev] = useState<number>(() => getTaskLedgerSnapshot().rev);

  useEffect(() => {
    const onLedgerDelta = (ev: Event) => {
      const custom = ev as CustomEvent<TaskLedgerDeltaEventDetail>;
      const detail = custom.detail;
      if (!detail) return;
      setLedgerRev(detail.snapshot.rev);
      const { delta } = detail;
      if (!delta) return;
      if (delta.kind === 'add' && delta.title) {
        setLocalAdded(prev =>
          prev.some(n => n.node_id === delta.taskId)
            ? prev
            : [...prev, { node_id: delta.taskId, description: delta.title!, status: 'pending' }],
        );
      } else if (delta.kind === 'edit' && delta.title) {
        setDescOverrides(prev => ({ ...prev, [delta.taskId]: delta.title! }));
      } else if (delta.kind === 'status' && delta.status) {
        const mapped: PlanNodeStatus =
          delta.status === 'completed' || delta.status === 'done'
            ? 'completed'
            : delta.status === 'running'
              ? 'in_progress'
              : 'pending';
        setStatusOverrides(prev => ({ ...prev, [delta.taskId]: mapped }));
      } else if (delta.kind === 'remove') {
        setRemovedIds(prev => {
          const next = new Set(prev);
          next.add(delta.taskId);
          return next;
        });
      }
    };
    window.addEventListener(TASK_LEDGER_DELTA_EVENT, onLedgerDelta);
    return () => window.removeEventListener(TASK_LEDGER_DELTA_EVENT, onLedgerDelta);
  }, []);

  const mergedNodes = [
    ...nodes,
    ...localAdded.filter(a => !nodes.some(n => n.node_id === a.node_id)),
  ]
    .filter(n => !removedIds.has(n.node_id))
    .map(n => ({
      ...n,
      status: statusOverrides[n.node_id] ?? n.status,
      description: descOverrides[n.node_id] ?? n.description,
    }));

  const sessionId = planSessionId ?? '';
  const version = planVersion ?? 1;

  const submitNew = () => {
    const trimmed = newDescription.trim();
    if (!trimmed) return;
    const nodeId = `n-${crypto.randomUUID()}`;
    setLocalAdded(prev => [...prev, { node_id: nodeId, description: trimmed, status: 'pending' }]);
    recordTaskLedgerOp({ kind: 'add', taskId: nodeId, title: trimmed, status: 'pending', actor: 'human' });
    void hopperSubmit(trimmed, []).catch(() => {});
    if (planSessionId && planVersion != null) {
      void insertPlanNode(planSessionId, planVersion, nodeId, trimmed, []);
    }
    setNewDescription('');
    setAdding(false);
  };

  if ((!planSessionId || planVersion == null) && mergedNodes.length === 0) {
    return (
      <div data-testid="plan-panel" className="[&_.py-16]:py-6 flex flex-col gap-2 p-2">
        <EmptyState
          title="No to-dos yet"
          description="Start a task to see its to-do list here."
        />
        {adding ? (
          <input
            autoFocus
            className="rounded-sm border border-border-subtle bg-transparent px-1 text-[12px] text-text-secondary"
            placeholder="new step…"
            value={newDescription}
            onChange={e => setNewDescription(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') submitNew();
              if (e.key === 'Escape') setAdding(false);
            }}
            onBlur={submitNew}
          />
        ) : (
          <button
            type="button"
            onClick={() => setAdding(true)}
            className="self-center rounded border border-brass/30 bg-brass/10 px-2.5 py-1 font-mono text-[11px] text-brass transition hover:bg-brass/20"
          >
            + Add step
          </button>
        )}
      </div>
    );
  }

  const blockedCount = mergedNodes.filter(n => n.status === 'blocked_on_approval').length;

  const approve = async () => {
    if (!planSessionId) return;
    setApproving(true);
    try {
      await approvePlanNodes(planSessionId);
    } finally {
      setApproving(false);
    }
  };

  return (
    <div className="flex flex-col gap-1 p-2" data-testid="plan-panel">
      <div className="flex items-center justify-between whitespace-nowrap pb-1 text-[11px] font-mono text-text-muted">
        <span className="uppercase tracking-[0.08em]">Steps</span>
        <span data-testid="plan-panel-cas-rev" className="text-brass">
          Synced · v{ledgerRev}
        </span>
      </div>
      {mergedNodes.length > 0 && (
        <svg
          data-testid="plan-panel-progress-svg"
          viewBox="0 0 240 6"
          className="mb-1 h-1.5 w-full overflow-visible"
          aria-label="Step completion progress bar"
        >
          {mergedNodes.map((n, idx) => {
            const count = mergedNodes.length;
            const gap = count > 20 ? 1 : count > 1 ? 3 : 0;
            const segW = Math.max(2, (240 - gap * (count - 1)) / count);
            const x = idx * (segW + gap);
            const fill =
              n.status === 'completed'
                ? 'var(--color-status-pass)'
                : n.status === 'in_progress'
                  ? 'rgb(var(--brass))'
                  : n.status === 'blocked_on_approval'
                    ? 'var(--color-status-warn)'
                    : n.status === 'failed'
                      ? 'var(--color-status-fail)'
                      : 'rgba(255,255,255,0.14)';
            return <rect key={n.node_id} x={x} y="0.5" width={segW} height="5" rx="1.5" style={{ fill }} />;
          })}
        </svg>
      )}
      {mergedNodes.length === 0 ? (
        <p className="text-[11px] text-text-muted">Nothing to do yet.</p>
      ) : (
        mergedNodes.map(n => (
          <PlanNodeRow
            key={n.node_id}
            node={n}
            planSessionId={sessionId}
            planVersion={version}
            onToggleStatus={(id, nextStatus) => {
              setStatusOverrides(prev => ({ ...prev, [id]: nextStatus }));
              recordTaskLedgerOp({ kind: 'status', taskId: id, status: nextStatus, actor: 'human' });
              if (nextStatus === 'completed') {
                void hopperMarkDone(id).catch(() => {});
              }
            }}
            onRemoveNode={id => {
              setRemovedIds(prev => {
                const next = new Set(prev);
                next.add(id);
                return next;
              });
              recordTaskLedgerOp({ kind: 'remove', taskId: id, actor: 'human' });
              void hopperCancel(id).catch(() => {});
            }}
          />
        ))
      )}
      {blockedCount > 0 && (
        <div
          data-testid="plan-approval-footer"
          className="mt-1 flex items-center justify-between gap-2 rounded-sm border border-(--color-status-warn)/40 bg-(--color-status-warn)/10 px-2 py-1 text-[11px]"
        >
          <span className="text-(--color-status-warn)">
            {blockedCount} {blockedCount === 1 ? 'step' : 'steps'} awaiting approval
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => void approve()}
              disabled={approving}
              className="text-(--color-status-pass) hover:brightness-125 disabled:opacity-50"
            >
              Approve
            </button>
            <button
              type="button"
              onClick={() => onDiscard?.()}
              className="text-text-muted hover:text-(--color-status-fail)"
            >
              Discard
            </button>
          </div>
        </div>
      )}
      {adding ? (
        <input
          autoFocus
          className="mt-1 rounded-sm border border-border-subtle bg-transparent px-1 text-[12px] text-text-secondary"
          placeholder="new step…"
          value={newDescription}
          onChange={e => setNewDescription(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter') submitNew();
            if (e.key === 'Escape') setAdding(false);
          }}
          onBlur={submitNew}
        />
      ) : (
        <button
          type="button"
          onClick={() => setAdding(true)}
          className="mt-1 self-start text-[11px] text-text-muted hover:text-brass"
        >
          + Add step
        </button>
      )}
    </div>
  );
}
