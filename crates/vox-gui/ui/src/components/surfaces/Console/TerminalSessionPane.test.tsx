import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor, act } from '@testing-library/react';
import React from 'react';
import type { TermAttachmentDto, TermEventPayload, TermOutputPayload } from '../../../transport';

const openMock = vi.fn().mockResolvedValue('s-1');
const attachMock = vi.fn<(...args: unknown[]) => Promise<TermAttachmentDto | null>>();
const inputMock = vi.fn().mockResolvedValue(undefined);
const submitMock = vi.fn().mockResolvedValue(1);
const resizeMock = vi.fn().mockResolvedValue(undefined);
const pinMock = vi.fn().mockResolvedValue(undefined);
const promoteBgMock = vi.fn().mockResolvedValue(true);
const setControlMock = vi.fn().mockResolvedValue(undefined);

let outputListeners: Array<(payload: TermOutputPayload) => void> = [];
let eventListeners: Array<(payload: TermEventPayload) => void> = [];

vi.mock('../../../transport', () => ({
  termSessionOpen: (...args: unknown[]) => openMock(...args),
  termSessionAttach: (...args: unknown[]) => attachMock(...args),
  termSessionInput: (...args: unknown[]) => inputMock(...args),
  termSessionSubmit: (...args: unknown[]) => submitMock(...args),
  termSessionResize: (...args: unknown[]) => resizeMock(...args),
  termSessionPin: (...args: unknown[]) => pinMock(...args),
  termSessionPromoteBg: (...args: unknown[]) => promoteBgMock(...args),
  termSessionSetControl: (...args: unknown[]) => setControlMock(...args),
  listenTermOutput: vi.fn(async (cb: (payload: TermOutputPayload) => void) => {
    outputListeners.push(cb);
    return () => {
      outputListeners = outputListeners.filter((fn) => fn !== cb);
    };
  }),
  listenTermEvent: vi.fn(async (cb: (payload: TermEventPayload) => void) => {
    eventListeners.push(cb);
    return () => {
      eventListeners = eventListeners.filter((fn) => fn !== cb);
    };
  }),
  voxTransport: {
    openLocator: vi.fn().mockResolvedValue({ action: 'opened' }),
  },
}));

let termConstructCount = 0;
const termWriteMock = vi.fn();

vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    buffer = { active: { cursorY: 0, baseY: 0, length: 0, getLine: () => undefined } };
    constructor() {
      termConstructCount += 1;
    }
    open(parent: HTMLElement) {
      if (parent && !parent.querySelector('.xterm-helper-textarea')) {
        const ta = document.createElement('textarea');
        ta.className = 'xterm-helper-textarea';
        parent.appendChild(ta);
      }
    }
    write(data: string) {
      termWriteMock(data);
    }
    reset() {}
    focus() {}
    onData() {}
    dispose() {}
    loadAddon(addon: { activate?: (t: unknown) => void }) {
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

vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit() {}
  },
}));

import {
  TerminalSessionPane,
  resetSessionPaneAttachRegistryForTests,
} from './TerminalSessionPane';

