import React, { useCallback, useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invokeCmd } from '../../../transport';
import { Glass } from '../../ui/Glass';
import { EmptyState } from '../../ui/EmptyState';
import { StatusPill } from '../../ui/StatusPill';
import { DataTable } from '../../ui/DataTable';
import { Icon } from '../../ui/Icons';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { RUNS_POLL_MS, RUNS_LIST_LIMIT, SCOREBOARD_WINDOW_DAYS } from '../../../config/constants';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import type { Toast } from '../../../types/tauri';

interface ScoreboardRow {
  model_id: string;
  task_category: string;
  strength_tag: string;
  n_calls: number;
  success_rate: number;
  p50_latency_ms?: number | null;
  cost_per_success_usd?: number | null;
  quality_score: number;
}

interface RunRow {
  run_id: string;
  workflow_name: string;
  status: string;
  planned_steps: number;
  completed_steps: number;
  updated_at_ms: number;
  last_error?: string | null;
  command?: string | null;
  model?: string | null;
  cost_usd?: number | null;
}

interface RunsViewProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}

function isRunCompleted(status: string): boolean {
  return /complete|done|success/i.test(status);
}

export function RunsView({ pushToast, gamifyEnabled = false }: RunsViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [scoreboard, setScoreboard] = useState<ScoreboardRow[]>([]);
  const [runs, setRuns] = useState<RunRow[]>([]);
  const [decision, setDecision] = useState<any>(null);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [fetchedRun, setFetchedRun] = useState<RunRow | null>(null);
  const [loading, setLoading] = useState(true);
  const [highSuccessOnly, setHighSuccessOnly] = useState(false);
  const [runStatusFilter, setRunStatusFilter] = useState<'all' | 'completed' | 'failed'>('all');
  const rewardedRunIds = useRef<Set<string>>(new Set());

  const refresh = useCallback(async () => {
    try {
      const sb = await invokeCmd<ScoreboardRow[]>('get_model_scoreboard', { windowDays: SCOREBOARD_WINDOW_DAYS });
      setScoreboard(sb);
      const recent = await invokeCmd<RunRow[]>('list_gui_runs', { limit: RUNS_LIST_LIMIT });
      setRuns(recent);
      for (const run of recent) {
        if (isRunCompleted(run.status) && !rewardedRunIds.current.has(run.run_id)) {
          rewardedRunIds.current.add(run.run_id);
          void recordGamifyGuiEvent(
            'workflow_completed',
            { run_id: run.run_id, workflow_name: run.workflow_name },
            { enabled: gamifyEnabled }
          );
        }
      }
      const summary = await invokeCmd<any>('get_routing_summary_live');
      setDecision(summary?.decision_preview ?? null);
    } catch (err) {
      pushToast({ tone: 'error', title: 'Runs load failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setLoading(false);
    }
  }, [pushToast, gamifyEnabled]);

  useEffect(() => {
    refresh();
    // Embedded mini-render: one initial fetch only, no repeating poll.
    if (embedded) return;
    const id = setInterval(refresh, RUNS_POLL_MS);
    return () => clearInterval(id);
  }, [refresh, embedded]);

  useEffect(() => {
    if (!selectedRunId) {
      setFetchedRun(null);
      return;
    }
    let cancelled = false;
    invokeCmd<RunRow | null>('get_gui_run', { runId: selectedRunId })
      .then((row) => {
        if (!cancelled && row) setFetchedRun(row);
      })
      .catch(() => {
        if (!cancelled) setFetchedRun(null);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedRunId]);

  const listSelected = runs.find((r) => r.run_id === selectedRunId) ?? runs[0] ?? null;
  const selectedRun =
    fetchedRun && fetchedRun.run_id === (selectedRunId ?? listSelected?.run_id)
      ? fetchedRun
      : listSelected;

  const scoreboardCols = [
    { key: 'model_id', header: 'Model' },
    { key: 'task_category', header: 'Cat' },
    { key: 'n_calls', header: 'Calls' },
    { 
      key: 'success_rate', 
      header: 'Ok%', 
      render: (r: ScoreboardRow) => (
        <span className={r.success_rate > 0.9 ? 'text-emerald-400' : r.success_rate > 0.75 ? 'text-amber-400' : 'text-rose-400'}>
          {(r.success_rate * 100).toFixed(0)}%
        </span>
      ) 
    },
    { 
      key: 'p50_latency_ms', 
      header: 'p50', 
      render: (r: ScoreboardRow) => <span>{r.p50_latency_ms ? `${(r.p50_latency_ms / 1000).toFixed(1)}s` : '—'}</span> 
    },
    {
      key: 'cost_per_success_usd',
      header: '$/succ',
      render: (r: ScoreboardRow) => <span>{r.cost_per_success_usd != null ? `$${r.cost_per_success_usd.toFixed(4)}` : '—'}</span>
    },
    // No `Q` (quality_score) column: the value is a constant, not a measurement.
    // Its two writers are `success ? 1.0 : 0.0` per call and
    // `COALESCE(AVG(llm_feedback.rating)/5.0, 1.0)` over a table with zero rows
    // (`FeedbackCollector` is constructed nowhere outside its own doctest), so
    // every model scored a flat 1.00 — rendered here in bold brass, the most
    // prominent styling in the table. Restore this column when the completeness
    // gate gives `quality_score` a real definition (plan Task M2); until then a
    // dash would wrongly read as "no data yet" rather than "this measures
    // nothing".
  ];

  const runsCols = [
    { 
      key: 'run_id', 
      header: 'Run ID', 
      render: (r: RunRow) => <span className="font-mono text-text-muted">#{r.run_id}</span> 
    },
    { 
      key: 'workflow_name', 
      header: 'Workflow',
      render: (r: RunRow) => (
        <button
          type="button"
          onClick={() => setSelectedRunId(r.run_id)}
          aria-pressed={selectedRun?.run_id === r.run_id}
          className="hover:text-brass text-left font-medium outline-hidden focus:ring-1 focus:ring-brass/40 rounded-sm px-1 -mx-1"
        >
          {r.workflow_name}
        </button>
      )
    },
    { 
      key: 'status', 
      header: 'Status', 
      render: (r: RunRow) => (
        <StatusPill 
          tone={r.status === 'success' || r.status === 'complete' || r.status === 'done' ? 'pass' : r.status === 'failed' ? 'fail' : 'Executing'} 
          label={r.status} 
          size="xs" 
        />
      ) 
    },
    { 
      key: 'steps', 
      header: 'Steps', 
      render: (r: RunRow) => <span>{r.completed_steps}/{r.planned_steps}</span> 
    },
  ];

  const avgSuccess =
    scoreboard.length > 0
      ? scoreboard.reduce((acc, r) => acc + r.success_rate, 0) / scoreboard.length
      : 0;
  const completedCount = runs.filter((r) => isRunCompleted(r.status)).length;
  const failedCount = runs.filter((r) => /fail|error/i.test(r.status)).length;
  const displayedScoreboard = highSuccessOnly
    ? scoreboard.filter((r) => r.success_rate > 0.9)
    : scoreboard;
  const displayedRuns =
    runStatusFilter === 'completed'
      ? runs.filter((r) => isRunCompleted(r.status))
      : runStatusFilter === 'failed'
        ? runs.filter((r) => /fail|error/i.test(r.status))
        : runs;

  return (
    <div className="@container/surface flex min-h-0 min-w-0 flex-col gap-4 p-4">
      {!embedded && (
        <RomanSurfaceHeader
          domainTag="COMPUTE · WORKFLOW TELEMETRY"
          title="Workflow Runs & Model Scoreboard"
          subtitle="Inspect 7-day empirical model success rates, p50 latency, and live workflow step receipts."
          waxSealBadge="TELEMETRY LIVE"
          icon={<Icon.bolt className="size-5 text-text-primary" aria-hidden="true" />}
          gauges={[
            {
              label: 'Models',
              valueText: `${scoreboard.length}`,
              ratio: scoreboard.length > 0 ? Math.max(0, Math.min(1, avgSuccess)) : 0,
              tone: 'gold',
              purpose: 'Mean 7-day empirical model success rate across categories',
              actionHint: highSuccessOnly
                ? 'Click to show all tracked models'
                : 'Click to filter scoreboard to models >90% success',
              active: highSuccessOnly,
              onClick: () => setHighSuccessOnly((v) => !v),
            },
            {
              label: 'Runs',
              valueText: `${completedCount}/${runs.length}`,
              ratio: runs.length > 0 ? completedCount / runs.length : 0,
              tone: 'emerald',
              purpose: 'Completed workflow runs out of loaded recent activity',
              actionHint:
                runStatusFilter === 'completed'
                  ? 'Click to show all workflow runs'
                  : 'Click to filter recent activity to completed runs',
              active: runStatusFilter === 'completed',
              onClick: () =>
                setRunStatusFilter((f) => (f === 'completed' ? 'all' : 'completed')),
            },
            {
              label: 'Failed',
              valueText: `${failedCount}`,
              ratio: runs.length > 0 ? failedCount / runs.length : 0,
              tone: failedCount > 0 ? 'crimson' : 'emerald',
              purpose: 'Workflow runs ending with failure status',
              actionHint:
                runStatusFilter === 'failed'
                  ? 'Click to show all workflow runs'
                  : 'Click to filter recent activity to failed runs',
              active: runStatusFilter === 'failed',
              onClick: () =>
                setRunStatusFilter((f) => (f === 'failed' ? 'all' : 'failed')),
            },
          ]}
        />
      )}
      {decision && (
        <Glass className="p-3">
          <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">Latest Route Decision</div>
          <div className="mt-1 font-mono text-xs text-text-secondary">{decision.selected_model}</div>
          <div className="text-[10px] text-text-muted mt-1">
            state={decision.discovery_state}
            {decision.intelligence_score != null && ` · intel=${decision.intelligence_score.toFixed(2)}`}
            {decision.efficiency_score != null && ` · eff=${decision.efficiency_score.toFixed(2)}`}
            {decision.latency_score != null && ` · lat=${decision.latency_score.toFixed(2)}`}
          </div>
        </Glass>
      )}

      <BentoGrid preset="hero" className="items-start">
        <BentoCard as="section" span="hero-main" className="gap-3">
          <h2 className="font-display text-sm tracking-widest uppercase text-text-secondary">Model Scoreboard (7d)</h2>
          <BentoScrollRegion ariaLabel="Model scoreboard 7-day results" className="max-h-[32rem]">
            <DataTable
              rows={displayedScoreboard}
              columns={scoreboardCols}
              getRowId={r => `${r.model_id}-${r.task_category}`}
              loading={loading}
              density="compact"
              emptyState={
                <EmptyState
                  variant="no-data"
                  title="No model runs tracked yet"
                  description="Scoreboard data accumulates dynamically once agents complete routing workflows."
                />
              }
            />
          </BentoScrollRegion>
        </BentoCard>

        <BentoCard as="section" span="hero-rail" className="gap-3">
          <h2 className="font-display text-sm tracking-widest uppercase text-text-secondary">Recent Activity</h2>
          <BentoScrollRegion ariaLabel="Recent workflow runs" className="max-h-[24rem]">
            <DataTable
              rows={displayedRuns}
              columns={runsCols}
              getRowId={r => r.run_id}
              loading={loading}
              density="compact"
              emptyState={
                <EmptyState
                  variant="no-data"
                  title="No recent workflows"
                  description="A list of task executions and model calls appears here dynamically."
                />
              }
            />
          </BentoScrollRegion>
          {selectedRun && (
            <Glass size="sm" className="mt-1 rounded-lg border border-border-subtle bg-bg-elevated p-3">
              <div className="font-display text-[10px] tracking-[0.2em] uppercase text-text-muted">Run Details</div>
              <div className="mt-2 text-[11px] text-text-secondary">{selectedRun.workflow_name}</div>
              <div className="font-mono text-[10px] text-text-muted mt-1">{selectedRun.run_id}</div>
              <div className="text-[10px] text-text-muted mt-1">
                status={selectedRun.status} · steps={selectedRun.completed_steps}/{selectedRun.planned_steps}
              </div>
              <div className="text-[10px] text-text-muted mt-1">
                updated={new Date(selectedRun.updated_at_ms).toLocaleString()}
              </div>
              {selectedRun.command && (
                <div className="font-mono text-[10px] text-text-muted mt-1 break-all">
                  cmd={selectedRun.command}
                </div>
              )}
              {selectedRun.model || selectedRun.cost_usd != null ? (
                <div className="text-[10px] text-text-muted mt-1">
                  {selectedRun.model ? `model=${selectedRun.model}` : null}
                  {selectedRun.model && selectedRun.cost_usd != null ? ' · ' : null}
                  {selectedRun.cost_usd != null ? `cost=$${selectedRun.cost_usd.toFixed(4)}` : null}
                </div>
              ) : null}
              {selectedRun.last_error ? (
                <pre className="mt-2 whitespace-pre-wrap rounded-sm border border-rose-300/20 bg-rose-950/20 p-2 text-[10px] text-rose-200">
                  {selectedRun.last_error}
                </pre>
              ) : (
                <div className="mt-2 text-[10px] text-emerald-300">No recorded error for this run.</div>
              )}
            </Glass>
          )}
        </BentoCard>
      </BentoGrid>
    </div>
  );
}
