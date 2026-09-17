import { describe, it, expect } from 'vitest';
import { buildChatTurn, CHAT_TURN_KEYS } from './buildChatTurn';

const full = {
  description: 'harden the crypto invariants',
  priority: 'urgent', active_skill: 'ponytail', tier: 'cloud',
  dry_run: true, clutch: 'genius', risk: 'low', mode: 'act',
  context: [
    { kind: 'file', ref: 'crates/vox-crypto/src/lib.rs' },
    { kind: 'agent', ref: 'A-01' },
  ],
  execution_mode: 'chat' as const,
};

describe('buildChatTurn', () => {
  it('carries every composer control', () => {
    const out = buildChatTurn(full, {
      sessionId: 's1', modelOverride: 'openrouter/anthropic/claude-opus-5',
      groundingCheckEnabled: true,
      // The real originating chat session, distinct from the dispatch
      // `sessionId` above (which can be a synthetic background-session id on
      // the background path) -- see bug 3, chat_session_id lineage.
      chatSessionId: 'real-chat-session',
    });
    expect(out.model_override).toBe('openrouter/anthropic/claude-opus-5');
    expect(out.tier).toBe('cloud');
    expect(out.clutch).toBe('genius');
    expect(out.risk).toBe('low');
    expect(out.priority).toBe('urgent');
    expect(out.grounding_check_enabled).toBe(true);
    // agent chips are not files
    expect(out.context_files).toEqual(['crates/vox-crypto/src/lib.rs']);
    // Bug 1: mode (plan/act/verify) must reach the wire.
    expect(out.mode).toBe('act');
    // Bug 2: priority/dry_run must reach the wire (priority asserted above).
    expect(out.dry_run).toBe(true);
    // Bug 3: the real originating session, not the (possibly synthetic)
    // dispatch session_id.
    expect(out.chat_session_id).toBe('real-chat-session');
    expect(out.trace_id).toBeTruthy();
    expect(out.turn_id).toBeTruthy();
  });

  it('uses pre-minted correlation ids when provided', () => {
    const out = buildChatTurn(full, {
      sessionId: 's1',
      traceId: 'trace-fixed',
      turnId: 'turn-fixed',
    });
    expect(out.trace_id).toBe('trace-fixed');
    expect(out.turn_id).toBe('turn-fixed');
  });

  it('prefers payload turn_id/trace_id when ctx omits them (Drive handoff)', () => {
    const out = buildChatTurn(
      { ...full, turn_id: 'drive-turn', trace_id: 'drive-trace' },
      { sessionId: 's1' },
    );
    expect(out.turn_id).toBe('drive-turn');
    expect(out.trace_id).toBe('drive-trace');
  });

  it('ctx ids win over payload ids when both set', () => {
    const out = buildChatTurn(
      { ...full, turn_id: 'drive-turn', trace_id: 'drive-trace' },
      { sessionId: 's1', turnId: 'ctx-turn', traceId: 'ctx-trace' },
    );
    expect(out.turn_id).toBe('ctx-turn');
    expect(out.trace_id).toBe('ctx-trace');
  });

  it('falls back chat_session_id to the dispatch sessionId when ctx omits it', () => {
    const out = buildChatTurn(full, { sessionId: 's1' });
    expect(out.chat_session_id).toBe('s1');
  });

  it('takes execution from the composer switch, not a legacy sentinel', () => {
    expect(buildChatTurn(full, { sessionId: 's1' }).execution).toBe('sync');
    expect(buildChatTurn({ ...full, execution_mode: 'task' }, { sessionId: 's1' }).execution)
      .toBe('background');
  });

  it('emits a stable key set', () => {
    const out = buildChatTurn(full, { sessionId: 's1' });
    expect(Object.keys(out).sort()).toEqual([...CHAT_TURN_KEYS].sort());
  });

  it('has no intent field', () => {
    // Loquela folds intent into `description` via composeDescription. A field
    // here would be permanently null, and double-counted if Loquela ever
    // emitted it without removing the fold.
    expect(CHAT_TURN_KEYS).not.toContain('intent');
  });

  it('routes /research and /deepresearch slash commands to background execution with force_research', () => {
    const res1 = buildChatTurn({ description: '/research best noise cancelling headphones' }, { sessionId: 's1' });
    expect(res1.execution).toBe('background');
    expect(res1.force_research).toBe(true);
    expect(res1.content).toBe('best noise cancelling headphones');

    const res2 = buildChatTurn({ description: '/deepresearch compare Rust async runtimes' }, { sessionId: 's1' });
    expect(res2.execution).toBe('background');
    expect(res2.force_research).toBe(true);
    expect(res2.content).toBe('compare Rust async runtimes');
    expect(res2.research_scope).toBe('deep');

    const res3 = buildChatTurn(
      { description: '/research --domain=codegen --site=docs.rs tokio select' },
      { sessionId: 's1' }
    );
    expect(res3.execution).toBe('background');
    expect(res3.force_research).toBe(true);
    expect(res3.content).toBe('tokio select');
    expect(res3.domain_mode).toBe('codegen');
    expect(res3.site_scope).toBe('docs.rs');
  });
});
