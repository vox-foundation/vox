import React, { useEffect, useState, useCallback } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { Glass } from '../../ui/Glass';
import { EmptyState } from '../../ui/EmptyState';
import { Icon } from '../../ui/Icons';
import { VoxSelect } from '../../ui/Select';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import {
  activityQuery,
  listenActivityAppended,
  type ActivityRowDto as ActivityRow,
  type ActivityFilterDto as ActivityFilter,
} from '../../../transport';
import { ACTIVITY_REFRESH_DEBOUNCE_MS } from '../../../config/constants';
import type { Toast } from '../../../types/tauri';

export type { ActivityRow, ActivityFilter };

export interface ActivityTimelineProps {
  rows: ActivityRow[];
}

type TimelineItem =
  | { type: 'single'; row: ActivityRow }
  | { type: 'folded'; rows: ActivityRow[]; key: string; totalCost: number; agentId: string };

function getCostUsd(row: ActivityRow): number {
  try {
    const parsed = JSON.parse(row.detail_json);
    if (parsed && typeof parsed === 'object') {
      if ('CostIncurred' in parsed && parsed.CostIncurred && typeof parsed.CostIncurred === 'object' && 'cost_usd' in parsed.CostIncurred) {
        return Number(parsed.CostIncurred.cost_usd) || 0;
      }
      if ('cost_usd' in parsed) {
        return Number(parsed.cost_usd) || 0;
      }
    }
  } catch (e) {
    // Fall back to summary parsing
  }
  const match = row.summary.match(/Cost incurred: \$([0-9.]+)/);
  if (match) {
    return parseFloat(match[1]) || 0;
  }
  return 0;
}

function foldCostRuns(rows: ActivityRow[]): TimelineItem[] {
  const result: TimelineItem[] = [];
  let i = 0;
  while (i < rows.length) {
    const current = rows[i];
    if (current.kind === 'CostIncurred' && current.agent_id) {
      const run: ActivityRow[] = [current];
      let j = i + 1;
      while (j < rows.length) {
        const next = rows[j];
        if (next.kind === 'CostIncurred' && next.agent_id === current.agent_id) {
          run.push(next);
          j++;
        } else {
          break;
        }
      }
      if (run.length >= 3) {
        let totalCost = 0;
        for (const r of run) {
          totalCost += getCostUsd(r);
        }
        result.push({
          type: 'folded',
          rows: run,
          key: `folded-${current.id}-${j}`,
          totalCost,
          agentId: current.agent_id,
        });
        i = j;
      } else {
        for (const r of run) {
          result.push({ type: 'single', row: r });
        }
        i = j;
      }
    } else {
      result.push({ type: 'single', row: current });
      i++;
    }
  }
  return result;
}

