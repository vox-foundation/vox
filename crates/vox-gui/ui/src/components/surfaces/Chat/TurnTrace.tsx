import React, { useEffect, useId, useMemo, useState } from 'react';
import type { TurnEventDto } from '../../../types/dashboard';
import type { ChatVerbosity } from '../../../hooks/useChatVerbosity';
import { buildTurnTrace, summaryText } from '../../../lib/turnTrace';
import { ChatTurnEventRow } from './ChatTurnEventRow';

interface TurnTraceProps {
  events?: TurnEventDto[];
  verbosity: ChatVerbosity;
  latencyMs?: number;
  /** The reply's model id; shown plainly only when the turn has no routing decision. */
  modelId?: string;
  /** "not this one" on a skill-activation chip — see `ChatTurnEventRow`. */
  onExcludeSkill?: (skillId: string) => void;
}

/**
 * One assistant turn's account: inline skill chips and interrupts (things a human must act on),
 * then one summary row that expands into the ordered steps. Default expansion follows the
 * verbosity control (`buildTurnTrace`). Every row is built from server-derived event fields.
 */
export function TurnTrace({ events, verbosity, latencyMs, modelId, onExcludeSkill }: TurnTraceProps) {
  const trace = useMemo(
    () => buildTurnTrace(events, verbosity, { latencyMs, modelId }),
    [events, verbosity, latencyMs, modelId],
  );
  const [expanded, setExpanded] = useState(trace.defaultExpanded);
  useEffect(() => setExpanded(trace.defaultExpanded), [trace.defaultExpanded]);
  const stepsId = useId();
  const summary = summaryText(trace.summary);

  if (
    trace.steps.length === 0 &&
    trace.interrupts.length === 0 &&
    trace.inline.length === 0 &&
    summary === ''
  ) {
    return null;
  }

  return (
    <div data-testid="chat-trace" className="mt-1 flex flex-col items-start gap-1">
      {trace.inline.map((event, i) => (
        <ChatTurnEventRow key={`inline-${i}`} event={event} onExcludeSkill={onExcludeSkill} />
      ))}
      {trace.interrupts.map((item, i) => (
        <ChatTurnEventRow key={`interrupt-${i}`} event={item.event} />
      ))}
      {trace.steps.length > 0 ? (
        <>
          <button
            type="button"
            data-testid="chat-trace-summary"
            aria-expanded={expanded}
            aria-controls={stepsId}
            onClick={() => setExpanded((v) => !v)}
            className="flex items-center gap-1.5 rounded-md px-1 py-0.5 font-mono text-[11px] text-text-muted hover:text-text-secondary"
          >
            <span aria-hidden="true">{expanded ? '▾' : '▸'}</span>
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
                <ChatTurnEventRow event={item.event} />
                {item.count > 1 && (
                  <span className="font-mono text-[11px] text-text-muted">{item.count}×</span>
                )}
              </li>
            ))}
          </ol>
        </>
      ) : summary !== '' ? (
        <div data-testid="chat-trace-summary" className="px-1 py-0.5 font-mono text-[11px] text-text-muted">
          {summary}
        </div>
      ) : null}
    </div>
  );
}
