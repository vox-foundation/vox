import React, { useEffect, useRef, useState } from 'react';
import { Glass } from '../../ui/Glass';
import { Pill } from '../../ui/Pill';
import { ContextWindowMeter } from './ContextWindowMeter';
import { useLabel } from '../../../hooks/useLanguage';
import { useMetricSeries, type MetricPoint } from '../../../hooks/useMetricSeries';
import { getContextBudget, type ContextBudgetPayload } from '../../../transport';
import type { Agent } from '../../../types/dashboard';



export interface ChatExecutionTask {
  id: string;
  title: string;
  status?: string;
}

export interface ChatExecutionRailKpis {
  activeAgents: { value: number };
  queueDepth: { value: number };
  mesh: { peers: number };
}

export interface ChatExecutionRailProps {
  tasks: ChatExecutionTask[];
  kpis: ChatExecutionRailKpis;
  intents?: string[];
  activeModel?: string | null;
  openrouterSpendUsd?: number | null;
  /** Session budget burn (DriveConsole $spent) — sparkled separately from OpenRouter. */
  sessionSpentUsd?: number | null;
  onNavigate: (viewKey: string) => void;
  /** Active chat session id — passed to get_context_budget so the meter shows real token usage. */
  sessionId?: string | null;
  /** Opens the inline Routing panel (folded Matrix surface — gui-ia-blueprint: matrix → chat rail). */
  onOpenRouting?: () => void;
  /** Live agent shards — the topology the retired chat Flow dock used to draw. */
  agents?: Agent[];
  selectedAgentId?: string;
  /** Open an agent on the Agents → Flow surface (full topology). */
  onOpenAgent?: (agentId: string) => void;
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
    'inline-flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1 text-[10px] text-text-muted transition hover:bg-overlay-subtle hover:text-text-secondary';

  const body = (
    <>
      <span className="uppercase tracking-[0.14em] text-text-muted">{label}</span>
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

export function ChatExecutionRail({
  tasks,
  kpis,
  intents,
  activeModel,
  openrouterSpendUsd,
  sessionSpentUsd,
  onNavigate,
  sessionId,
  onOpenRouting,
  agents = [],
  selectedAgentId,
  onOpenAgent,
}: ChatExecutionRailProps) {
  const [budget, setBudget] = useState<ContextBudgetPayload | null>(null);
  const { series: sessionSpendSeries, append: appendSessionSpend } = useMetricSeries(
    'chat.session-spend',
    [],
  );
  const prevSessionSpend = useRef<number | undefined>(undefined);

  useEffect(() => {
    getContextBudget(sessionId)
      .then(setBudget)
      .catch(() => {/* daemon unavailable; meter stays hidden */});
  }, [sessionId]);

  useEffect(() => {
    if (sessionSpentUsd == null || Number.isNaN(sessionSpentUsd)) return;
    if (prevSessionSpend.current !== sessionSpentUsd) {
      prevSessionSpend.current = sessionSpentUsd;
      appendSessionSpend(sessionSpentUsd);
    }
  }, [sessionSpentUsd, appendSessionSpend]);

  const peerLabel = kpis.mesh.peers === 1 ? '1 peer' : `${kpis.mesh.peers} peers`;

  return (
    <aside aria-label="Execution rail" className="w-full min-w-0">
      <Glass className="flex h-full flex-col gap-3 p-3">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-[10px] uppercase tracking-[0.18em] text-brass">{useLabel('chat-execution')}</h2>
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
                    <p className="mt-0.5 text-[10px] uppercase tracking-[0.12em] text-text-muted">
                      {task.status}
                    </p>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>

        {intents != null && intents.length > 0 && (
          <section
            role="region"
            aria-label="Intent map"
            className="flex flex-col gap-1 pt-3"
          >
            <div className="mb-1.5 border-b border-border-subtle pb-1 font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">
              Intents
            </div>
            {intents.slice(0, 3).map(intent => (
              <button
                key={intent}
                type="button"
                aria-label={intent}
                onClick={() => onOpenRouting?.()}
                className="rounded-sm px-2 py-1 text-left text-[11px] text-text-secondary transition hover:bg-overlay-subtle hover:text-brass"
              >
                {intent}
              </button>
            ))}
          </section>
        )}

        {agents.length > 0 && (
          <section aria-label="Agent shards" className="flex flex-col gap-1.5 pt-1">
            <div className="flex items-center justify-between gap-2 border-b border-border-subtle pb-1">
              <div className="font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">
                Agents
              </div>
              <button
                type="button"
                onClick={() => onNavigate('flow')}
                className="font-mono text-[10px] text-text-muted hover:text-brass"
              >
                Open topology
              </button>
            </div>
            <ul className="flex flex-col gap-1">
              {agents.map(agent => (
                <li key={agent.id}>
                  <button
                    type="button"
                    onClick={() => {
                      if (onOpenAgent) onOpenAgent(agent.id);
                      else onNavigate('flow');
                    }}
                    className={`flex w-full min-w-0 items-center gap-2 rounded-md px-1.5 py-1 text-left hover:bg-overlay-subtle ${
                      selectedAgentId === agent.id ? 'bg-overlay-subtle' : ''
                    }`}
                  >
                    <Pill phase={agent.phase} />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-[11px] text-text-primary">{agent.codename}</span>
                      <span className="block truncate font-mono text-[10px] text-text-muted">{agent.task}</span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}

        <section aria-label="Resource strip" className="flex flex-col gap-1 pt-3">
          <div className="mb-1.5 border-b border-border-subtle pb-1 font-display text-[9px] uppercase tracking-[0.28em] text-text-muted">
            Resources
          </div>
          <Segment
            testId="execution-rail-agents"
            label="Agents"
            value={String(kpis.activeAgents.value)}
            onClick={() => onNavigate('agents')}
          />
          <Segment
            testId="execution-rail-queue"
            label="Queue"
            value={String(kpis.queueDepth.value)}
            onClick={() => onNavigate('runs')}
          />
          <Segment
            testId="execution-rail-mesh"
            label="Mesh"
            value={peerLabel}
            onClick={() => onNavigate('mesh')}
          />
          {activeModel != null && activeModel !== '' && (
            <Segment
              testId="execution-rail-model"
              label="Model"
              value={activeModel}
              onClick={() => onNavigate('models')}
            />
          )}
          {openrouterSpendUsd != null && !Number.isNaN(openrouterSpendUsd) && (
            <Segment
              testId="execution-rail-openrouter"
              label="OpenRouter"
              value={formatOpenRouterSpend(openrouterSpendUsd)}
              onClick={() => onNavigate('settings')}
            />
          )}
          {sessionSpentUsd != null && !Number.isNaN(sessionSpentUsd) && (
            <Segment
              testId="execution-rail-session"
              label="Session"
              value={formatOpenRouterSpend(sessionSpentUsd)}
              trailing={
                sessionSpendSeries.length >= 2 ? (
                  <SessionSpendSpark series={sessionSpendSeries} />
                ) : null
              }
            />
          )}
        </section>

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