export function ActivityTimeline({ rows }: ActivityTimelineProps) {
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});

  const toggleExpand = (key: string) => {
    setExpandedKeys((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const getKindColorClass = (kind: string) => {
    switch (kind) {
      case 'AgentSpawned':
      case 'TaskStarted':
      case 'TaskCompleted':
      case 'WorkflowStarted':
      case 'WorkflowCompleted':
        return 'border-border-strong bg-bg-elevated text-(--color-status-pass)';
      case 'TaskFailed':
      case 'WorkflowFailed':
        return 'border-border-strong bg-bg-elevated text-(--color-status-fail)';
      case 'BudgetAlert':
      case 'AttentionBudgetAlert':
      case 'ConflictDetected':
        return 'border-border-strong bg-bg-elevated text-(--color-status-warn)';
      case 'CostIncurred':
        return 'border-border-strong bg-bg-elevated text-text-primary';
      default:
        return 'border-border-strong bg-bg-surface text-text-primary';
    }
  };

  const getIcon = (kind: string) => {
    switch (kind) {
      case 'AgentSpawned':
      case 'TaskStarted':
      case 'WorkflowStarted':
        return <Icon.bolt className="size-4" />;
      case 'TaskCompleted':
      case 'WorkflowCompleted':
        return <Icon.check className="size-4" />;
      case 'TaskFailed':
      case 'WorkflowFailed':
        return <Icon.x className="size-4" />;
      case 'BudgetAlert':
      case 'AttentionBudgetAlert':
      case 'ConflictDetected':
        return <Icon.alert className="size-4" />;
      case 'CostIncurred':
        return <Icon.cpu className="size-4" />;
      default:
        return <Icon.alert className="size-4" />;
    }
  };

  const items = foldCostRuns(rows);

  return (
    <div className="flex flex-col gap-3">
      {items.map((item) => {
        if (item.type === 'single') {
          const row = item.row;
          return (
            <div
              key={row.id}
              data-testid="activity-row"
              className={`vox-marble-tablet flex items-start gap-4 p-3 rounded-lg border text-xs leading-relaxed transition-colors ${getKindColorClass(
                row.kind
              )}`}
            >
              <div className="flex items-center justify-center p-1.5 rounded-sm bg-bg-surface border border-border-strong">
                {getIcon(row.kind)}
              </div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1 flex-wrap">
                  <span className="font-semibold uppercase tracking-wider text-[11px]">
                    {row.kind}
                  </span>
                  <span className="text-text-secondary">
                    {new Date(row.ts_ms).toLocaleTimeString()}
                  </span>
                  {row.agent_id && (
                    <span className="px-2 py-0.5 rounded-sm bg-bg-surface border border-border-strong text-[11px] font-medium text-text-primary">
                      Agent: {row.agent_id}
                    </span>
                  )}
                </div>
                <p className="text-text-primary font-semibold">{row.summary}</p>
              </div>
            </div>
          );
        } else {
          const isExpanded = !!expandedKeys[item.key];
          return (
            <div
              key={item.key}
              data-testid="activity-row"
              className={`vox-marble-tablet flex flex-col gap-2 p-3 rounded-lg border text-xs leading-relaxed transition-colors ${getKindColorClass(
                'CostIncurred'
              )}`}
            >
              <div className="flex items-start gap-4">
                <div className="flex items-center justify-center p-1.5 rounded-sm bg-bg-surface border border-border-strong">
                  {getIcon('CostIncurred')}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1 flex-wrap">
                    <span className="font-semibold uppercase tracking-wider text-[11px]">
                      CostIncurred (Folded)
                    </span>
                    <span className="text-text-secondary">
                      {new Date(item.rows[0].ts_ms).toLocaleTimeString()}
                    </span>
                    {item.agentId && (
                      <span className="px-2 py-0.5 rounded-sm bg-bg-surface border border-border-strong text-[11px] font-medium text-text-primary">
                        Agent: {item.agentId}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center justify-between gap-4">
                    <p className="text-text-primary font-semibold">
                      spent ${item.totalCost.toFixed(4)} ({item.rows.length} calls)
                    </p>
                    <button
                      type="button"
                      data-slot="button"
                      onClick={() => toggleExpand(item.key)}
                      className="h-7 px-2.5 rounded border border-border-strong bg-bg-surface hover:bg-bg-elevated text-[11px] text-text-primary font-semibold transition-colors"
                      data-testid="fold-toggle"
                    >
                      {isExpanded ? 'Hide' : 'Expand'}
                    </button>
                  </div>
                </div>
              </div>
              {isExpanded && (
                <div className="pl-12 pr-2 py-1 flex flex-col gap-1.5 border-t border-border-subtle mt-1">
                  {item.rows.map((row) => (
                    <div
                      key={row.id}
                      className="flex items-center justify-between text-[11px] text-text-secondary hover:text-text-primary py-0.5"
                    >
                      <span className="truncate">{row.summary}</span>
                      <span className="text-text-muted ml-2 whitespace-nowrap">
                        {new Date(row.ts_ms).toLocaleTimeString()}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        }
      })}
    </div>
  );
}

export interface ActivitySurfaceProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}

export function ActivitySurface({ pushToast }: ActivitySurfaceProps) {
  const embedded = useIsEmbeddedSurface();
  const [rows, setRows] = useState<ActivityRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [agentFilter, setAgentFilter] = useState<string>('');
  const [kindFilter, setKindFilter] = useState<string>('');

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    try {
      const filter = {
        agent_id: agentFilter === '' ? null : agentFilter,
        kind: kindFilter === '' ? null : kindFilter,
        limit: 50,
        before_id: null,
      };
      const res = await activityQuery(filter);
      // activityQuery is typed ActivityRowDto[], but a backend/IPC (or the visual-audit
      // mock) can yield null/undefined; never let a non-array reach state (rows.map()).
      setRows(Array.isArray(res) ? res : []);
    } catch (err) {
      pushToast({
        tone: 'warn',
        title: 'Query Failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setLoading(false);
    }
  }, [agentFilter, kindFilter, pushToast]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  useEffect(() => {
    const onVisibility = () => {
      if (document.visibilityState === 'visible') {
        void fetchLogs();
      }
    };
    document.addEventListener('visibilitychange', onVisibility);
    return () => document.removeEventListener('visibilitychange', onVisibility);
  }, [fetchLogs]);

  // Reactive updates on "vox://activity-appended", which the GUI bridge emits for every activity-loggable
  // event and replay frame (crates/vox-gui/src/commands/event_annotate.rs).
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    // listen() rejects when the Tauri event bridge is unavailable (bare
    // browser, tests, headless capture) — guard so nothing leaks an
    // unhandled rejection and cleanup still resolves.
    const unlistenPromise = listenActivityAppended(() => {
      clearTimeout(timer);
      timer = setTimeout(fetchLogs, ACTIVITY_REFRESH_DEBOUNCE_MS);
    }).catch(() => undefined);
    return () => {
      clearTimeout(timer);
      unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, [fetchLogs]);

  // Extract unique agents and kinds from current rows to populate filter lists dynamically
  const uniqueAgents = Array.from(
    new Set(rows.map((r) => r.agent_id).filter((id): id is string => !!id))
  ).sort();

  const uniqueKinds = Array.from(new Set(rows.map((r) => r.kind))).sort();

  return (
    <div className={`flex flex-col h-full gap-3 ${embedded ? 'p-1.5' : 'p-4'} text-text-primary`}>
      {embedded ? (
        <div
          data-testid="activity-embedded-summary-bar"
          className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2"
        >
          <span className="font-display text-xs font-bold uppercase tracking-wider text-text-primary">
            Agent Activity Timeline
          </span>
          <span className="font-mono text-[11px] text-text-secondary">
            {rows.length} events · {uniqueAgents.length} agents · Live event-driven sync
          </span>
        </div>
      ) : (
        <RomanSurfaceHeader
          domainTag="OPERATIONS · ACTIVITY"
          title="Agent Activity Timeline"
          subtitle="Durable log of high-signal events emitted across agent orchestrations."
          waxSealBadge="TRACE LEDGER"
          headingLevel={2}
          icon={<Icon.bolt className="size-5 text-text-primary" />}
          gauges={[
            {
              label: 'EVENTS',
              valueText: `${rows.length}`,
              ratio: Math.max(0, Math.min(1, rows.length / 50)),
              tone: 'gold',
              purpose: 'Captured orchestration events in current window',
            },
            {
              label: 'AGENTS',
              valueText: `${uniqueAgents.length}`,
              ratio: Math.max(0, Math.min(1, uniqueAgents.length / 10)),
              tone: 'emerald',
              purpose: 'Distinct active agent actors in timeline',
            },
          ]}
        />
      )}

      <div className="vox-recessed-console-frame flex-1 flex flex-col gap-3 p-3 min-h-0">
      <Glass className="vox-recessed-tablet flex items-center gap-3 p-3 flex-wrap border-border-strong bg-bg-elevated">
        <div className="flex flex-col gap-1 min-w-[120px]">
          <label
            htmlFor="activity-agent-filter"
            className="text-[11px] uppercase font-bold tracking-wider text-text-muted"
          >
            Agent
          </label>
          <VoxSelect
            id="activity-agent-filter"
            value={agentFilter}
            aria-label="Agent"
            onValueChange={setAgentFilter}
            options={[
              { value: '', label: 'All Agents' },
              ...uniqueAgents.map((id) => ({ value: id, label: id })),
            ]}
            className="h-8 bg-bg-elevated border-border-subtle text-xs text-text-primary"
          />
        </div>

        <div className="flex flex-col gap-1 min-w-[150px]">
          <label
            htmlFor="activity-kind-filter"
            className="text-[11px] uppercase font-bold tracking-wider text-text-muted"
          >
            Event Type
          </label>
          <VoxSelect
            id="activity-kind-filter"
            value={kindFilter}
            aria-label="Event Type"
            onValueChange={setKindFilter}
            options={[
              { value: '', label: 'All Kinds' },
              ...uniqueKinds.map((kind) => ({ value: kind, label: kind })),
            ]}
            className="h-8 bg-bg-elevated border-border-subtle text-xs text-text-primary"
          />
        </div>

        <div
          data-testid="activity-live-sync-pill"
          title="Automatically syncs on vox://activity-appended and tab focus without manual polling"
          className="inline-flex h-8 items-center gap-1.5 rounded border border-border-subtle bg-bg-surface px-2.5 font-mono text-[11px] text-text-secondary mt-auto"
        >
          <span className="size-2 rounded-full bg-(--color-status-pass)" aria-hidden="true" />
          <span>{loading ? 'Syncing…' : `Live auto-sync · ${rows.length} events`}</span>
        </div>
      </Glass>

      <div className="flex-1 overflow-y-auto pr-1">
        {loading && rows.length === 0 ? (
          <div className="flex justify-center items-center py-20">
            <Icon.alert className="animate-spin text-text-muted size-8" />
          </div>
        ) : rows.length === 0 ? (
          <EmptyState
            variant="no-data"
            title="No Activity Logged"
            description="No high-signal agent events match your current filter settings."
          />
        ) : (
          <ActivityTimeline rows={rows} />
        )}
      </div>
      </div>
    </div>
  );
}
