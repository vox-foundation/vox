// @vitest-environment jsdom
import { act, render } from '@testing-library/react';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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
}));

vi.mock('../../transport', () => ({
  voxTransport: { listModels: vi.fn().mockResolvedValue([]) },
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

import { AxisDriveHost } from './AxisDriveHost';

describe('AxisDriveHost agent-event bridge', () => {
  beforeEach(() => {
    mocks.agentHandler = null;
    mocks.driveHandler = null;
    mocks.invoke.mockReset();
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'get_drive_mode') return 'live';
      if (command === 'inference_provider_status') return [];
      return undefined;
    });
  });

  it('records kind.text token frames under the active turn before submit resolves', async () => {
    let resolveSubmit: ((result: { ok: true }) => void) | undefined;
    const submit = vi.fn(() => new Promise<{ ok: true }>(resolve => {
      resolveSubmit = resolve;
    }));

    render(
      <AxisDriveHost
        setters={{ setChatModelOverride: vi.fn() }}
        onSubmit={submit}
        sessionReady
      />,
    );
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(mocks.driveHandler).not.toBeNull();
    expect(mocks.agentHandler).not.toBeNull();

    let request: Promise<void> | undefined;
    await act(async () => {
      request = mocks.driveHandler!({
        payload: { id: 'req-1', verb: 'send', body: { text: 'hello' } },
      });
      await Promise.resolve();
    });
    expect(submit).toHaveBeenCalledOnce();

    mocks.agentHandler!({
      id: 7,
      timestamp_ms: 123,
      kind: {
        type: 'token_streamed',
        text: 'ab',
        session_id: 'sess',
      },
    });

    await act(async () => {
      resolveSubmit!({ ok: true });
      await request;
    });

    const respondCall = mocks.invoke.mock.calls.find(([command]) => command === 'drive_respond');
    expect(respondCall).toBeDefined();
    const body = JSON.parse(
      (respondCall![1] as { args: { body: string } }).args.body,
    ) as { state: { last_turn_id: string; events: Array<{ kind: string; text?: string; turn_id: string }> } };
    const token = body.state.events.find(event => event.kind === 'token_streamed');
    expect(token).toMatchObject({ text: 'ab', turn_id: body.state.last_turn_id });

    mocks.agentHandler!({
      id: 8,
      timestamp_ms: 124,
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
