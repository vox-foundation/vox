import { describe, expect, it } from 'vitest';
import {
  DRIVE_EVENTS_CAP,
  DRIVE_EVENT_RAW_CAP,
  appendDriveEvent,
  clearDriveEvents,
  recordAgentFrame,
  type DriveEventState,
} from './driveEvents';

function empty(): DriveEventState {
  return { events: [], events_dropped: 0, last_turn_id: null, next_seq: 1 };
}

describe('driveEvents', () => {
  it('appends submit_ok and tracks turn id', () => {
    const next = appendDriveEvent(empty(), {
      turn_id: 't1',
      kind: 'submit_ok',
      text: 'hello',
    });
    expect(next.events).toHaveLength(1);
    expect(next.events[0]?.kind).toBe('submit_ok');
    expect(next.events[0]?.seq).toBe(1);
    expect(next.last_turn_id).toBe('t1');
    expect(next.next_seq).toBe(2);
  });

  it('drops oldest at CAP+1 and increments events_dropped by 1', () => {
    let state = empty();
    for (let i = 0; i < DRIVE_EVENTS_CAP + 1; i++) {
      state = appendDriveEvent(state, { turn_id: 't', kind: 'token_streamed', text: String(i) });
    }
    expect(state.events).toHaveLength(DRIVE_EVENTS_CAP);
    expect(state.events_dropped).toBe(1);
    expect(state.events[0]?.text).toBe('1');
  });

  it('redacts bearer-looking substrings in text', () => {
    const next = appendDriveEvent(empty(), {
      turn_id: 't',
      kind: 'diag',
      text: 'Authorization: Bearer sk-secret-value',
    });
    expect(next.events[0]?.text ?? '').not.toContain('sk-secret-value');
    expect(next.events[0]?.text ?? '').toMatch(/\[redacted\]/i);
  });

  it('caps and redacts raw JSON secrets', () => {
    const big = { headers: { Authorization: 'Bearer sk-secret-value' }, pad: 'x'.repeat(9000) };
    const next = appendDriveEvent(empty(), { turn_id: 't', kind: 'agent_event', raw: big });
    const serialized = JSON.stringify(next.events[0]?.raw);
    expect(serialized.length).toBeLessThanOrEqual(DRIVE_EVENT_RAW_CAP + 8);
    expect(serialized).not.toContain('sk-secret-value');
  });

  it('clearDriveEvents preserves next_seq', () => {
    const filled = appendDriveEvent(empty(), { turn_id: 't', kind: 'submit_ok' });
    const cleared = clearDriveEvents(filled);
    expect(cleared.events).toEqual([]);
    expect(cleared.events_dropped).toBe(0);
    expect(cleared.last_turn_id).toBeNull();
    expect(cleared.next_seq).toBe(filled.next_seq);
  });

  it('recordAgentFrame reads kind.type and kind.text', () => {
    const frame = { kind: { type: 'token_streamed', text: 'tok', session_id: 's1' } };
    const next = recordAgentFrame(empty(), frame, 'turn-1');
    expect(next.events[0]?.kind).toBe('token_streamed');
    expect(next.events[0]?.text).toBe('tok');
  });
});
