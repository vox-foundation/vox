import { describe, it, expect } from 'vitest';
import {
  noticeReducer, EMPTY_NOTICES, MAX_NOTICES, MAX_BODY_CHARS, COALESCE_WINDOW_MS, unreadProblems,
} from './noticeStore';

const warn = (title: string, atMs: number) =>
  ({ type: 'record' as const, input: { severity: 'warning' as const, scope: 'engine' as const, source: 'engine', title, atMs } });

describe('noticeReducer', () => {
  it('records newest first and counts unread problems', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('a', 1));
    s = noticeReducer(s, { type: 'record', input: { severity: 'success', scope: 'action', source: 'backend-ok', title: 'ok', atMs: 2 } });
    expect(s.notices.map(n => n.title)).toEqual(['ok', 'a']);
    expect(unreadProblems(s.notices)).toBe(1);
  });

  it('coalesces a repeat inside the window into one notice with a count', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('Tool timed out', 1_000));
    s = noticeReducer(s, warn('Tool timed out', 1_000 + COALESCE_WINDOW_MS - 1));
    expect(s.notices).toHaveLength(1);
    expect(s.notices[0].count).toBe(2);
    s = noticeReducer(s, warn('Tool timed out', 1_000 + 3 * COALESCE_WINDOW_MS));
    expect(s.notices).toHaveLength(2);
  });

  it('a merged repeat shows the latest title and never lowers the severity', () => {
    const rec = (severity: 'error' | 'warning', title: string, atMs: number) =>
      ({ type: 'record' as const, input: { severity, scope: 'engine' as const, source: 'engine', title, groupKey: 'g', atMs } });
    let s = noticeReducer(EMPTY_NOTICES, rec('error', 'budget critical', 1));
    s = noticeReducer(s, rec('warning', 'budget high', 2));
    expect(s.notices).toHaveLength(1);
    expect(s.notices[0]).toMatchObject({ title: 'budget high', severity: 'error', count: 2 });
  });

  it('a repeat after reading brings the notice back as unread', () => {
    let s = noticeReducer(EMPTY_NOTICES, warn('x', 1));
    s = noticeReducer(s, { type: 'markAllRead' });
    expect(unreadProblems(s.notices)).toBe(0);
    s = noticeReducer(s, warn('x', 2));
    expect(unreadProblems(s.notices)).toBe(1);
  });

  it('keeps at most MAX_NOTICES, dropping the oldest', () => {
    let s = EMPTY_NOTICES;
    for (let i = 0; i < MAX_NOTICES + 5; i++) s = noticeReducer(s, warn(`n${i}`, i * 10 * COALESCE_WINDOW_MS));
    expect(s.notices).toHaveLength(MAX_NOTICES);
    expect(s.notices[s.notices.length - 1].title).toBe('n5');
  });

  it('caps a notice body at MAX_BODY_CHARS', () => {
    const s = noticeReducer(EMPTY_NOTICES, { type: 'record', input: {
      severity: 'error', scope: 'engine', source: 'engine', title: 't', body: 'x'.repeat(5000), atMs: 1 } });
    expect(s.notices[0].body).toHaveLength(MAX_BODY_CHARS);
  });
});
