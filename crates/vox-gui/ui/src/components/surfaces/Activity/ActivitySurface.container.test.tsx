// @vitest-environment jsdom
import { render, screen, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';

// activityQuery is TYPED Promise<ActivityRowDto[]>, but at runtime it can resolve
// null (e.g. the visual-audit Tauri mock, or a backend/IPC returning null). The
// surface must not crash on that.

// Captured callbacks so tests can fire synthetic events.
let capturedAgentEventsCb: (() => void) | null = null;
let capturedAppendedCb: (() => void) | null = null;

vi.mock('../../../transport', () => ({
  activityQuery: vi.fn().mockResolvedValue(null),
  listenActivityAppended: vi.fn().mockImplementation((cb: () => void) => {
    capturedAppendedCb = cb;
    return Promise.resolve(() => { capturedAppendedCb = null; });
  }),
  listenAgentEvents: vi.fn().mockImplementation((cb: () => void) => {
    capturedAgentEventsCb = cb;
    return Promise.resolve(() => { capturedAgentEventsCb = null; });
  }),
}));

import { ActivitySurface } from './ActivitySurface';
import * as transport from '../../../transport';
import { ACTIVITY_REFRESH_DEBOUNCE_MS } from '../../../config/constants';

/** Local error-boundary probe: makes a render-time throw observable as DOM. */
class Probe extends React.Component<{ children: React.ReactNode }, { msg: string | null }> {
  state = { msg: null as string | null };
  static getDerivedStateFromError(err: Error) {
    return { msg: err.message };
  }
  render() {
    return this.state.msg ? <div data-testid="probe-error">{this.state.msg}</div> : this.props.children;
  }
}

describe('ActivitySurface null-safety', () => {
  beforeEach(() => {
    capturedAgentEventsCb = null;
    capturedAppendedCb = null;
    vi.mocked(transport.activityQuery).mockResolvedValue(null as any);
  });

  it('renders without crashing when activityQuery resolves null', async () => {
    const { container } = render(
      <Probe>
        <ActivitySurface pushToast={vi.fn()} />
      </Probe>,
    );
    // Flush mount effects + the awaited activityQuery() microtask chain so the
    // setRows(null) re-render (and any crash) has definitely happened before we assert.
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    // Before the guard, setRows(null) makes rows.map() throw "Cannot read properties of
    // null (reading 'map')", caught by the Probe → probe-error. After the guard, the
    // surface stays mounted and shows its empty state for the null result.
    expect(screen.queryByTestId('probe-error')).toBeNull();
    // (container.textContent avoids RTL's icon-split / <option> matcher quirks)
    expect(container.textContent).toContain('No Activity Logged');
  });

  it('re-queries once per burst of activity-appended and ignores other engine events', async () => {
    vi.useFakeTimers();
    try {
      const activityQueryMock = vi.mocked(transport.activityQuery);
      activityQueryMock.mockResolvedValue([]);
      render(<Probe><ActivitySurface pushToast={vi.fn()} /></Probe>);
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
      const afterMount = activityQueryMock.mock.calls.length;
      expect(afterMount).toBeGreaterThanOrEqual(1);
      expect(capturedAppendedCb).not.toBeNull();

      // Token and other engine frames no longer re-query.
      await act(async () => {
        capturedAgentEventsCb?.();
        capturedAgentEventsCb?.();
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(activityQueryMock.mock.calls.length).toBe(afterMount);

      // A burst of appended rows becomes one query after the debounce.
      await act(async () => {
        capturedAppendedCb!();
        capturedAppendedCb!();
        capturedAppendedCb!();
        await vi.advanceTimersByTimeAsync(ACTIVITY_REFRESH_DEBOUNCE_MS + 10);
      });
      expect(activityQueryMock.mock.calls.length).toBe(afterMount + 1);
    } finally {
      vi.useRealTimers();
    }
  });
});
