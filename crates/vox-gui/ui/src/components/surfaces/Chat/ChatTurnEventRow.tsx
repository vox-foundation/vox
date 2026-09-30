import React from 'react';
import type { TurnEventDto } from '../../../types/dashboard';
import { isKnownTurnEvent, modeLabel, routingModelLabel } from '../../../lib/turnEvents';

const CHIP =
  'flex items-center gap-2 self-start rounded-full border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[11px] text-text-secondary';

interface ChatTurnEventRowProps {
  event: TurnEventDto;
  /** "not this one" — appends the skill to session-scoped `skill_exclusions`
   *  and re-dispatches the turn. Only rendered for `skill_activated` events. */
  onExcludeSkill?: (skillId: string) => void;
}

/**
 * Renders a single chat-turn event derived from a tool call's RESULT (see
 * Rust `turn_event_for_result` and `receipt_turn_event`) — e.g. a chip naming a skill the model
 * loaded or a tool execution receipt chip. Deliberately separate from `ChatAgentEventRow` (which owns the
 * three HITL plan/verify controls) — this component owns nothing but
 * read-only chips plus the skill-exclusion action.
 *
 * An unrecognized `kind` renders nothing rather than throwing — event shapes
 * are additive and forward compatibility matters more than a hard failure
 * on an unknown one.
 */
export function ChatTurnEventRow({ event, onExcludeSkill }: ChatTurnEventRowProps) {
  if (!isKnownTurnEvent(event)) return null;
  if (event.kind === 'skill_activated') {
    const skillId = typeof event.skill_id === 'string' ? event.skill_id : 'unknown';
    return (
      <div
        data-testid="chat-turn-event-row"
        className="flex items-center gap-2 self-start rounded-full border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[10px] text-text-secondary"
      >
        <span>skill activated · {skillId}</span>
        {onExcludeSkill && skillId !== 'unknown' && (
          <button
            type="button"
            className="text-text-muted hover:text-brass"
            onClick={() => onExcludeSkill(skillId)}
          >
            not this one
          </button>
        )}
      </div>
    );
  }

  if (event.kind === 'tool_receipt') {
    if (typeof event.tool !== 'string' || typeof event.receipt_id !== 'string') {
      return null;
    }
    const tool = event.tool;
    const receiptId = event.receipt_id;
    const verified = event.verified === true;
    return (
      <div
        data-testid="chat-turn-receipt-row"
        data-verified={verified ? 'true' : 'false'}
        className="flex items-center gap-2 self-start rounded-full border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[10px] text-text-secondary"
        title={`receipt ${receiptId}`}
      >
        <span>
          receipt · {tool} · {verified ? 'verified' : 'unverified'}
        </span>
        <span className="text-text-muted">{receiptId.slice(0, 8)}</span>
      </div>
    );
  }

  if (event.kind === 'receipt_claims') {
    if (
      typeof event.valid !== 'number' ||
      !Number.isFinite(event.valid) ||
      typeof event.fabricated !== 'number' ||
      !Number.isFinite(event.fabricated) ||
      typeof event.unverified !== 'number' ||
      !Number.isFinite(event.unverified)
    ) {
      return null;
    }
    const valid = event.valid;
    const fabricated = event.fabricated;
    const unverified = event.unverified;
    const flagged = fabricated > 0 || unverified > 0;
    return (
      <div
        data-testid="chat-turn-claims-row"
        data-flagged={flagged ? 'true' : 'false'}
        className={`flex items-center gap-2 self-start rounded-full border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[10px] ${
          flagged ? 'text-amber-300' : 'text-text-secondary'
        }`}
      >
        <span>
          claims · {valid} valid · {fabricated} fabricated · {unverified} unverified
        </span>
      </div>
    );
  }

  if (event.kind === 'delegation_spawned') {
    const taskId = typeof event.task_id === 'number' ? event.task_id : null;
    return (
      <div data-testid="chat-turn-delegation-row" className={CHIP}>
        <span>
          delegated · agent {String(event.agent_id)}
          {taskId != null ? ` · task ${taskId}` : ''}
        </span>
      </div>
    );
  }

  if (event.kind === 'research_milestone') {
    // `query` is model-supplied and never rendered; counts only.
    return (
      <div data-testid="chat-turn-research-row" className={CHIP}>
        <span>
          research · {String(event.waves_executed)} waves · {String(event.claims_verified)} claims
          verified · {String(event.contradictions_resolved)} contradictions resolved
        </span>
      </div>
    );
  }

  if (event.kind === 'routing_decision') {
    const mode = typeof event.mode === 'string' ? event.mode : null;
    return (
      <div
        data-testid="chat-turn-routing-row"
        data-resolved-from={String(event.resolved_from)}
        title={typeof event.objective === 'string' ? event.objective : undefined}
        className={CHIP}
      >
        <span>
          routed to {routingModelLabel(event)}
          {mode ? ` · ${modeLabel(mode)}` : ''} — {String(event.reason)}
        </span>
      </div>
    );
  }

  return null;
}


