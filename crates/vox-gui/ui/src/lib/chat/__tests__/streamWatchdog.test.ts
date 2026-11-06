import { describe, expect, it, vi } from 'vitest';
import {
  createStreamWatchdog,
  FIRST_EVENT_TIMEOUT_MS,
  STREAM_IDLE_TIMEOUT_MS,
} from '../streamWatchdog';

describe('streamWatchdog (T3-GUI-PIPELINE-04)', () => {
  it('exposes canonical 30s first-event and 120s idle timeout constants', () => {
    expect(FIRST_EVENT_TIMEOUT_MS).toBe(30_000);
    expect(STREAM_IDLE_TIMEOUT_MS).toBe(120_000);
  });

  it('fires onFirstEventTimeout and onResumeCatchUp when no chunk arrives before firstEventTimeoutMs', async () => {
    vi.useFakeTimers();
    try {
      const onFirst = vi.fn();
      const onResume = vi.fn();
      const wd = createStreamWatchdog({
        sessionId: 'sess-wd-1',
        firstEventTimeoutMs: 50,
        idleTimeoutMs: 200,
        onFirstEventTimeout: onFirst,
        onResumeCatchUp: onResume,
      });
      wd.startTurn();
      vi.advanceTimersByTime(60);
      expect(onFirst).toHaveBeenCalledWith('sess-wd-1');
      expect(onResume).toHaveBeenCalledWith('sess-wd-1', 0, 0);
      wd.stop();
    } finally {
      vi.useRealTimers();
    }
  });

  it('cancels first-event timeout on chunk arrival and fires onIdleTimeout with lastSeq and charOffset', () => {
    vi.useFakeTimers();
    try {
      const onFirst = vi.fn();
      const onIdle = vi.fn();
      const wd = createStreamWatchdog({
        sessionId: 'sess-wd-2',
        firstEventTimeoutMs: 50,
        idleTimeoutMs: 100,
        onFirstEventTimeout: onFirst,
        onIdleTimeout: onIdle,
      });
      wd.startTurn();
      vi.advanceTimersByTime(25);
      wd.recordChunk(3, 42);
      vi.advanceTimersByTime(40);
      expect(onFirst).not.toHaveBeenCalled();

      vi.advanceTimersByTime(70);
      expect(onIdle).toHaveBeenCalledWith('sess-wd-2', 3, 42);
      wd.stop();
    } finally {
      vi.useRealTimers();
    }
  });
});
