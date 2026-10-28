import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  startRealBackendServer,
  redactTerminalText,
  type RealBackendHandle,
} from './realBackendServer';

describe('realBackendServer (REAL-01, REAL-02, REAL-03)', () => {
  let backend: RealBackendHandle;

  beforeEach(async () => {
    backend = await startRealBackendServer();
  });

  afterEach(async () => {
    await backend.close();
  });

  async function rpc(cmd: string, args: Record<string, unknown> = {}) {
    const resp = await fetch(`${backend.url}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ cmd, args, defaultView: 'chat' }),
    });
    expect(resp.ok).toBe(true);
    const data = await resp.json();
    return data.result;
  }

  it('persists chat sessions, messages, and oversight level on disk across requests (REAL-02)', async () => {
    const session = await rpc('chat_create_session', { title: 'Persisted Session' });
    expect(session.session_id).toBe('real-session-2');

    await rpc('chat_append_message', {
      input: {
        session_id: session.session_id,
        role: 'user',
        content: 'Verify real backend',
        already_submitted: true,
      },
    });
    const turn = await rpc('chat_turn', {
      input: {
        session_id: session.session_id,
        content: 'Verify real backend',
        execution: 'sync',
      },
    });
    expect(turn.content).toContain('Verified real-backend reply (cost: $0.0014');
    expect(turn.cost_usd).toBe(0.0014);

    const messages = await rpc('chat_get_messages', { sessionId: session.session_id });
    expect(messages).toHaveLength(2);
    expect(messages[0].role).toBe('user');
    expect(messages[1].role).toBe('assistant');

    await rpc('set_oversight_level', { level: 5 });
    expect(await rpc('get_oversight_level')).toBe(5);
    expect(backend.readState().oversightLevel).toBe(5);
  });

  it('executes terminal blocks and scrubs secrets on context export (REAL-03)', async () => {
    await rpc('term_session_submit', {
      sessionId: 's-1',
      line: '!echo Authorization: Bearer sk-1234567890abcdefghijklmnop',
    });
    await rpc('term_session_submit', {
      sessionId: 's-1',
      line: '40 + 2',
    });

    const attached = await rpc('term_session_attach', { sessionId: 's-1' });
    expect(attached.blocks).toHaveLength(3);
    expect(attached.blocks[2].output[0].text).toContain('Int(42)');

    const ctx = await rpc('term_session_context', { sessionId: 's-1', maxBlocks: 5 });
    expect(ctx[1].output).toContain('[REDACTED_KEY]');
    expect(ctx[1].output).not.toContain('sk-1234567890abcdefghijklmnop');
    expect(redactTerminalText('token sk-abcdef1234567890abcd')).toBe('token [REDACTED_KEY]');
  });
});
