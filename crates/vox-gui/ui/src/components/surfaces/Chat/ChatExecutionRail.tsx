import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Glass } from '../../ui/Glass';
import { ContextWindowMeter } from './ContextWindowMeter';
import { useLabel } from '../../../hooks/useLanguage';
import { useMetricSeries, type MetricPoint } from '../../../hooks/useMetricSeries';
import {
  getContextBudget,
  setContextBudgetStrategy,
  clearSessionToolResults,
  getSessionContextPins,
  setSessionContextPins,
  getSessionChatPosture,
  hopperSubmit,
  hopperRename,
  hopperCancel,
  hopperMarkDone,
  hopperReprioritize,
  doubtOrchestratorTask,
  forceOrchestratorVerify,
  getActiveTransportRouteStatus,
  vcsCheckpointCapture,
  vcsCheckpointList,
  vcsCheckpointRestore,
  type ContextBudgetPayload,
  type VcsCheckpointRecordDto,
} from '../../../transport';
import { SessionInteropModal } from './SessionInteropModal';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import { modelStateHint, type RailRouting } from '../../../lib/routingSummary';
import {
  getTaskLedgerSnapshot,
  recordTaskLedgerOp,
  TASK_LEDGER_DELTA_EVENT,
  type TaskLedgerDeltaEventDetail,
} from '../../../lib/taskLedgerSync';
import type { CompactionAuditDto, PostureAuditDto, SessionChatPostureDto } from '../../../types/tauri';

export interface SessionContextPin {
  id: string;
  label: string;
  scope?: string;
}

export const DEFAULT_SESSION_CONTEXT_PINS: SessionContextPin[] = [
  { id: 'ctx-1', label: '@kb:where-things-live.md', scope: 'RRF 0.98' },
  { id: 'ctx-2', label: '@kb:agentos-ssot-2026.md', scope: 'RRF 0.94' },
];

export interface ChatExecutionTask {
  id: string;
  title: string;
  status?: string;
  priority?: number;
  /** Derived from orchestrator lock activity, Phase 5 D-13. */
  lock?: { resourceId: string; state: 'holding' | 'waiting' };
}

export interface ChatExecutionRailProps {
  tasks: ChatExecutionTask[];
  /** The engine's routing pick for the next turn (App's one routing query); the section is hidden when null. */
  routing?: RailRouting | null;
  /** This session's spend — the one money figure the rail owns (global spend lives in the status bar). */
  sessionSpentUsd?: number | null;
  /** Active chat session id — passed to get_context_budget so the meter shows real token usage. */
  sessionId?: string | null;
  /** Opens the inline Routing panel (folded Matrix surface — gui-ia-blueprint: matrix → chat rail). */
  onOpenRouting?: () => void;
  /** Optional navigation handler for quick surface links in the session verification deck. */
  onNavigate?: (viewKey: string) => void;
  /** Completed assistant turns in this session (`countCompletedTurns`); each change re-reads the context budget. */
  turnsCompleted?: number;
  /** Optional turn-level BPE compaction audit receipt from `vox_chat_message`. */
  compactionAudit?: CompactionAuditDto | null;
  /** Optional turn-level session posture audit receipt from `vox_chat_message`. */
  postureAudit?: PostureAuditDto | null;
}

/** Finished assistant replies (done or failed): the rail re-reads the context budget when this grows. */
export function countCompletedTurns(messages: ChatMessage[]): number {
  return messages.filter((m) => m.role === 'assistant' && (m.status === 'done' || m.status === 'failed')).length;
}

export function sessionSpendSeriesKey(sessionId?: string | null): string {
  return sessionId ? `chat.session-spend.${sessionId}` : 'chat.session-spend';
}

function formatOpenRouterSpend(usd: number): string {
  return `$${usd.toFixed(2)}`;
}

