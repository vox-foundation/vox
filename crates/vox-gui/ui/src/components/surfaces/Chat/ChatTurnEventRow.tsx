import React from 'react';
import type { TurnEventDto } from '../../../types/dashboard';

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

  return null;
}

