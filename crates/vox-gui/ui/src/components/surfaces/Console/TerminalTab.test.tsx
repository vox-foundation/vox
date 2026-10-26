import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, cleanup, waitFor, screen } from '@testing-library/react';
import React from 'react';

const openMock = vi.fn().mockResolvedValue('s-1');
const attachMock = vi.fn().mockResolvedValue({
  session_id: 's-1',
  blocks: [
    {
      id: 1,
      kind: 'Shell',
      status: 'Ok',
      origin: { kind: 'user' },
      input: 'git status',
      output: [{ stream: 'Stdout', text: 'clean\n', ts_ms: 1 }],
      exit_code: 0,
      started_ms: 1,
      ended_ms: 2,
    },
  ],
  replay_utf8: '$ git status\r\nclean\r\n',
  seq: 3,
});
const inputMock = vi.fn().mockResolvedValue(undefined);
const submitMock = vi.fn().mockResolvedValue(2);
const resizeMock = vi.fn().mockResolvedValue(undefined);

vi.mock('../../../transport', () => ({
  termSessionOpen: (...a: unknown[]) => openMock(...a),
  termSessionAttach: (...a: unknown[]) => attachMock(...a),
  termSessionInput: (...a: unknown[]) => inputMock(...a),
  termSessionSubmit: (...a: unknown[]) => submitMock(...a),
  termSessionResize: (...a: unknown[]) => resizeMock(...a),
  listenTermOutput: vi.fn().mockResolvedValue(() => {}),
  listenTermEvent: vi.fn().mockResolvedValue(() => {}),
}));

const termWriteMock = vi.fn();
const loadedAddons: unknown[] = [];

vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    buffer = { active: { cursorY: 0, baseY: 0 } };
    open() {}
    write(data: string) {
      termWriteMock(data);
    }
    onData() {}
    dispose() {}
    loadAddon(addon: { activate?: (t: unknown) => void }) {
      loadedAddons.push(addon);
      addon.activate?.(this);
    }
    get cols() {
      return 80;
    }
    get rows() {
      return 24;
    }
  },
}));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }));

import { TerminalTab } from './TerminalTab';
import { WebglTerminalAddon, renderTermBlockForAgent } from './webglAddon';

describe('TerminalTab', () => {
  beforeEach(() => {
    cleanup();
    openMock.mockClear();
    attachMock.mockClear();
    inputMock.mockClear();
    submitMock.mockClear();
    resizeMock.mockClear();
    termWriteMock.mockClear();
    loadedAddons.length = 0;
  });

  it('exposes the host as a labeled application region', () => {
    render(<TerminalTab tabId="tab-1" pendingLine={null} />);
    const host = screen.getByLabelText('terminal');
    expect(host.getAttribute('role')).toBe('application');
  });

  it('opens and attaches a SessionHost session on mount and replays snapshot', async () => {
    const onBlocksChange = vi.fn();
    const onSessionResolved = vi.fn();
    render(
      <TerminalTab
        tabId="tab-1"
        pendingLine={null}
        onBlocksChange={onBlocksChange}
        onSessionResolved={onSessionResolved}
      />,
    );
    await waitFor(() => expect(openMock).toHaveBeenCalledWith(80, 24));
    await waitFor(() => expect(attachMock).toHaveBeenCalledWith('s-1'));
    expect(onSessionResolved).toHaveBeenCalledWith('s-1');
    expect(termWriteMock).toHaveBeenCalledWith('$ git status\r\nclean\r\n');
    expect(onBlocksChange).toHaveBeenCalledWith(
      expect.arrayContaining([expect.objectContaining({ id: 1, input: 'git status' })]),
    );
  });

  it('loads WebglTerminalAddon and disposes it gracefully on context loss', async () => {
    render(<TerminalTab tabId="tab-1" pendingLine={null} />);
    await waitFor(() => expect(attachMock).toHaveBeenCalled());
    const webgl = loadedAddons.find((a): a is WebglTerminalAddon => a instanceof WebglTerminalAddon);
    expect(webgl).toBeTruthy();
    expect(webgl!.isDisposed()).toBe(false);
    webgl!.simulateContextLoss();
    expect(webgl!.isDisposed()).toBe(true);
  });

  it('submits a pending line through termSessionSubmit', async () => {
    const { rerender } = render(<TerminalTab tabId="tab-1" pendingLine={null} />);
    await waitFor(() => expect(attachMock).toHaveBeenCalledWith('s-1'));
    rerender(<TerminalTab tabId="tab-1" pendingLine={{ text: 'ls', seq: 1 }} />);
    await waitFor(() => expect(submitMock).toHaveBeenCalledWith('s-1', 'ls'));
  });

  it('formats TermBlockDto for agent handoff via renderTermBlockForAgent', () => {
    const formatted = renderTermBlockForAgent(
      {
        id: 1,
        kind: 'Shell',
        status: 'Ok',
        input: 'cargo check',
        output: [{ stream: 'Stdout', text: 'Finished dev\n', ts_ms: 1 }],
        exit_code: 0,
        started_ms: 1,
        ended_ms: 2,
      },
      'fallback',
    );
    expect(formatted).toBe('$ cargo check\nFinished dev\n(exit 0)');
  });
});
