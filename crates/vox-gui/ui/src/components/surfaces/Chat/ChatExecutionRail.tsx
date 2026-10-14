import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../../ui/Glass';
import { ContextWindowMeter } from './ContextWindowMeter';
import { useLabel } from '../../../hooks/useLanguage';
import { useMetricSeries, type MetricPoint } from '../../../hooks/useMetricSeries';
import { getContextBudget, type ContextBudgetPayload } from '../../../transport';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import { modelStateHint, type RailRouting } from '../../../lib/routingSummary';



export interface ChatExecutionTask {
  id: string;
  title: string;
  status?: string;
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
  /** Completed assistant turns in this session (`countCompletedTurns`); each change re-reads the context budget. */
  turnsCompleted?: number;
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
  turnsCompleted = 0,
}: ChatExecutionRailProps) {
  const [budget, setBudget] = useState<ContextBudgetPayload | null>(null);
  const budgetSessionRef = useRef(sessionId);

  // Re-read the context budget on every session change and every completed turn (read once per session, it went stale).
  useEffect(() => {
    let cancelled = false;
    if (budgetSessionRef.current !== sessionId) {
      budgetSessionRef.current = sessionId;
      setBudget(null); // a new session never shows the previous session's reading
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

  const stateHint = modelStateHint(routing?.state);
  const routingLine = routing
    ? `Routes to ${routing.model}${routing.reason ? ` — ${routing.reason}` : ''}`
    : '';

  return (
    <aside aria-label="Execution rail" className="w-full min-w-0">
      <Glass className="flex h-full flex-col gap-3 p-3">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-[11px] uppercase tracking-[0.08em] text-brass">{useLabel('chat-execution')}</h2>
        </div>

        <section
          role="region"
          aria-label="Active tasks"
          className="flex min-h-0 flex-1 flex-col gap-2"
        >
          {tasks.length === 0 ? (
            <p className="text-[11px] text-text-muted">No active tasks for this session.</p>
          ) : (
            <ul className="flex flex-col gap-1.5 overflow-y-auto custom-scrollbar">
              {tasks.map(task => (
                <li
                  key={task.id}
                  className="rounded-lg border border-border-subtle bg-overlay-subtle px-2.5 py-2"
                >
                  <p className="text-xs text-text-secondary leading-snug truncate" title={task.title}>{task.title}</p>
                  {task.status && (
                    <p className="mt-0.5 text-[11px] uppercase tracking-[0.08em] text-text-muted">
                      {task.status}
                    </p>
                  )}
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
              ))}
            </ul>
          )}
        </section>

        {routing && (
          <section role="region" aria-label="Routing" className="flex flex-col gap-1 pt-3">
            <div className="mb-1.5 flex items-baseline justify-between gap-2 border-b border-border-subtle pb-1">
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
              className="truncate rounded-sm px-2 py-1 text-left text-[11px] text-text-secondary transition hover:bg-overlay-subtle hover:text-brass"
            >
              {routingLine}
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
          <section aria-label="Session spend" className="flex flex-col gap-1 pt-3">
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
          />
        )}
      </Glass>
    </aside>
  );
}