describe('TerminalSessionPane', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    resetSessionPaneAttachRegistryForTests();
    outputListeners = [];
    eventListeners = [];
    termConstructCount = 0;
    attachMock.mockResolvedValue({
      session_id: 's-1',
      blocks: [
        {
          id: 1,
          kind: 'Shell',
          status: 'Ok',
          origin: { kind: 'user' },
          input: 'cargo check',
          output: [{ stream: 'Stdout', text: '\x1b[32mFinished\x1b[0m dev profile\n', ts_ms: 10 }],
          exit_code: 0,
          started_ms: 5,
          ended_ms: 12,
          cwd: '/Users/dev/vox',
          git_branch: 'main',
        },
      ],
      replay_utf8: '$ cargo check\r\nFinished dev profile\r\n',
      seq: 7,
      alt_screen: false,
      cwd: '/Users/dev/vox',
      git_branch: 'main',
    });
  });

  it('attaches once with 1-arg call, suppresses helper textarea in Blocks mode, reduces events in-memory, and resyncs with afterSeq only on SnapshotResync or Reconnect', async () => {
    render(<TerminalSessionPane sessionId="s-1" />);

    await waitFor(() => expect(attachMock).toHaveBeenCalledTimes(1));
    expect(attachMock).toHaveBeenNthCalledWith(1, 's-1');

    expect(screen.getByTestId('block-stream-view')).toBeTruthy();
    const vtViewport = screen.getByLabelText('terminal');
    expect(vtViewport.getAttribute('data-testid')).toBe('terminal-viewport');

    const helperTa = vtViewport.querySelector<HTMLTextAreaElement>('.xterm-helper-textarea');
    expect(helperTa).toBeTruthy();
    expect(helperTa?.tabIndex).toBe(-1);
    expect(helperTa?.getAttribute('aria-hidden')).toBe('true');
    expect(screen.queryAllByRole('textbox')).toHaveLength(0);

    await waitFor(() => expect(eventListeners.length).toBeGreaterThan(0));

    act(() => {
      for (const cb of eventListeners) {
        cb({
          session_id: 's-1',
          event: {
            BlockOpened: {
              id: 2,
              kind: 'Shell',
              input: 'echo delta',
              origin: { kind: 'user' },
              cwd: '/Users/dev/vox/crates',
              git_branch: 'feat/pane',
            },
          },
        });
        cb({
          session_id: 's-1',
          event: {
            OutputAppended: {
              id: 2,
              chunk: { stream: 'Stdout', text: 'delta line\n', ts_ms: 20 },
            },
          },
        });
        cb({
          session_id: 's-1',
          event: {
            BlockClosed: { id: 2, exit_code: 0, status: 'Ok' },
          },
        });
        cb({
          session_id: 's-1',
          event: {
            BlockPinned: { id: 2, pinned: true },
          },
        });
        cb({
          session_id: 's-1',
          event: {
            CwdChanged: { cwd: '/Users/dev/vox/ui', git_branch: 'feat/split' },
          },
        });
      }
    });

    expect(await screen.findByText('delta line')).toBeTruthy();
    expect(screen.getByText('feat/split')).toBeTruthy();
    expect(attachMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      for (const cb of eventListeners) {
        cb({
          session_id: 's-1',
          event: {
            SnapshotResync: {
              blocks: [],
              resync_required: true,
              alt_screen: false,
            },
          },
        });
      }
    });

    // CR-02: SnapshotResync must NOT re-call termSessionAttach (pty.rs already re-attached)
    expect(attachMock).toHaveBeenCalledTimes(1);
    expect(screen.queryAllByTestId('term-block-card')).toHaveLength(0);
  });

  it('reconnects using afterSeq when initial attach fails and Reconnect Session is clicked', async () => {
    attachMock.mockRejectedValueOnce(new Error('daemon pipe closed'));
    render(<TerminalSessionPane sessionId="s-1" />);

    const reconnectBtn = await screen.findByRole('button', { name: /Reconnect Session/i });
    attachMock.mockResolvedValueOnce({
      session_id: 's-1',
      blocks: [],
      replay_utf8: '',
      seq: 4,
    });

    fireEvent.click(reconnectBtn);
    await waitFor(() => expect(attachMock).toHaveBeenCalledTimes(2));
    expect(attachMock).toHaveBeenNthCalledWith(2, 's-1', 0);
  });

  it('flips synchronously (<16 ms) into ALT_SCREEN TUI mode across all 3 sources without remounting Terminal', async () => {
    render(<TerminalSessionPane sessionId="s-1" />);
    await waitFor(() => expect(attachMock).toHaveBeenCalledWith('s-1'));
    await waitFor(() => expect(outputListeners.length).toBeGreaterThan(0));
    await waitFor(() => expect(eventListeners.length).toBeGreaterThan(0));

    expect(termConstructCount).toBe(1);
    expect(screen.queryByTestId('term-tui-active-banner')).toBeNull();

    const t0 = performance.now();
    act(() => {
      for (const cb of outputListeners) {
        cb({ session_id: 's-1', seq: 8, data: '\x1b[?1049h' });
      }
    });
    const elapsedMs = performance.now() - t0;

    expect(elapsedMs).toBeLessThan(50);
    expect(screen.getByTestId('term-tui-active-banner')).toBeTruthy();
    expect(screen.getByText('TUI Active - Return on exit')).toBeTruthy();
    expect(screen.getByTestId('terminal-vt-layer').className).toContain('opacity-100');
    expect(termConstructCount).toBe(1);

    // Override button returns to Blocks view temporarily if app leaves alt-screen dirty
    fireEvent.click(screen.getByRole('button', { name: 'Exit Raw View' }));
    expect(screen.queryByTestId('term-tui-active-banner')).toBeNull();
    expect(screen.getByTestId('terminal-vt-layer').className).toContain('opacity-0');

    // AltScreenChanged event re-enters and exits cleanly
    act(() => {
      for (const cb of eventListeners) {
        cb({ session_id: 's-1', event: { AltScreenChanged: { active: true } } });
      }
    });
    expect(screen.getByTestId('term-tui-active-banner')).toBeTruthy();

    act(() => {
      for (const cb of outputListeners) {
        cb({ session_id: 's-1', seq: 9, data: '\x1b[?1049l' });
      }
    });
    expect(screen.queryByTestId('term-tui-active-banner')).toBeNull();
    expect(termConstructCount).toBe(1);
  });

  it('opens TermSearchBar on Ctrl+F or Search Output click, counts/navigates matches across ANSI blocks, and unmounts to null on Escape', async () => {
    attachMock.mockResolvedValueOnce({
      session_id: 's-1',
      blocks: [
        {
          id: 1,
          kind: 'Shell',
          status: 'Ok',
          input: 'echo alpha',
          output: [{ stream: 'Stdout', text: '\x1b[31merror\x1b[0m one\n', ts_ms: 1 }],
          exit_code: 0,
          started_ms: 1,
          ended_ms: 2,
        },
        {
          id: 2,
          kind: 'Shell',
          status: 'Failed',
          input: 'echo beta',
          output: [{ stream: 'Stderr', text: 'second \x1b[33merror\x1b[0m line\n', ts_ms: 3 }],
          exit_code: 1,
          started_ms: 3,
          ended_ms: 4,
        },
      ],
      replay_utf8: '',
      seq: 2,
    });

    render(<TerminalSessionPane sessionId="s-1" />);
    await waitFor(() => expect(screen.getAllByTestId('term-block-card')).toHaveLength(2));

    expect(screen.queryByTestId('term-search-bar')).toBeNull();

    fireEvent.keyDown(screen.getByTestId('terminal-session-pane'), { key: 'f', ctrlKey: true });
    expect(screen.getByTestId('term-search-bar')).toBeTruthy();

    const searchInput = screen.getByTestId('term-search-input');
    fireEvent.change(searchInput, { target: { value: 'error' } });
    expect(screen.getByTestId('term-search-count').textContent).toBe('1 of 2');

    fireEvent.click(screen.getByTestId('term-search-next'));
    expect(screen.getByTestId('term-search-count').textContent).toBe('2 of 2');

    fireEvent.click(screen.getByTestId('term-search-prev'));
    expect(screen.getByTestId('term-search-count').textContent).toBe('1 of 2');

    fireEvent.change(searchInput, { target: { value: 'nonexistent-pattern' } });
    expect(screen.getByTestId('term-search-count').textContent).toBe(
      '0 of 0 matches - try another pattern or press Esc to clear',
    );

    fireEvent.keyDown(searchInput, { key: 'Escape' });
    expect(screen.queryByTestId('term-search-bar')).toBeNull();
    expect(document.activeElement).toBe(screen.getByTestId('terminal-session-pane'));

    // WR-05: In Raw VT mode, 0 VT matches must NOT fall back to hidden block matches
    fireEvent.click(screen.getByTestId('term-view-mode-toggle'));
    fireEvent.keyDown(screen.getByTestId('terminal-session-pane'), { key: 'f', ctrlKey: true });
    fireEvent.change(screen.getByTestId('term-search-input'), { target: { value: 'error' } });
    expect(screen.getByTestId('term-search-count').textContent).toBe(
      '0 of 0 matches - try another pattern or press Esc to clear',
    );
  });

  it('deduplicates concurrent initial termSessionAttach calls for the same sessionId and preempts Autopilot on user input', async () => {
    const onControlSync = vi.fn();
    render(
      <div>
        <TerminalSessionPane
          paneId="p-console"
          sessionId="s-1"
          controlMode="autopilot"
          onControlSyncFromBackend={onControlSync}
        />
        <TerminalSessionPane
          paneId="p-dock"
          compact
          sessionId="s-1"
          controlMode="autopilot"
        />
      </div>,
    );

    await waitFor(() => expect(attachMock).toHaveBeenCalledTimes(1));
    expect(attachMock).toHaveBeenCalledWith('s-1');

    const dockInput = screen.getByLabelText('dock terminal input');
    fireEvent.change(dockInput, { target: { value: 'ls' } });
    expect(setControlMock).toHaveBeenCalledWith('s-1', 'manual');
  });

  it('renders compact dock header with Open in Console CTA and inline dock command input', async () => {
    const onOpenConsole = vi.fn();
    render(<TerminalSessionPane compact sessionId="s-1" onOpenConsole={onOpenConsole} />);
    await waitFor(() => expect(attachMock).toHaveBeenCalledWith('s-1'));

    const openBtn = screen.getByTestId('chat-term-open-console');
    expect(openBtn.textContent).toContain('Open in Console');
    fireEvent.click(openBtn);
    expect(onOpenConsole).toHaveBeenCalledTimes(1);

    const dockInput = screen.getByLabelText('dock terminal input');
    fireEvent.change(dockInput, { target: { value: 'vox doctor' } });
    fireEvent.submit(dockInput.closest('form')!);
    await waitFor(() => expect(submitMock).toHaveBeenCalledWith('s-1', 'vox doctor'));
  });
});
