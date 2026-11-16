import React, { useEffect, useId, useMemo, useState } from 'react';
import type { TurnEventDto } from '../../../types/dashboard';
import type { ChatVerbosity } from '../../../hooks/useChatVerbosity';
import { useTerminalPendingApprovals } from '../../../hooks/useTerminalPendingApprovals';
import { buildTurnTrace, summaryText } from '../../../lib/turnTrace';
import { Icon } from '../../ui/Icons';
import { TERMINAL_MONO_STYLE } from '../Console/terminalTheme';
import { TerminalApprovalCard } from '../Console/TerminalApprovalCard';
import { ChatTerminalBlockCard, isTerminalBlockToolName } from './ChatTerminalBlockCard';
import { ChatTurnEventRow } from './ChatTurnEventRow';

interface TurnTraceProps {
  events?: TurnEventDto[];
  verbosity: ChatVerbosity;
  latencyMs?: number;
  /** The reply's model id; shown plainly only when the turn has no routing decision. */
  modelId?: string;
  /** "not this one" on a skill-activation chip - see `ChatTurnEventRow`. */
  onExcludeSkill?: (skillId: string) => void;
  onNavigate?: (viewKey: string) => void;
}

/**
 * One assistant turn's account: inline skill chips and interrupts (things a human must act on),
 * then one summary row that expands into the ordered steps. Default expansion follows the
 * verbosity control (`buildTurnTrace`). Every row is built from server-derived event fields.
 */
export function TurnTrace({
  events,
  verbosity,
  latencyMs,
  modelId,
  onExcludeSkill,
  onNavigate,
}: TurnTraceProps) {
  const trace = useMemo(
    () => buildTurnTrace(events, verbosity, { latencyMs, modelId }),
    [events, verbosity, latencyMs, modelId],
  );
  const terminalEvents = useMemo(() => {
    const byKey = new Map<string, TurnEventDto>();
    (events ?? []).forEach((ev, idx) => {
      if (ev.kind !== 'tool_receipt' || !isTerminalBlockToolName(ev.tool)) {
        return;
      }
      const key =
        ev.block_id !== undefined && ev.block_id !== null
          ? `block:${String(ev.block_id)}`
          : typeof ev.receipt_id === 'string' && ev.receipt_id.length > 0
            ? `receipt:${ev.receipt_id}`
            : `idx:${idx}`;
      const prev = byKey.get(key);
      byKey.set(key, prev ? { ...prev, ...ev } : ev);
    });
    return Array.from(byKey.entries());
  }, [events]);
  const hasTerminalTool = useMemo(
    () => (events ?? []).some((ev) => isTerminalBlockToolName(ev.tool)),
    [events],
  );
  const terminalSessionId = useMemo(() => {
    for (const ev of events ?? []) {
      if (isTerminalBlockToolName(ev.tool) && typeof ev.session_id === 'string' && ev.session_id.length > 0) {
        return ev.session_id;
      }
    }
    return undefined;
  }, [events]);
  const { approvals, resolvingId, resolveApproval } = useTerminalPendingApprovals({
    sessionId: terminalSessionId,
    enabled: hasTerminalTool,
  });
  const [expanded, setExpanded] = useState(trace.defaultExpanded);
  useEffect(() => setExpanded(trace.defaultExpanded), [trace.defaultExpanded]);
  const stepsId = useId();
  const summary = summaryText(trace.summary);

  if (
    trace.steps.length === 0 &&
    trace.interrupts.length === 0 &&
    trace.inline.length === 0 &&
    terminalEvents.length === 0 &&
    approvals.length === 0 &&
    summary === ''
  ) {
    return null;
  }

  return (
    <div data-testid="chat-trace" className="mt-1 flex flex-col items-start gap-1">
      {trace.inline.map((event, i) => (
        <ChatTurnEventRow key={`inline-${i}`} event={event} onExcludeSkill={onExcludeSkill} onNavigate={onNavigate} />
      ))}
      {trace.interrupts.map((item, i) => (
        <ChatTurnEventRow key={`interrupt-${i}`} event={item.event} onNavigate={onNavigate} />
      ))}
      {terminalEvents.map(([key, ev]) => (
        <ChatTerminalBlockCard
          key={`term-${key}`}
          event={ev}
          pendingApprovals={approvals}
          resolvingApprovalId={resolvingId}
          onResolveApproval={resolveApproval}
          onNavigate={onNavigate}
        />
      ))}
      {terminalEvents.length === 0 && approvals[0] ? (
        <TerminalApprovalCard
          key={approvals[0].approval_id}
          compact
          approval={approvals[0]}
          resolving={resolvingId === approvals[0].approval_id}
          queueTotal={approvals.length}
          onResolve={resolveApproval}
        />
      ) : null}
      {trace.steps.length > 0 ? (
        <>
          <button
            type="button"
            data-testid="chat-trace-summary"
            data-terminal-mono="true"
            style={TERMINAL_MONO_STYLE}
            aria-expanded={expanded}
            aria-controls={stepsId}
            onClick={() => setExpanded((v) => !v)}
            className="flex items-center gap-1 rounded-md px-1 py-1 font-mono text-[11px] text-text-muted hover:text-text-secondary"
          >
            {expanded ? (
              <Icon.chevronDown className="size-3 shrink-0" aria-hidden="true" />
            ) : (
              <Icon.chevR className="size-3 shrink-0" aria-hidden="true" />
            )}
            <span>{summary || `${trace.steps.length} steps`}</span>
          </button>
          <ol
            id={stepsId}
            data-testid="chat-trace-steps"
            hidden={!expanded}
            className="space-y-1 border-l border-border-subtle pl-3"
          >
            {trace.steps.map((item, i) => (
              <li
                key={i}
                data-testid="chat-trace-step"
                data-kind={item.event.kind}
                data-status={item.status}
                className="flex items-center gap-2"
              >
                <ChatTurnEventRow event={item.event} onNavigate={onNavigate} />
                {item.count > 1 && (
                  <span data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="font-mono text-[11px] text-text-muted">
                    {item.count}×
                  </span>
                )}
              </li>
            ))}
          </ol>
        </>
      ) : summary !== '' ? (
        <div data-testid="chat-trace-summary" data-terminal-mono="true" style={TERMINAL_MONO_STYLE} className="px-1 py-1 font-mono text-[11px] text-text-muted">
          {summary}
        </div>
      ) : null}
    </div>
  );
}
