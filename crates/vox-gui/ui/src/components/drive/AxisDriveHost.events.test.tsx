// @vitest-environment jsdom
import { act, render } from '@testing-library/react';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { emptyLiveState, type DriveState } from '../../lib/axisDrive';
import { MODEL_LIST_LIMIT } from '../../config/constants';

interface AgentFrame {
  id: number;
  timestamp_ms: number;
  kind: { type: string; text?: string; session_id?: string };
}

interface DriveRequestEvent {
  payload: {
    id: string;
    verb: 'send' | 'state';
    body: { text?: string };
  };
}

const mocks = vi.hoisted(() => ({
  agentHandler: null as ((frame: AgentFrame) => void) | null,
  driveHandler: null as ((event: DriveRequestEvent) => Promise<void>) | null,
  invoke: vi.fn(),
  listModels: vi.fn().mockResolvedValue([]),
}));

vi.mock('../../transport', () => ({
  voxTransport: { listModels: mocks.listModels },
  listenAgentEvents: vi.fn().mockImplementation(
    async (handler: (frame: AgentFrame) => void) => {
      mocks.agentHandler = handler;
      return () => {
        mocks.agentHandler = null;
      };
    },
  ),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockImplementation(
    async (_event: string, handler: (event: DriveRequestEvent) => Promise<void>) => {
      mocks.driveHandler = handler;
      return () => {
        mocks.driveHandler = null;
      };
    },
  ),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mocks.invoke,
}));

import { AxisDriveHost, attachDriveAgentListener } from './AxisDriveHost';

describe('AxisDriveHost agent-event bridge', () => {
  beforeEach(() => {
    mocks.agentHandler = null;
    mocks.driveHandler = null;
    mocks.listModels.mockClear();
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'get_drive_mode') return 'live';
      if (command === 'inference_provider_status') return [];
      if (command === 'orchestrator_daemon_ready') return true;
      return undefined;
    });
  });

  it('attachDriveAgentListener drops late frames after cancel (StrictMode)', async () => {
    const stateRef = { current: emptyLiveState() as DriveState };
    const activeTurnIdRef = { current: 'turn-1' as string | null };
    const stop = attachDriveAgentListener(stateRef, activeTurnIdRef, {
      sessionId: 'sess-a',
    });
    // Simulate async listen resolving then immediate cancel (StrictMode remount).
    stop();
    await act(async () => {
      await Promise.resolve();
    });
    expect(mocks.agentHandler).toBeNull();
  });

  it('double attach: cancelled listener cannot record after remount', async () => {
    const stateRef = { current: emptyLiveState() as DriveState };
    const activeTurnIdRef = { current: 'turn-1' as string | null };
    const stop1 = attachDriveAgentListener(stateRef, activeTurnIdRef);
    await act(async () => {
      await Promise.resolve();
    });
    const firstHandler = mocks.agentHandler;
    stop1();
    const stop2 = attachDriveAgentListener(stateRef, activeTurnIdRef);
    await act(async () => {
      await Promise.resolve();
    });
    expect(mocks.agentHandler).not.toBe(firstHandler);
    mocks.agentHandler!({
      id: 1,
      timestamp_ms: 1,
      kind: { type: 'token_streamed', text: 'ok' },
    });
    expect(stateRef.current.events.some(e => e.text === 'ok')).toBe(true);
    stop2();
  });

  it('filters agent frames whose session_id does not match', async () => {
    const stateRef = { current: emptyLiveState() as DriveState };
    const activeTurnIdRef = { current: 'turn-1' as string | null };
    attachDriveAgentListener(stateRef, activeTurnIdRef, { sessionId: 'sess-a' });
    await act(async () => {
      await Promise.resolve();
    });
    expect(mocks.agentHandler).toBeTruthy();
    mocks.agentHandler!({
      id: 1,
      timestamp_ms: 1,
      kind: { type: 'token_streamed', text: 'skip', session_id: 'other' },
    });
    expect(stateRef.current.events).toHaveLength(0);
    mocks.agentHandler!({
      id: 2,
      timestamp_ms: 2,
      kind: { type: 'token_streamed', text: 'keep', session_id: 'sess-a' },
    });
    expect(stateRef.current.events.some(e => e.text === 'keep')).toBe(true);
  });

  it('loadModels uses MODEL_LIST_LIMIT on set/send; state skips catalog IPC', async () => {
    render(
      <AxisDriveHost
        setters={{ setChatModelOverride: vi.fn() }}
        onSubmit={vi.fn().mockResolvedValue({ ok: true })}
        sessionReady
      />,
    );
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    await act(async () => {
      await mocks.driveHandler!({
        payload: { id: 'req-1', verb: 'state', body: {} },
      });
    });
    expect(mocks.listModels).not.toHaveBeenCalled();
    await act(async () => {
      await mocks.driveHandler!({
        payload: {
          id: 'req-2',
          verb: 'set',
          body: { model_override: 'openrouter/auto' },
        },
      });
    });
    expect(mocks.listModels).toHaveBeenCalledWith(MODEL_LIST_LIMIT);
  });

  it('stamps orch_fresh on drive responses when the daemon is ready', async () => {
    render(
      <AxisDriveHost
        setters={{ setChatModelOverride: vi.fn() }}
        onSubmit={vi.fn().mockResolvedValue({ ok: true })}
        sessionReady
      />,
    );
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    await act(async () => {
      await mocks.driveHandler!({
        payload: { id: 'req-1', verb: 'state', body: {} },
      });
    });
    const responds = mocks.invoke.mock.calls.filter(([command]) => command === 'drive_respond');
    expect(responds.length).toBeGreaterThan(0);
    const body = JSON.parse((responds.at(-1)![1] as { args: { body: string } }).args.body);
    expect(body.state.orch_fresh).toBe(true);
  });

  it('records agent frames only while a send turn is active', async () => {
    render(
      <AxisDriveHost
        setters={{ setChatModelOverride: vi.fn() }}
        onSubmit={vi.fn().mockResolvedValue({ ok: true, text: 'hi' })}
        sessionReady
      />,
    );
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    await act(async () => {
      await mocks.driveHandler!({
        payload: { id: 'req-1', verb: 'send', body: { text: 'ping' } },
      });
    });

    const sendResponds = mocks.invoke.mock.calls.filter(([command]) => command === 'drive_respond');
    const body = JSON.parse(
      (sendResponds.at(-1)![1] as { args: { body: string } }).args.body,
    ) as { state: { last_turn_id: string; events: Array<{ kind: string; text?: string; turn_id: string }> } };
    const token = body.state.events.find(e => e.kind === 'token_streamed');
    // No frames recorded during this send because we did not emit mid-flight.
    expect(token).toBeUndefined();
    expect(body.state.last_turn_id).toBeTruthy();

    // After send completes, activeTurnId is cleared — late frames must not stick.
    mocks.agentHandler?.({
      id: 99,
      timestamp_ms: Date.now(),
      kind: {
        type: 'token_streamed',
        text: 'late',
        session_id: 'sess',
      },
    });

    await act(async () => {
      await mocks.driveHandler!({
        payload: { id: 'req-2', verb: 'state', body: {} },
      });
    });
    const stateResponds = mocks.invoke.mock.calls.filter(([command]) => command === 'drive_respond');
    const stateBody = JSON.parse(
      (stateResponds.at(-1)![1] as { args: { body: string } }).args.body,
    ) as { state: { events: Array<{ kind: string; text?: string }> } };
    expect(stateBody.state.events.some(event => event.kind === 'token_streamed' && event.text === 'late')).toBe(false);
  });
});