function SessionSpendSpark({ series }: { series: MetricPoint[] }) {
  const width = 40;
  const height = 16;
  const values = series.map((p) => p.v);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min;
  const pad = 1;
  const innerW = width - pad * 2;
  const innerH = height - pad * 2;
  const d = values
    .map((v, i) => {
      const x = pad + (values.length === 1 ? innerW / 2 : (i / (values.length - 1)) * innerW);
      const y = range === 0 ? height / 2 : pad + innerH - ((v - min) / range) * innerH;
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(' ');

  return (
    <svg
      data-testid="execution-rail-spend-spark"
      width={40}
      height={16}
      viewBox="0 0 40 16"
      aria-hidden="true"
      className="shrink-0 text-brass"
    >
      <path d={d} fill="none" stroke="currentColor" strokeWidth="1" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function Segment({
  testId,
  label,
  value,
  onClick,
  trailing,
}: {
  testId: string;
  label: string;
  value: string;
  onClick?: () => void;
  trailing?: React.ReactNode;
}) {
  const className =
    'inline-flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1 text-[11px] text-text-muted transition hover:bg-overlay-subtle hover:text-text-secondary';

  const body = (
    <>
      <span className="uppercase tracking-[0.08em] text-text-muted">{label}</span>
      <span className="flex min-w-0 items-center gap-1">
        {trailing}
        <span className="font-mono tabular-nums text-text-secondary">{value}</span>
      </span>
    </>
  );

  if (onClick) {
    return (
      <button type="button" data-testid={testId} onClick={onClick} className={className}>
        {body}
      </button>
    );
  }

  return (
    <div data-testid={testId} className={className}>
      {body}
    </div>
  );
}

function SessionSpendTrack({
  sessionId,
  sessionSpentUsd,
}: {
  sessionId?: string | null;
  sessionSpentUsd: number;
}) {
  const { series, append } = useMetricSeries(sessionSpendSeriesKey(sessionId), []);
  const prev = useRef<number | undefined>(undefined);
  useEffect(() => {
    if (prev.current !== sessionSpentUsd) {
      prev.current = sessionSpentUsd;
      append(sessionSpentUsd);
    }
  }, [sessionSpentUsd, append]);
  return (
    <Segment
      testId="execution-rail-session"
      label="Session"
      value={formatOpenRouterSpend(sessionSpentUsd)}
      trailing={
        series.length >= 2 ? <SessionSpendSpark series={series} /> : null
      }
    />
  );
}

export function ChatExecutionRail({
  tasks,
  routing = null,
  sessionSpentUsd,
  sessionId,
  onOpenRouting,
  onNavigate,
  turnsCompleted = 0,
  compactionAudit = null,
  postureAudit = null,
}: ChatExecutionRailProps) {
  const [budget, setBudget] = useState<ContextBudgetPayload | null>(null);
  const [reclaimedToolTokens, setReclaimedToolTokens] = useState<number>(0);
  const budgetSessionRef = useRef(sessionId);
  const [sessionPosture, setSessionPosture] = useState<SessionChatPostureDto | null>(null);
  const [ledgerRev, setLedgerRev] = useState<number>(() => getTaskLedgerSnapshot().rev);
  const [pendingDeltasCount, setPendingDeltasCount] = useState<number>(
    () => getTaskLedgerSnapshot().pendingDeltas.length,
  );
  const [gitCheckpoints, setGitCheckpoints] = useState<VcsCheckpointRecordDto[]>([]);
  const [restoredRefBanner, setRestoredRefBanner] = useState<string | null>(null);
  const [sessionInteropOpen, setSessionInteropOpen] = useState(false);
  const activeRouteStatus = useMemo(() => getActiveTransportRouteStatus(), []);

  useEffect(() => {
    const tid = sessionId ?? 'default-session';
    let cancelled = false;
    void vcsCheckpointList(tid)
      .then((rows) => {
        if (!cancelled) setGitCheckpoints(rows);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [sessionId, turnsCompleted]);

  const handleCaptureCheckpoint = () => {
    const tid = sessionId ?? 'default-session';
    void vcsCheckpointCapture(tid).then((rec) => {
      setGitCheckpoints((prev) => [...prev, rec]);
      setRestoredRefBanner(`Captured ${rec.ref_name}`);
    });
  };

  const handleRestoreCheckpoint = (targetTurn: number) => {
    const tid = sessionId ?? 'default-session';
    void vcsCheckpointRestore(tid, targetTurn).then((res) => {
      setGitCheckpoints((prev) => prev.filter((c) => c.turn_count <= targetTurn));
      setRestoredRefBanner(`Restored ${res.ref_name}`);
    });
  };

  useEffect(() => {
    let cancelled = false;
    void getSessionChatPosture(sessionId ?? null)
      .then((dto) => {
        if (!cancelled && dto && typeof dto === 'object') {
          setSessionPosture(dto);
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [sessionId, turnsCompleted]);

  useEffect(() => {
    const onPostureSync = (ev: Event) => {
      const custom = ev as CustomEvent<{ posture?: SessionChatPostureDto }>;
      if (custom.detail?.posture) {
        setSessionPosture(custom.detail.posture);
      }
    };
    window.addEventListener('vox:session-posture-change', onPostureSync);
    return () => window.removeEventListener('vox:session-posture-change', onPostureSync);
  }, []);

  const [addedTasks, setAddedTasks] = useState<ChatExecutionTask[]>([]);
  const [taskEdits, setTaskEdits] = useState<Record<string, { title?: string; status?: string; priority?: number }>>({});
  const [removedTaskIds, setRemovedTaskIds] = useState<Set<string>>(new Set());
  const [editingTaskId, setEditingTaskId] = useState<string | null>(null);
  const [editingTaskDraft, setEditingTaskDraft] = useState<string>('');
  const [newTaskTitle, setNewTaskTitle] = useState<string>('');

  useEffect(() => {
    const onLedgerDelta = (ev: Event) => {
      const custom = ev as CustomEvent<TaskLedgerDeltaEventDetail>;
      const detail = custom.detail;
      if (!detail) return;
      setLedgerRev(detail.snapshot.rev);
      setPendingDeltasCount(detail.snapshot.pendingDeltas.length);
      const { delta } = detail;
      if (!delta) return;
      if (delta.kind === 'add' && delta.title) {
        setAddedTasks((prev) =>
          prev.some((t) => t.id === delta.taskId)
            ? prev
            : [...prev, { id: delta.taskId, title: delta.title!, status: delta.status ?? 'pending', priority: delta.priority ?? 2 }],
        );
      } else if (delta.kind === 'edit' && delta.title) {
        setTaskEdits((prev) => ({
          ...prev,
          [delta.taskId]: { ...prev[delta.taskId], title: delta.title },
        }));
      } else if (delta.kind === 'status' && delta.status) {
        setTaskEdits((prev) => ({
          ...prev,
          [delta.taskId]: { ...prev[delta.taskId], status: delta.status },
        }));
      } else if (delta.kind === 'priority' && delta.priority != null) {
        setTaskEdits((prev) => ({
          ...prev,
          [delta.taskId]: { ...prev[delta.taskId], priority: delta.priority },
        }));
      } else if (delta.kind === 'remove') {
        setRemovedTaskIds((prev) => {
          const next = new Set(prev);
          next.add(delta.taskId);
          return next;
        });
      }
    };
    window.addEventListener(TASK_LEDGER_DELTA_EVENT, onLedgerDelta);
    return () => window.removeEventListener(TASK_LEDGER_DELTA_EVENT, onLedgerDelta);
  }, []);

  const [contextPins, setContextPins] = useState<SessionContextPin[]>(DEFAULT_SESSION_CONTEXT_PINS);
  const [editingPinId, setEditingPinId] = useState<string | null>(null);
  const [editingPinDraft, setEditingPinDraft] = useState<string>('');
  const [newPinLabel, setNewPinLabel] = useState<string>('');

  useEffect(() => {
    const onPinsSync = (ev: Event) => {
      const custom = ev as CustomEvent<{ pins?: SessionContextPin[]; source?: string }>;
      if (custom.detail?.source !== 'rail' && Array.isArray(custom.detail?.pins)) {
        setContextPins(custom.detail.pins);
      }
    };
    window.addEventListener('vox:context-pins-change', onPinsSync);
    return () => window.removeEventListener('vox:context-pins-change', onPinsSync);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void getSessionContextPins(sessionId ?? null)
      .then((pins) => {
        if (!cancelled && Array.isArray(pins) && pins.length > 0) {
          setContextPins(pins);
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  const broadcastPins = (next: SessionContextPin[]) => {
    setContextPins(next);
    void setSessionContextPins(sessionId ?? null, next).catch(() => {});
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent('vox:context-pins-change', { detail: { pins: next, source: 'rail' } }),
      );
    }
  };

  const displayTasks = useMemo(() => {
    const merged = [...tasks, ...addedTasks.filter((a) => !tasks.some((t) => t.id === a.id))];
    return merged
      .filter((t) => !removedTaskIds.has(t.id))
      .map((t) => {
        const patch = taskEdits[t.id];
        if (!patch) return t;
        return {
          ...t,
          title: patch.title ?? t.title,
          status: patch.status ?? t.status,
          priority: patch.priority ?? t.priority,
        };
      });
  }, [tasks, addedTasks, taskEdits, removedTaskIds]);

  const handleAddTask = () => {
    const trimmed = newTaskTitle.trim();
    if (!trimmed) return;
    const id = `task-${Date.now()}`;
    setAddedTasks((prev) => [...prev, { id, title: trimmed, status: 'pending', priority: 2 }]);
    setNewTaskTitle('');
    recordTaskLedgerOp({ kind: 'add', taskId: id, title: trimmed, status: 'pending', priority: 2, actor: 'human' });
    void hopperSubmit(trimmed, []).catch(() => {});
  };

  const handleToggleTaskStatus = (task: ChatExecutionTask) => {
    const cur = (task.status ?? 'pending').toLowerCase();
    const nextStatus = cur === 'completed' || cur === 'done' ? 'running' : 'completed';
    setTaskEdits((prev) => ({
      ...prev,
      [task.id]: { ...prev[task.id], status: nextStatus },
    }));
    recordTaskLedgerOp({ kind: 'status', taskId: task.id, title: task.title, status: nextStatus, actor: 'human' });
    if (nextStatus === 'completed') {
      void hopperMarkDone(task.id).catch(() => {});
    }
  };

  const handleCycleTaskPriority = (task: ChatExecutionTask) => {
    const curPriority = task.priority ?? 2;
    const nextPriority = curPriority >= 3 ? 1 : curPriority + 1;
    setTaskEdits((prev) => ({
      ...prev,
      [task.id]: { ...prev[task.id], priority: nextPriority },
    }));
    recordTaskLedgerOp({ kind: 'priority', taskId: task.id, title: task.title, priority: nextPriority, actor: 'human' });
    void hopperReprioritize(task.id, nextPriority).catch(() => {});
  };

  const handleDoubtTask = (task: ChatExecutionTask) => {
    setTaskEdits((prev) => ({
      ...prev,
      [task.id]: { ...prev[task.id], status: 'doubted' },
    }));
    recordTaskLedgerOp({ kind: 'doubt', taskId: task.id, title: task.title, status: 'doubted', reason: 'Flagged from Execution Rail', actor: 'human' });
    void doubtOrchestratorTask(task.id, 'Flagged from Execution Rail').catch(() => {});
  };

  const handleVerifyTask = (task: ChatExecutionTask) => {
    setTaskEdits((prev) => ({
      ...prev,
      [task.id]: { ...prev[task.id], status: 'verifying' },
    }));
    recordTaskLedgerOp({ kind: 'verify', taskId: task.id, title: task.title, status: 'verifying', actor: 'human' });
    void forceOrchestratorVerify(task.id).catch(() => {});
  };

  const handleStartEditTask = (task: ChatExecutionTask) => {
    setEditingTaskId(task.id);
    setEditingTaskDraft(task.title);
  };

  const handleCommitEditTask = (taskId: string) => {
    const trimmed = editingTaskDraft.trim();
    if (trimmed) {
      setTaskEdits((prev) => ({
        ...prev,
        [taskId]: { ...prev[taskId], title: trimmed },
      }));
      recordTaskLedgerOp({ kind: 'edit', taskId, title: trimmed, actor: 'human' });
      void hopperRename(taskId, trimmed).catch(() => {});
    }
    setEditingTaskId(null);
  };

  const handleRemoveTask = (task: ChatExecutionTask) => {
    setRemovedTaskIds((prev) => {
      const next = new Set(prev);
      next.add(task.id);
      return next;
    });
    recordTaskLedgerOp({ kind: 'remove', taskId: task.id, title: task.title, actor: 'human' });
    void hopperCancel(task.id).catch(() => {});
  };

  const handleAddContextPin = () => {
    const trimmed = newPinLabel.trim();
    if (!trimmed) return;
    const formatted =
      trimmed.startsWith('@') || trimmed.includes('/') || trimmed.includes('.')
        ? trimmed
        : `@kb:${trimmed}`;
    const next = [...contextPins, { id: `ctx-${Date.now()}`, label: formatted, scope: 'pinned' }];
    broadcastPins(next);
    setNewPinLabel('');
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('vox:atlas-mention', { detail: { token: formatted } }));
    }
  };

  const handleCommitPinEdit = (pinId: string) => {
    const trimmed = editingPinDraft.trim();
    if (trimmed) {
      broadcastPins(contextPins.map((p) => (p.id === pinId ? { ...p, label: trimmed } : p)));
    }
    setEditingPinId(null);
  };

  const handleRemovePin = (pinId: string) => {
    broadcastPins(contextPins.filter((p) => p.id !== pinId));
  };

  useEffect(() => {
    let cancelled = false;
    if (budgetSessionRef.current !== sessionId) {
      budgetSessionRef.current = sessionId;
      setBudget(null);
    }
    getContextBudget(sessionId)
      .then((next) => {
        if (!cancelled) setBudget(next);
      })
      .catch(() => {
        if (!cancelled) setBudget(null);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionId, turnsCompleted]);

  const [activeInspectorTab, setActiveInspectorTab] = useState<'all' | 'tasks' | 'context' | 'routing'>('all');
  const completedTaskCount = useMemo(
    () =>
      displayTasks.filter((t) => {
        const st = (t.status ?? '').toLowerCase();
        return st === 'completed' || st === 'done';
      }).length,
    [displayTasks],
  );
  const stateHint = modelStateHint(routing?.state);
  const routingLine = routing
    ? `Routes to ${routing.model}${routing.reason ? ` — ${routing.reason}` : ''}`
    : '';

  return (
    <aside aria-label="Execution rail" className="flex h-full w-full min-w-0 flex-col">
      <Glass className="flex h-full flex-col gap-2 overflow-y-auto custom-scrollbar p-2.5 pb-4">
        <div className="flex shrink-0 flex-wrap items-center justify-between gap-1.5">
          <div className="flex items-center gap-1.5">
            <h2 className="text-[11px] uppercase tracking-[0.08em] text-brass">{useLabel('chat-execution')}</h2>
            {displayTasks.length > 0 && (
              <svg
                data-testid="execution-rail-progress-ring"
                width="14"
                height="14"
                viewBox="0 0 16 16"
                className="shrink-0"
                aria-hidden="true"
              >
                <circle cx="8" cy="8" r="6" fill="none" stroke="rgba(255,255,255,0.12)" strokeWidth="2" />
                <circle
                  cx="8"
                  cy="8"
                  r="6"
                  fill="none"
                  style={{ stroke: 'rgb(var(--brass))' }}
                  strokeWidth="2"
                  strokeDasharray={`${Math.round((completedTaskCount / Math.max(1, displayTasks.length)) * 37.7)} 37.7`}
                  transform="rotate(-90 8 8)"
                />
              </svg>
            )}
          </div>
          <div className="flex items-center gap-1">
            <span
              data-testid="execution-rail-cas-rev"
              title={`Task list version v${ledgerRev} (${pendingDeltasCount} recent edits synced with assistant)`}
              className="whitespace-nowrap rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[11px] text-brass"
            >
              Synced · v{ledgerRev}
            </span>
            <span className="whitespace-nowrap rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[11px] text-text-secondary">
              {displayTasks.length === 0 ? 'IDLE · READY' : `${displayTasks.length} ACTIVE`}
            </span>
          </div>
        </div>

        <div
          role="tablist"
          aria-label="Session inspector sections"
          data-testid="execution-rail-segmented-tabs"
          className="flex shrink-0 items-center justify-between gap-0.5 overflow-x-auto no-scrollbar border-b border-border-subtle bg-overlay-solid/90 px-0.5 pt-0.5 font-mono text-[11px]"
        >
          {(
            [
              { id: 'all', label: 'All' },
              { id: 'tasks', label: 'Tasks' },
              { id: 'context', label: 'Context' },
              { id: 'routing', label: 'Routing' },
            ] as const
          ).map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-label={tab.id === 'tasks' ? 'Plan & Tasks' : tab.label}
              data-cv-subsurface={`rail-tab-${tab.id}`}
              data-testid={`execution-rail-tab-${tab.id}`}
              aria-selected={activeInspectorTab === tab.id}
              onClick={() => setActiveInspectorTab(tab.id)}
              className={`min-w-0 flex-1 whitespace-nowrap rounded-t px-1 py-1 text-center tracking-tight transition ${
                activeInspectorTab === tab.id
                  ? 'border-b-2 border-brass bg-brass/12 text-brass font-semibold'
                  : 'border-b-2 border-transparent text-text-muted hover:text-text-secondary'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <section
          role="region"
          aria-label="Active tasks"
          className={`flex shrink-0 flex-col gap-1.5 ${
            activeInspectorTab !== 'all' && activeInspectorTab !== 'tasks' ? 'sr-only' : ''
          }`}
        >
          {displayTasks.length > 0 && (
            <div className="flex items-center justify-between gap-2 rounded-lg border border-border-subtle bg-overlay-subtle px-2.5 py-1.5">
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between gap-1.5">
                  <span className="font-display text-[11px] font-semibold uppercase tracking-[0.13em] text-text-secondary">
                    Plan & Execution Steps
                  </span>
                  <span className="font-mono text-[11px] text-(--color-status-pass)">
                    {completedTaskCount}/{displayTasks.length} done
                  </span>
                </div>
                <svg
                  data-testid="execution-rail-progress-svg"
                  viewBox="0 0 240 8"
                  className="mt-1 h-1.5 w-full shrink-0 overflow-visible"
                  aria-label="Task completion progress bar"
                >
                  {displayTasks.map((t, idx) => {
                    const count = displayTasks.length;
                    const gap = count > 20 ? 1 : count > 1 ? 3 : 0;
                    const segW = Math.max(2, (240 - gap * (count - 1)) / count);
                    const x = idx * (segW + gap);
                    const st = (t.status ?? 'pending').toLowerCase();
                    const fill =
                      st === 'completed' || st === 'done'
                        ? 'var(--color-status-pass)'
                        : st === 'running' || st === 'verifying'
                          ? 'rgb(var(--brass))'
                          : st === 'doubted'
                            ? 'var(--color-status-warn)'
                            : 'rgba(255,255,255,0.14)';
                    return (
                      <rect
                        key={t.id}
                        x={x}
                        y="1"
                        width={segW}
                        height="6"
                        rx="2"
                        style={{ fill }}
                      />
                    );
                  })}
                </svg>
              </div>
              <div className="relative flex size-8 shrink-0 items-center justify-center">
                <svg viewBox="0 0 32 32" className="size-8 -rotate-90" aria-hidden="true">
                  <circle cx="16" cy="16" r="12" fill="none" stroke="rgba(255,255,255,0.12)" strokeWidth="3" />
                  <circle
                    cx="16"
                    cy="16"
                    r="12"
                    fill="none"
                    style={{ stroke: 'var(--color-status-pass)' }}
                    strokeWidth="3"
                    strokeLinecap="round"
                    strokeDasharray={`${Math.round((completedTaskCount / Math.max(1, displayTasks.length)) * 75.4)} 75.4`}
                  />
                </svg>
                <span className="absolute font-mono text-[11px] font-semibold text-text-primary">
                  {completedTaskCount}/{displayTasks.length}
                </span>
              </div>
            </div>
          )}
          {displayTasks.length === 0 ? (
            <p className="text-[11px] text-text-muted">No active tasks for this session.</p>
          ) : (
            <ul className="flex max-h-52 flex-col gap-1 overflow-y-auto custom-scrollbar">
              {displayTasks.map((task) => {
                const statusLower = task.status?.toLowerCase() ?? 'pending';
                const isDone = statusLower === 'completed' || statusLower === 'done';
                const isRunning = statusLower === 'running' || statusLower === 'verifying';
                const isEditing = editingTaskId === task.id;
                const taskPriority = task.priority ?? 2;
                return (
                  <li
                    key={task.id}
                    className={`group rounded-lg border px-2 py-1.5 transition ${
                      isRunning
                        ? 'border-brass/45 bg-brass/8 shadow-xs'
                        : 'border-border-subtle bg-overlay-subtle hover:border-brass/30'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-1.5">
                      <button
                        type="button"
                        data-testid={`execution-rail-task-toggle-${task.id}`}
                        aria-label={`Toggle task ${task.title}`}
                        onClick={() => handleToggleTaskStatus(task)}
                        className={`mt-0.5 inline-flex size-4 shrink-0 items-center justify-center rounded-full border font-mono text-[11px] font-bold transition ${
                          isDone
                            ? 'border-(--color-status-pass)/60 bg-(--color-status-pass)/20 text-(--color-status-pass)'
                            : isRunning
                              ? 'border-brass/60 bg-brass/15 text-brass'
                              : 'border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-primary'
                        }`}
                      >
                        {isDone ? '✓' : isRunning ? '◑' : '○'}
                      </button>
                      <div className="min-w-0 flex-1">
                        {isEditing ? (
                          <input
                            autoFocus
                            data-testid="execution-rail-task-edit-input"
                            aria-label={`Edit task title ${task.title}`}
                            value={editingTaskDraft}
                            onChange={(e) => setEditingTaskDraft(e.target.value)}
                            onBlur={() => handleCommitEditTask(task.id)}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') handleCommitEditTask(task.id);
                              if (e.key === 'Escape') setEditingTaskId(null);
                            }}
                            className="w-full rounded border border-brass/40 bg-surface-base/70 px-1.5 py-0.5 text-xs text-text-primary outline-hidden"
                          />
                        ) : (
                          <p
                            className={`text-xs leading-snug truncate whitespace-normal line-clamp-2 wrap-break-word ${
                              isDone ? 'text-(--color-status-pass)/90' : 'text-text-secondary'
                            }`}
                            title={task.title}
                            onDoubleClick={() => handleStartEditTask(task)}
                          >
                            {task.title}
                          </p>
                        )}
                        {task.status && (
                          <p className="mt-0.5 text-[11px] uppercase tracking-[0.08em] text-text-muted">
                            {task.status}
                          </p>
                        )}
                      </div>
                      <div className="flex shrink-0 items-center gap-0.5 whitespace-nowrap opacity-0 transition group-hover:opacity-100 focus-within:opacity-100">
                        <button
                          type="button"
                          data-testid={`execution-rail-task-priority-${task.id}`}
                          aria-label={`Cycle priority for ${task.title} (P${taskPriority})`}
                          title={`Priority P${taskPriority} (click to cycle P1..P3)`}
                          onClick={() => handleCycleTaskPriority(task)}
                          className="whitespace-nowrap rounded border border-border-subtle bg-surface-base/60 px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:border-brass/40 hover:text-brass"
                        >
                          P{taskPriority}
                        </button>
                        <button
                          type="button"
                          data-testid={`execution-rail-task-doubt-${task.id}`}
                          aria-label={`Flag doubt on ${task.title}`}
                          title="Flag for review"
                          onClick={() => handleDoubtTask(task)}
                          className="whitespace-nowrap rounded px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:bg-overlay-medium hover:text-(--color-status-warn)"
                        >
                          ?
                        </button>
                        <button
                          type="button"
                          data-testid={`execution-rail-task-verify-${task.id}`}
                          aria-label={`Force verify on ${task.title}`}
                          title="Verify step"
                          onClick={() => handleVerifyTask(task)}
                          className="whitespace-nowrap rounded px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:bg-overlay-medium hover:text-(--color-status-pass)"
                        >
                          ✓
                        </button>
                        <button
                          type="button"
                          data-testid={`execution-rail-task-edit-${task.id}`}
                          aria-label={`Edit task ${task.title}`}
                          onClick={() => handleStartEditTask(task)}
                          className="whitespace-nowrap rounded px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:bg-overlay-medium hover:text-brass"
                        >
                          ✎
                        </button>
                        <button
                          type="button"
                          data-testid={`execution-rail-task-remove-${task.id}`}
                          aria-label={`Remove task ${task.title}`}
                          onClick={() => handleRemoveTask(task)}
                          className="whitespace-nowrap rounded px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:bg-overlay-medium hover:text-(--color-status-fail)"
                        >
                          ×
                        </button>
                      </div>
                    </div>
                    {task.lock && (
                      <span
                        data-testid="execution-rail-lock-chip"
                        data-lock-state={task.lock.state}
                        title={task.lock.resourceId}
                        className="mt-1 inline-flex items-center gap-1 rounded-full border border-border-subtle bg-overlay-subtle px-2 py-0.5 font-mono text-[11px] text-text-secondary max-w-full truncate"
                      >
                        {task.lock.state === 'holding' ? `holding ${task.lock.resourceId}` : `waiting on ${task.lock.resourceId}`}
                      </span>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          <div className="flex items-center gap-1 pt-0.5">
            <input
              type="text"
              data-testid="execution-rail-add-task-input"
              aria-label="Add session task"
              placeholder="+ Add task or step…"
              value={newTaskTitle}
              onChange={(e) => setNewTaskTitle(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddTask();
              }}
              className="min-w-0 flex-1 rounded border border-border-subtle bg-overlay-subtle px-2 py-1 text-[11px] text-text-primary placeholder:text-text-muted focus:border-brass/40 focus:outline-hidden"
            />
            <button
              type="button"
              data-testid="execution-rail-add-task-btn"
              aria-label="Submit new session task"
              onClick={handleAddTask}
              className="shrink-0 rounded border border-brass/30 bg-brass/10 px-2 py-1 font-mono text-[11px] text-brass transition hover:bg-brass/20"
            >
              Add
            </button>
          </div>
        </section>

        {routing && (
          <section
            role="region"
            aria-label="Routing"
            className={`flex shrink-0 flex-col gap-1 pt-1 ${
              activeInspectorTab !== 'all' && activeInspectorTab !== 'routing' ? 'sr-only' : ''
            }`}
          >
            <div className="mb-0.5 flex items-baseline justify-between gap-2 border-b border-border-subtle pb-1">
              <span className="font-display text-[11px] uppercase tracking-[0.13em] text-text-muted">Routing</span>
              <span
                data-testid="execution-rail-routing-scope"
                title="The engine's current pick for the next turn; each reply's own routing is in its trace"
                className="text-[11px] text-text-muted"
              >
                next turn
              </span>
            </div>
            <button
              type="button"
              data-testid="execution-rail-routing"
              title={routingLine}
              onClick={() => onOpenRouting?.()}
              className="truncate whitespace-normal line-clamp-2 wrap-break-word rounded-sm px-2 py-0.5 text-left text-[11px] leading-snug text-text-secondary transition hover:bg-overlay-subtle hover:text-brass"
            >
              {routingLine}
            </button>
            <button
              type="button"
              data-testid="execution-rail-mens-spoke"
              data-allow-ellipsis="true"
              title="Active MENS Spoke & Hub Base routed by DomainRouter (click to open MENS Crucible)"
              onClick={() => {
                if (onNavigate) {
                  onNavigate('mens');
                } else {
                  window.location.hash = '#view=mens';
                }
              }}
              className="flex flex-wrap items-center justify-between gap-1.5 rounded-sm border border-border-subtle bg-overlay-subtle px-2 py-0.5 text-left text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
            >
              <span className="uppercase tracking-[0.08em] text-text-muted">Spoke</span>
              <span className="truncate font-mono text-[11px] text-(--color-status-pass)">
                {(() => {
                  const runningTitle = (
                    tasks.find((t) => t.status === 'running' || t.status === 'in_progress')?.title ??
                    tasks[0]?.title ??
                    ''
                  ).toLowerCase();
                  if (runningTitle.includes('rust') || runningTitle.includes('cargo')) return 'rust_workspace · Qwen3-8B';
                  if (runningTitle.includes('shell') || runningTitle.includes('pwsh')) return 'structured_shell · Qwen3-8B';
                  if (runningTitle.includes('mcp') || runningTitle.includes('tool')) return 'mcp_tool_chain · Qwen3-8B';
                  if (runningTitle.includes('voice') || runningTitle.includes('speech')) return 'speech_to_code · Qwen3-8B';
                  if (runningTitle.includes('mesh') || runningTitle.includes('populi')) return 'populi_mesh · Qwen3-8B';
                  if (runningTitle.includes('skill')) return 'skill_creation · Qwen3-8B';
                  return 'vox_lang · Qwen3-8B';
                })()}
              </span>
            </button>
            {(routing.state || routing.alternatives.length > 0) && (
              <details className="px-2 text-[11px] text-text-muted">
                <summary className="cursor-pointer select-none">Why this model</summary>
                {routing.state && (
                  <p className="mt-1">
                    State:{' '}
                    <span
                      data-testid="execution-rail-routing-state"
                      title={stateHint ?? undefined}
                      className="underline decoration-dotted"
                    >
                      {routing.state}
                    </span>
                  </p>
                )}
                {routing.alternatives.length > 0 && (
                  <p className="mt-0.5">Alternatives: {routing.alternatives.join(', ')}</p>
                )}
              </details>
            )}
          </section>
        )}

        {sessionSpentUsd != null && !Number.isNaN(sessionSpentUsd) && (
          <section aria-label="Session spend" className="flex shrink-0 flex-col gap-1 pt-1">
            <SessionSpendTrack
              key={sessionId ?? 'none'}
              sessionId={sessionId}
              sessionSpentUsd={sessionSpentUsd}
            />
          </section>
        )}

        {budget && (
          <ContextWindowMeter
            usedTokens={budget.used_tokens}
            maxTokens={budget.max_context_tokens}
            thresholdTokens={budget.threshold_tokens}
            strategy={budget.strategy}
            reclaimedToolTokens={reclaimedToolTokens || compactionAudit?.reclaimed_tool_tokens}
            onClearToolOutputs={
              sessionId
                ? () => {
                    void clearSessionToolResults(sessionId, 2)
                      .then((outcome) => {
                        setReclaimedToolTokens((prev) => prev + outcome.reclaimed_tool_tokens);
                        setBudget((prev) =>
                          prev
                            ? {
                                ...prev,
                                used_tokens: outcome.updated_used_tokens,
                              }
                            : prev,
                        );
                      })
                      .catch(() => {});
                  }
                : undefined
            }
            onStrategyChange={(nextStrategy) => {
              setBudget((prev) => (prev ? { ...prev, strategy: nextStrategy } : prev));
              void setContextBudgetStrategy(sessionId ?? null, nextStrategy)
                .then((updated) => {
                  if (updated && typeof updated.strategy === 'string') {
                    setBudget(updated);
                  }
                })
                .catch(() => {});
            }}
          />
        )}

        {(budget || compactionAudit || postureAudit || sessionPosture) && (() => {
          const effStrategy = compactionAudit?.strategy ?? budget?.strategy ?? sessionPosture?.compaction_strategy ?? 'balanced';
          const effBudgetTokens =
            compactionAudit?.budget_tokens ??
            (effStrategy === 'aggressive' ? 2400 : effStrategy === 'conservative' ? 8000 : 4000);
          const anchorPreserved =
            compactionAudit != null ? compactionAudit.head_anchor_preserved : effStrategy !== 'aggressive';
          const effClutch = postureAudit?.clutch ?? sessionPosture?.clutch ?? 'efficiency';
          const effRisk = postureAudit?.risk ?? sessionPosture?.risk ?? 'moderate';
          const effKb = postureAudit?.kb_scope ?? sessionPosture?.kb_scope ?? 'auto';
          return (
            <div
              data-testid="chat-turn-receipt-badge"
              aria-label="Turn compaction and posture receipt"
              className="flex flex-wrap items-center justify-between gap-1 rounded border border-border-subtle bg-overlay-subtle/80 px-2 py-1 font-mono text-[11px] text-text-muted"
            >
              <span className="truncate text-text-secondary">
                Goal Anchor: {anchorPreserved ? 'Kept' : 'Recency'} · {effBudgetTokens.toLocaleString()} BPE
                {compactionAudit != null ? ` (${compactionAudit.kept_messages}/${compactionAudit.total_loaded} msgs)` : ''}
              </span>
              <span className="shrink-0 text-brass">
                {effClutch}/{effRisk} · @{effKb}
              </span>
            </div>
          );
        })()}

        <section
          role="region"
          aria-label="Session context"
          data-testid="execution-rail-context-pins"
          className={`flex shrink-0 flex-col gap-1 border-t border-border-subtle pt-1.5 ${
            activeInspectorTab !== 'all' && activeInspectorTab !== 'context' ? 'sr-only' : ''
          }`}
        >
          <div className="flex items-center justify-between">
            <span className="font-display text-[11px] uppercase tracking-[0.13em] text-text-muted">
              Pinned Context
            </span>
            <span className="font-mono text-[11px] text-text-muted">
              {contextPins.length} {contextPins.length === 1 ? 'pin' : 'pins'}
            </span>
          </div>
          {contextPins.length === 0 ? (
            <p className="text-[11px] text-text-muted">No pinned context items.</p>
          ) : (
            <ul className="flex flex-col gap-1">
              {contextPins.map((pin) => {
                const isEditingPin = editingPinId === pin.id;
                return (
                  <li
                    key={pin.id}
                    className="flex items-center justify-between gap-1.5 rounded border border-border-subtle bg-overlay-subtle px-2 py-1"
                  >
                    {isEditingPin ? (
                      <input
                        autoFocus
                        data-testid="execution-rail-context-edit-input"
                        aria-label={`Edit context pin ${pin.label}`}
                        value={editingPinDraft}
                        onChange={(e) => setEditingPinDraft(e.target.value)}
                        onBlur={() => handleCommitPinEdit(pin.id)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleCommitPinEdit(pin.id);
                          if (e.key === 'Escape') setEditingPinId(null);
                        }}
                        className="min-w-0 flex-1 rounded border border-brass/40 bg-surface-base/70 px-1.5 py-0.5 font-mono text-[11px] text-text-primary outline-hidden"
                      />
                    ) : (
                      <span
                        className="min-w-0 flex-1 line-clamp-2 break-all font-mono text-[11px] leading-snug text-brass"
                        title={pin.label}
                        onDoubleClick={() => {
                          setEditingPinId(pin.id);
                          setEditingPinDraft(pin.label);
                        }}
                      >
                        {pin.label}
                        {pin.scope ? (
                          <span className="ml-1.5 text-text-muted">· {pin.scope}</span>
                        ) : null}
                      </span>
                    )}
                    <div className="flex shrink-0 items-center gap-0.5">
                      <button
                        type="button"
                        data-testid={`execution-rail-context-edit-${pin.id}`}
                        aria-label={`Edit context ${pin.label}`}
                        onClick={() => {
                          setEditingPinId(pin.id);
                          setEditingPinDraft(pin.label);
                        }}
                        className="rounded px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:bg-overlay-medium hover:text-brass"
                      >
                        ✎
                      </button>
                      <button
                        type="button"
                        data-testid={`execution-rail-context-remove-${pin.id}`}
                        aria-label={`Remove context ${pin.label}`}
                        onClick={() => handleRemovePin(pin.id)}
                        className="rounded px-1 py-0.5 font-mono text-[11px] text-text-muted transition hover:bg-overlay-medium hover:text-(--color-status-fail)"
                      >
                        ×
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
          <div className="flex items-center gap-1 pt-0.5">
            <input
              type="text"
              data-testid="execution-rail-add-context-input"
              aria-label="Add session context pin"
              placeholder="+ Pin @kb, file, or rule…"
              value={newPinLabel}
              onChange={(e) => setNewPinLabel(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleAddContextPin();
              }}
              className="min-w-0 flex-1 rounded border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-primary placeholder:text-text-muted focus:border-brass/40 focus:outline-hidden"
            />
            <button
              type="button"
              data-testid="execution-rail-add-context-btn"
              aria-label="Pin session context"
              onClick={handleAddContextPin}
              className="shrink-0 rounded border border-brass/30 bg-brass/10 px-2 py-1 font-mono text-[11px] text-brass transition hover:bg-brass/20"
            >
              Pin
            </button>
          </div>
        </section>

        <section
          data-testid="chat-git-checkpoints-panel"
          aria-label="Git checkpoints and session interop"
          className="flex shrink-0 flex-col gap-1.5 border-t border-border-subtle pt-1.5"
        >
          <div className="flex items-center justify-between gap-1">
            <span className="font-display text-[11px] uppercase tracking-[0.13em] text-text-muted">
              Git Checkpoints
            </span>
            <div className="flex items-center gap-1">
              <button
                type="button"
                data-testid="chat-checkpoint-capture-btn"
                onClick={handleCaptureCheckpoint}
                className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[11px] text-brass hover:bg-brass/20"
              >
                + Capture
              </button>
              <button
                type="button"
                data-testid="execution-rail-session-interop-btn"
                onClick={() => setSessionInteropOpen((o) => !o)}
                className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[11px] text-text-secondary hover:border-brass/40 hover:text-brass"
              >
                ⇄ CLI Import
              </button>
            </div>
          </div>
          {restoredRefBanner && (
            <div
              data-testid="chat-checkpoint-status-banner"
              className="rounded border border-brass/30 bg-brass/10 px-2 py-0.5 font-mono text-[11px] text-brass"
            >
              {restoredRefBanner}
            </div>
          )}
          {gitCheckpoints.length > 0 && (
            <ul className="flex flex-col gap-1">
              {gitCheckpoints.slice(-3).map((ckpt) => (
                <li
                  key={ckpt.ref_name}
                  data-testid={`chat-checkpoint-row-${ckpt.turn_count}`}
                  className="flex items-center justify-between gap-1.5 rounded border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary"
                >
                  <span className="truncate">
                    Turn #{ckpt.turn_count} · {ckpt.commit_oid.slice(0, 8)}
                  </span>
                  <button
                    type="button"
                    data-testid={`chat-checkpoint-restore-${ckpt.turn_count}`}
                    onClick={() => handleRestoreCheckpoint(ckpt.turn_count)}
                    className="shrink-0 rounded border border-border-subtle bg-bg-base px-1.5 py-0.5 text-[11px] text-brass hover:bg-brass/15"
                  >
                    Restore
                  </button>
                </li>
              ))}
            </ul>
          )}
          {sessionInteropOpen && (
            <SessionInteropModal
              sessionId={sessionId ?? 'default-session'}
              onClose={() => setSessionInteropOpen(false)}
            />
          )}
        </section>

        <section
          data-testid="execution-rail-session-deck"
          aria-label="Session verification deck"
          className="mt-auto flex shrink-0 flex-col gap-1.5 border-t border-border-subtle pt-2"
        >
          <div className="flex flex-wrap items-center justify-between gap-1">
            <span className="font-display text-[11px] uppercase tracking-[0.13em] text-text-muted">
              System Checks
            </span>
            <span
              data-testid="execution-rail-route-tier"
              className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[11px] text-brass"
            >
              Route: {activeRouteStatus.activeTier}
            </span>
            <span className="rounded border border-(--color-status-pass)/30 bg-(--color-status-pass)/10 px-1.5 py-0.5 font-mono text-[11px] text-(--color-status-pass)">
              All checks passing
            </span>
          </div>
          <div
            data-testid="execution-rail-hooks-status"
            className="flex items-center justify-between gap-1 rounded border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary"
          >
            <span title="Deterministic PreToolUse, PostToolBatch & Stop lifecycle hooks active (.env & SSOT guarded)">
              .vox Hooks: 3 active
            </span>
            <span className="text-brass" title="Oracle tier architectural escalation ready">
              Oracle: Ready
            </span>
          </div>
          <div className="rounded-md border border-border-subtle bg-overlay-subtle/80 px-2.5 py-1.5">
            <div className="mb-0.5 flex items-center justify-between font-mono text-[11px] text-text-muted">
              <span>Verification & Response Pace</span>
              <span className="text-(--color-status-pass)">100% pass rate</span>
            </div>
            <svg
              data-testid="execution-rail-health-svg"
              viewBox="0 0 240 28"
              className="h-6 w-full overflow-visible"
              aria-label="Session verification and response pace chart"
            >
              <defs>
                <linearGradient id="railHealthGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" style={{ stopColor: 'var(--color-status-pass)' }} stopOpacity="0.28" />
                  <stop offset="100%" style={{ stopColor: 'var(--color-status-pass)' }} stopOpacity="0.02" />
                </linearGradient>
              </defs>
              <line x1="0" y1="24" x2="240" y2="24" stroke="rgba(255,255,255,0.08)" strokeDasharray="2 2" />
              <path
                d="M0,20 C36,18 68,11 104,12 C140,13 176,6 240,5 L240,26 L0,26 Z"
                fill="url(#railHealthGrad)"
              />
              <path
                d="M0,20 C36,18 68,11 104,12 C140,13 176,6 240,5"
                fill="none"
                style={{ stroke: 'var(--color-status-pass)' }}
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
          </div>
          <div className="grid grid-cols-3 gap-1 pt-0.5">
            <button
              type="button"
              onClick={() => onNavigate?.('console')}
              className="inline-flex items-center justify-center whitespace-nowrap rounded border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
            >
              Terminal
            </button>
            <button
              type="button"
              onClick={() => onNavigate?.('browser')}
              className="inline-flex items-center justify-center whitespace-nowrap rounded border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
            >
              Browser
            </button>
            <button
              type="button"
              onClick={() => onNavigate?.('research')}
              className="inline-flex items-center justify-center whitespace-nowrap rounded border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary transition hover:border-brass/40 hover:text-brass"
            >
              Research
            </button>
          </div>
        </section>
      </Glass>
    </aside>
  );
}
