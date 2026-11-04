import { describe, it, expect } from 'vitest';
import { buildChatTurn, CHAT_TURN_KEYS } from './buildChatTurn';

const full = {
  description: 'harden the crypto invariants',
  priority: 'urgent', active_skill: 'ponytail', tier: 'cloud',
  dry_run: true, clutch: 'genius', risk: 'low', mode: 'act',
  oversight_level: '3', kb_scope: 'docs',
  context: [
    { kind: 'file', ref: 'crates/vox-crypto/src/lib.rs' },
    { kind: 'agent', ref: 'A-01' },
  ],
  execution_mode: 'chat' as const,
};

describe('buildChatTurn', () => {
  it('carries every composer control', () => {
    const out = buildChatTurn(full, {
      sessionId: 's1', modelOverride: 'openrouter/anthropic/claude-opus',
      groundingCheckEnabled: true,
      // The real originating chat session, distinct from the dispatch
      // `sessionId` above (which can be a synthetic background-session id on
      // the background path) -- see bug 3, chat_session_id lineage.
      chatSessionId: 'real-chat-session',
    });
    expect(out.model_override).toBe('openrouter/anthropic/claude-opus');
    expect(out.tier).toBe('cloud');
    expect(out.clutch).toBe('genius');
    expect(out.risk).toBe('low');
    expect(out.priority).toBe('urgent');
    expect(out.grounding_check_enabled).toBe(true);
    // agent chips are not files
    expect(out.context_files).toEqual(['crates/vox-crypto/src/lib.rs']);
    // Bug 1: mode (plan/act/verify) must reach the wire.
    expect(out.mode).toBe('act');
    expect(out.oversight_level).toBe('3');
    expect(out.kb_scope).toBe('docs');
    // Bug 2: priority/dry_run must reach the wire (priority asserted above).
    expect(out.dry_run).toBe(true);
    // Bug 3: the real originating session, not the (possibly synthetic)
    // dispatch session_id.
    expect(out.chat_session_id).toBe('real-chat-session');
    expect(out.trace_id).toBeTruthy();
    expect(out.turn_id).toBeTruthy();
  });

  it('preserves full knob matrix across all execution modes, tiers, clutches, risks, modes, oversight levels, and kb scopes', () => {
    const executionModes = [
      { mode: 'chat' as const, expected: 'sync' },
      { mode: 'task' as const, expected: 'background' },
      { mode: 'plan' as const, expected: 'plan' },
    ];
    const interactionModes = ['plan', 'act', 'verify'];
    const tiers = ['local', 'mesh', 'cloud', 'auto'];
    const clutches = ['free', 'efficiency', 'balanced', 'genius'];
    const risks = ['high', 'moderate', 'low'];
    const oversights = ['1', '2', '3', '4', '5'];
    const kbScopes = ['auto', 'docs', 'code', 'memory', 'off'];

    let combinationsTested = 0;
    for (const exec of executionModes) {
      for (const mode of interactionModes) {
        for (const tier of tiers) {
          for (const clutch of clutches) {
            for (const risk of risks) {
              const oversight = oversights[combinationsTested % oversights.length];
              const kbScope = kbScopes[combinationsTested % kbScopes.length];
              const out = buildChatTurn(
                {
                  description: `test-${combinationsTested}`,
                  execution_mode: exec.mode,
                  mode,
                  tier,
                  clutch,
                  risk,
                  oversight_level: oversight,
                  kb_scope: kbScope,
                  dry_run: combinationsTested % 2 === 0,
                  priority: combinationsTested % 3 === 0 ? 'urgent' : 'normal',
                },
                {
                  sessionId: 'matrix-session',
                  groundingCheckEnabled: combinationsTested % 2 === 1,
                },
              );
              expect(out.execution).toBe(exec.expected);
              expect(out.mode).toBe(mode);
              expect(out.tier).toBe(tier);
              expect(out.clutch).toBe(clutch);
              expect(out.risk).toBe(risk);
              expect(out.oversight_level).toBe(oversight);
              expect(out.kb_scope).toBe(kbScope);
              combinationsTested += 1;
            }
          }
        }
      }
    }
    expect(combinationsTested).toBe(3 * 3 * 4 * 4 * 3); // 432 combinations
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

  // D4: the daemon's research classifier owns routing now — slash commands stay
  // on the sync chat path with their raw text, so the turn carries its own
  // research_trace event.
  it('keeps research slash commands on the sync path with the raw text', () => {
    const t = buildChatTurn({ description: '/deepresearch compare a and b' }, { sessionId: 's1' });
    expect(t.execution).toBe('sync');
    expect(t.content).toBe('/deepresearch compare a and b');
    expect(t.research_scope ?? null).toBeNull();
    expect(t.force_research ?? null).toBeNull();

    const quick = buildChatTurn({ description: '/research best noise cancelling headphones' }, { sessionId: 's1' });
    expect(quick.execution).toBe('sync');
    expect(quick.content).toBe('/research best noise cancelling headphones');
  });

  it('still forwards slash domain/site hints', () => {
    const t = buildChatTurn(
      { description: '/research --domain=codegen --site=docs.rs tokio select' },
      { sessionId: 's1' },
    );
    expect(t.execution).toBe('sync');
    expect(t.domain_mode).toBe('codegen');
    expect(t.site_scope).toBe('docs.rs');
  });
});
