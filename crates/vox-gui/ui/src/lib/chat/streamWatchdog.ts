/**
 * Client-side stream watchdog & reconnect catch-up controller (`T3-GUI-PIPELINE-04`):
 * - `FIRST_EVENT_TIMEOUT_MS = 30_000`: fires `onFirstEventTimeout` if no stream event arrives within 30s of turn start.
 * - `STREAM_IDLE_TIMEOUT_MS = 120_000`: fires `onIdleTimeout` if no new stream chunk arrives for 120s after the first event.
 * - Tracks `lastSeq` and `charOffset` for reconnect catch-up via `vox_chat_stream_resume`.
 */

export const FIRST_EVENT_TIMEOUT_MS = 30_000;
export const STREAM_IDLE_TIMEOUT_MS = 120_000;

export interface StreamWatchdogCallbacks {
  readonly sessionId: string;
  readonly onFirstEventTimeout?: (sessionId: string) => void;
  readonly onIdleTimeout?: (sessionId: string, lastSeq: number, charOffset: number) => void;
  readonly onResumeCatchUp?: (
    sessionId: string,
    afterSeq: number,
    charOffset: number,
  ) => void | Promise<void>;
  readonly firstEventTimeoutMs?: number;
  readonly idleTimeoutMs?: number;
}

export interface StreamWatchdogHandle {
  readonly startTurn: () => void;
  readonly recordChunk: (seq: number, deltaChars: number) => void;
  readonly triggerReconnectCatchUp: () => Promise<void>;
  readonly stop: () => void;
  readonly getState: () => {
    readonly active: boolean;
    readonly receivedFirstEvent: boolean;
    readonly lastSeq: number;
    readonly charOffset: number;
  };
}

export function createStreamWatchdog(cb: StreamWatchdogCallbacks): StreamWatchdogHandle {
  const firstEventMs = cb.firstEventTimeoutMs ?? FIRST_EVENT_TIMEOUT_MS;
  const idleMs = cb.idleTimeoutMs ?? STREAM_IDLE_TIMEOUT_MS;

  let active = false;
  let receivedFirstEvent = false;
  let lastSeq = 0;
  let charOffset = 0;
  let firstTimer: ReturnType<typeof setTimeout> | null = null;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;

  const clearTimers = () => {
    if (firstTimer !== null) {
      clearTimeout(firstTimer);
      firstTimer = null;
    }
    if (idleTimer !== null) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
  };

  const armIdleTimer = () => {
    if (idleTimer !== null) {
      clearTimeout(idleTimer);
    }
    idleTimer = setTimeout(() => {
      if (!active) return;
      cb.onIdleTimeout?.(cb.sessionId, lastSeq, charOffset);
      void cb.onResumeCatchUp?.(cb.sessionId, lastSeq, charOffset);
    }, idleMs);
  };

  return {
    startTurn() {
      clearTimers();
      active = true;
      receivedFirstEvent = false;
      lastSeq = 0;
      charOffset = 0;
      firstTimer = setTimeout(() => {
        if (!active || receivedFirstEvent) return;
        cb.onFirstEventTimeout?.(cb.sessionId);
        void cb.onResumeCatchUp?.(cb.sessionId, lastSeq, charOffset);
      }, firstEventMs);
    },

    recordChunk(seq: number, deltaChars: number) {
      if (!active) return;
      receivedFirstEvent = true;
      if (firstTimer !== null) {
        clearTimeout(firstTimer);
        firstTimer = null;
      }
      if (seq > lastSeq) {
        lastSeq = seq;
      }
      if (deltaChars > 0) {
        charOffset += deltaChars;
      }
      armIdleTimer();
    },

    async triggerReconnectCatchUp() {
      if (!active) return;
      await cb.onResumeCatchUp?.(cb.sessionId, lastSeq, charOffset);
    },

    stop() {
      active = false;
      clearTimers();
    },

    getState() {
      return {
        active,
        receivedFirstEvent,
        lastSeq,
        charOffset,
      };
    },
  };
}
