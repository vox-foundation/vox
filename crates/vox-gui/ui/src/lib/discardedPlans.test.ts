import { describe, expect, it } from 'vitest';
import {
  forgetDiscardedPlan,
  isPlanDiscarded,
  rememberDiscardedPlan,
} from './discardedPlans';

describe('discardedPlans', () => {
  it('remembers a discard and reports it', () => {
    const next = rememberDiscardedPlan({}, 'chat-a', 'plan-a');
    expect(isPlanDiscarded(next, 'chat-a', 'plan-a')).toBe(true);
    expect(isPlanDiscarded(next, 'chat-a', 'plan-b')).toBe(false);
    expect(isPlanDiscarded(next, 'chat-b', 'plan-a')).toBe(false);
  });

  it('is idempotent for the same chat/plan pair', () => {
    const once = rememberDiscardedPlan({}, 'chat-a', 'plan-a');
    const twice = rememberDiscardedPlan(once, 'chat-a', 'plan-a');
    expect(twice).toBe(once);
    expect(twice['chat-a']).toEqual(['plan-a']);
  });

  it('forgets a discarded plan so a later /plan bind can reopen it', () => {
    const discarded = rememberDiscardedPlan({}, 'chat-a', 'plan-a');
    const cleared = forgetDiscardedPlan(discarded, 'chat-a', 'plan-a');
    expect(isPlanDiscarded(cleared, 'chat-a', 'plan-a')).toBe(false);
    expect(cleared).toEqual({});
  });
});
