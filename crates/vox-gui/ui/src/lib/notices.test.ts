import { describe, it, expect } from 'vitest';
import { noticeFromToast, noticeFromAgentEvent } from './notices';

describe('notices', () => {
  it('a toast becomes an action notice with a matching severity', () => {
    expect(noticeFromToast({ tone: 'ok', title: 'Saved', cause: 'backend-ok' }, 10)).toMatchObject(
      { severity: 'success', scope: 'action', source: 'backend-ok', title: 'Saved', groupKey: 'Saved', atMs: 10 });
    expect(noticeFromToast({ tone: 'warn', title: 'x', cause: 'backend-error' }, 1).severity).toBe('warning');
    expect(noticeFromToast({ tone: 'info', title: 'x', cause: 'external' }, 1).severity).toBe('info');
    expect(noticeFromToast({ tone: 'error', title: 'x', cause: 'backend-error' }, 1).severity).toBe('error');
  });

  it('only engine warnings and errors become notices, grouped per task', () => {
    const frame = (type: string, severity?: string) =>
      ({ id: 1, timestamp_ms: 5, severity, kind: { type, task_id: 7 } });
    expect(noticeFromAgentEvent(frame('task_failed', 'error'))).toMatchObject(
      { severity: 'error', scope: 'engine', source: 'engine', groupKey: 'engine:task_failed:7', atMs: 5 });
    expect(noticeFromAgentEvent(frame('tool_timed_out', 'warning'))?.severity).toBe('warning');
    expect(noticeFromAgentEvent(frame('task_completed', 'info'))).toBeNull();
    expect(noticeFromAgentEvent(frame('token_streamed', 'debug'))).toBeNull();
    expect(noticeFromAgentEvent(frame('task_failed'))).toBeNull();
  });

  it('an engine notice the mapper has no body for shows the event\'s own detail', () => {
    const n = noticeFromAgentEvent({ id: 1, timestamp_ms: 5, severity: 'error',
      kind: { type: 'injection_detected', detail: 'prompt override in tool output' } });
    expect(n?.body).toBe('prompt override in tool output');
    expect(n?.groupKey).toBe('engine:injection_detected');
  });
});
