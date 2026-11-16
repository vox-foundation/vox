import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, cleanup, waitFor, screen, act } from '@testing-library/react';
import React from 'react';
import type { TermEventPayload, TermOutputPayload } from '../../../transport';

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

let outputListeners: Array<(payload: TermOutputPayload) => void> = [];
let eventListeners: Array<(payload: TermEventPayload) => void> = [];

vi.mock('../../../transport', () => ({
  termSessionOpen: (...a: unknown[]) => openMock(...a),
  termSessionAttach: (...a: unknown[]) => attachMock(...a),
  termSessionInput: (...a: unknown[]) => inputMock(...a),
  termSessionSubmit: (...a: unknown[]) => submitMock(...a),
  termSessionResize: (...a: unknown[]) => resizeMock(...a),
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

const termWriteMock = vi.fn();
const termResetMock = vi.fn();
const termFocusMock = vi.fn();
const fitMock = vi.fn();
const loadedAddons: unknown[] = [];
let lastTermOptions: Record<string, unknown> | undefined;
let lastTermInstance: { options: Record<string, unknown> } | undefined;
let onDataListeners: Array<(data: string) => void> = [];

vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    options: Record<string, unknown>;
    buffer = { active: { cursorY: 0, baseY: 0, length: 0, getLine: () => undefined } };
    constructor(opts?: Record<string, unknown>) {
      this.options = { ...(opts ?? {}) };
      lastTermOptions = opts;
      lastTermInstance = this;
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
    reset() {
      termResetMock();
    }
    focus() {
      termFocusMock();
    }
    onData(cb: (data: string) => void) {
      onDataListeners.push(cb);
      return { dispose() {} };
    }
    onResize() {
      return { dispose() {} };
    }
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
vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit() {
      fitMock();
    }
    activate() {}
    dispose() {}
  },
}));

import { TerminalTab } from './TerminalTab';
import { WebglTerminalAddon, renderTermBlockForAgent } from './webglAddon';
import {
  SearchTerminalAddon,
  WebLinksTerminalAddon,
  Unicode11TerminalAddon,
  SerializeTerminalAddon,
} from './xtermAddons';
import {
  ARCANE_ANSI_16,
  HIGH_CONTRAST_ANSI_16,
  SLATE_ANSI_16,
  TERMINAL_MONO_STYLE,
  TRAVERTINE_ANSI_16,
  XTERM_FONT_FAMILY,
  resolveAnsi16Palette,
  resolveTerminalTheme,
  resolveTerminalWellSpec,
} from './terminalTheme';
import { computeContrastRatio } from '../../../lib/themeContract';

describe('TerminalTab', () => {
  beforeEach(() => {
    cleanup();
    openMock.mockClear();
    attachMock.mockClear();
    inputMock.mockClear();
    submitMock.mockClear();
    resizeMock.mockClear();
    termWriteMock.mockClear();
    termResetMock.mockClear();
    termFocusMock.mockClear();
    fitMock.mockClear();
    loadedAddons.length = 0;
    outputListeners = [];
    eventListeners = [];
    onDataListeners = [];
    lastTermOptions = undefined;
    lastTermInstance = undefined;
    delete (window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__;
    delete (window as unknown as { __VOX_E2E_REAL_XTERM__?: unknown }).__VOX_E2E_REAL_XTERM__;
    document.documentElement.dataset.theme = 'arcane';
  });

  it('exposes the host as a labeled application region and loads all 6 addons with proposed API', () => {
    render(<TerminalTab tabId="tab-1" pendingLine={null} />);
    const host = screen.getByLabelText('terminal');
    expect(host.getAttribute('role')).toBe('application');
    expect(host.getAttribute('data-testid')).toBe('terminal-viewport');
    expect(lastTermOptions).toMatchObject({
      allowProposedApi: true,
      convertEol: true,
      cursorBlink: false,
      fontFamily: XTERM_FONT_FAMILY,
      fontSize: 12,
    });
    expect(loadedAddons.some((a) => a instanceof WebglTerminalAddon)).toBe(true);
    expect(loadedAddons.some((a) => a instanceof SearchTerminalAddon)).toBe(true);
    expect(loadedAddons.some((a) => a instanceof WebLinksTerminalAddon)).toBe(true);
    expect(loadedAddons.some((a) => a instanceof Unicode11TerminalAddon)).toBe(true);
    expect(loadedAddons.some((a) => a instanceof SerializeTerminalAddon)).toBe(true);
    expect(loadedAddons).toHaveLength(6);
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

  it('loads WebglTerminalAddon, disposes it gracefully, and notifies onWebglFallback on context loss', async () => {
    const onWebglFallback = vi.fn();
    render(<TerminalTab tabId="tab-1" pendingLine={null} onWebglFallback={onWebglFallback} />);
    await waitFor(() => expect(attachMock).toHaveBeenCalled());
    const webgl = loadedAddons.find((a): a is WebglTerminalAddon => a instanceof WebglTerminalAddon);
    expect(webgl).toBeTruthy();
    expect(webgl!.isDisposed()).toBe(false);
    webgl!.simulateContextLoss();
    expect(webgl!.isDisposed()).toBe(true);
    expect(onWebglFallback).toHaveBeenCalledOnce();
  });

  it('submits pendingLine ONCE without post-submit termSessionAttach or replay_utf8 re-write storms', async () => {
    const { rerender } = render(<TerminalTab tabId="tab-1" pendingLine={null} />);
    await waitFor(() => expect(attachMock).toHaveBeenCalledTimes(1));
    expect(termWriteMock).toHaveBeenCalledTimes(1);

    rerender(<TerminalTab tabId="tab-1" pendingLine={{ text: 'ls', seq: 1 }} />);
    await waitFor(() => expect(submitMock).toHaveBeenCalledWith('s-1', 'ls'));
    expect(submitMock).toHaveBeenCalledTimes(1);
    expect(attachMock).toHaveBeenCalledTimes(1);
    expect(termWriteMock).toHaveBeenCalledTimes(1);
  });

  it('detects <16ms ALT_SCREEN escape transitions on listenTermOutput synchronously', async () => {
    const onAltScreenHint = vi.fn();
    render(
      <TerminalTab
        tabId="tab-1"
        sessionId="s-1"
        pendingLine={null}
        onAltScreenHint={onAltScreenHint}
      />,
    );
    await waitFor(() => expect(outputListeners.length).toBeGreaterThan(0));

    act(() => {
      for (const cb of outputListeners) {
        cb({ session_id: 's-1', seq: 4, data: '\x1b[?1049hTUI frame' });
      }
    });
    expect(onAltScreenHint).toHaveBeenLastCalledWith(true);

    act(() => {
      for (const cb of outputListeners) {
        cb({ session_id: 's-1', seq: 5, data: 'done\x1b[?1049l$ ' });
      }
    });
    expect(onAltScreenHint).toHaveBeenLastCalledWith(false);
  });

  it('suppresses .xterm-helper-textarea in Blocks mode and honors skipInitialAttach + replaySnapshot', async () => {
    const { rerender } = render(
      <TerminalTab
        tabId="tab-pane"
        sessionId="s-ext"
        pendingLine={null}
        skipInitialAttach={true}
        suppressHelperTextarea={true}
        replaySnapshot={{ text: '$ echo warm\r\nwarm\r\n', epoch: 1 }}
      />,
    );

    const host = screen.getByTestId('terminal-viewport');
    const helperTa = host.querySelector<HTMLTextAreaElement>('.xterm-helper-textarea');
    expect(helperTa).toBeTruthy();
    expect(helperTa!.tabIndex).toBe(-1);
    expect(helperTa!.getAttribute('aria-hidden')).toBe('true');
    expect(openMock).not.toHaveBeenCalled();
    expect(attachMock).not.toHaveBeenCalled();
    expect(termWriteMock).toHaveBeenCalledWith('$ echo warm\r\nwarm\r\n');

    rerender(
      <TerminalTab
        tabId="tab-pane"
        sessionId="s-ext"
        pendingLine={null}
        skipInitialAttach={true}
        suppressHelperTextarea={false}
        active={true}
        replaySnapshot={{ text: '$ echo warm\r\nwarm\r\n', epoch: 1 }}
      />,
    );
    expect(helperTa!.tabIndex).toBe(0);
    expect(helperTa!.getAttribute('aria-hidden')).toBeNull();
    expect(termFocusMock).toHaveBeenCalled();
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

  it('deduplicates listenTermOutput by seq, advances onOutputSeq, resets on initial replaySnapshot, and clears empty mockReplay', async () => {
    const onOutputSeq = vi.fn();
    const onUserInput = vi.fn();
    render(
      <TerminalTab
        tabId="tab-seq"
        sessionId="s-seq"
        skipInitialAttach={true}
        replaySnapshot={{ text: 'initial\r\n', epoch: 1, seq: 10 }}
        onOutputSeq={onOutputSeq}
        onUserInput={onUserInput}
      />,
    );

    await waitFor(() => expect(outputListeners.length).toBeGreaterThan(0));
    expect(termResetMock).toHaveBeenCalledTimes(1);
    expect(termWriteMock).toHaveBeenCalledWith('initial\r\n');
    termWriteMock.mockClear();

    act(() => {
      for (const cb of outputListeners) {
        cb({ session_id: 's-seq', seq: 9, data: 'stale' });
        cb({ session_id: 's-seq', seq: 10, data: 'duplicate' });
        cb({ session_id: 's-seq', seq: 11, data: 'fresh-11' });
        cb({ session_id: 's-seq', seq: 11, data: 'dup-11' });
      }
    });

    expect(termWriteMock).toHaveBeenCalledTimes(1);
    expect(termWriteMock).toHaveBeenCalledWith('fresh-11');
    expect(onOutputSeq).toHaveBeenCalledTimes(1);
    expect(onOutputSeq).toHaveBeenCalledWith(11);

    act(() => {
      for (const cb of onDataListeners) cb('a');
    });
    expect(onUserInput).toHaveBeenCalledTimes(1);
    expect(inputMock).toHaveBeenCalledWith('s-seq', 'a');

    cleanup();
    (window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__ = [];
    const { rerender } = render(
      <TerminalTab
        tabId="tab-mock"
        sessionId="s-mock"
        skipInitialAttach={true}
        replaySnapshot={{ text: 'populated-session', epoch: 1 }}
      />,
    );
    expect(screen.getByTestId('terminal-viewport').textContent).toContain('populated-session');

    rerender(
      <TerminalTab
        tabId="tab-mock"
        sessionId="s-mock-empty"
        skipInitialAttach={true}
        replaySnapshot={{ text: '', epoch: 2 }}
      />,
    );
    expect(screen.getByTestId('terminal-viewport').textContent).not.toContain('populated-session');
  });

  it('updates xterm theme on documentElement [data-theme] mutation', async () => {
    document.documentElement.dataset.theme = 'slate';
    render(<TerminalTab tabId="tab-theme" sessionId="s-theme" skipInitialAttach={true} />);
    const initialTheme = lastTermInstance?.options.theme as { brightCyan?: string; background?: string };
    expect(initialTheme?.brightCyan).toBe('#7dd3fc');
    expect(initialTheme?.background).toBe('#0c0e10');

    await act(async () => {
      document.documentElement.dataset.theme = 'arcane';
      await Promise.resolve();
    });
    await waitFor(() => {
      const nextTheme = lastTermInstance?.options.theme as { brightCyan?: string; background?: string };
      expect(nextTheme?.brightCyan).toBe('#8ad8db');
      expect(nextTheme?.background).toBe('#15110d');
    });
  });

  it('calibrates all 64 ANSI palette colors to WCAG AA/AAA against Dual-Surface wellBg with zero banned neon RGB collisions', () => {
    expect(XTERM_FONT_FAMILY).toBe("'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace");
    expect(TERMINAL_MONO_STYLE.fontFamily).toBe(
      "var(--font-terminal-mono, 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace)",
    );
    const bannedNeons = new Set(['#34d399', '#6ee7b7', '#22d3ee', '#06b6d4', '#10b981']);
    const checks = [
      { pref: 'arcane', expectedBg: '#15110d', minRatio: 4.5, expected: ARCANE_ANSI_16 },
      { pref: 'travertine', expectedBg: '#171512', minRatio: 4.5, expected: TRAVERTINE_ANSI_16 },
      { pref: 'glacier', expectedBg: '#171512', minRatio: 4.5, expected: TRAVERTINE_ANSI_16 },
      { pref: 'slate', expectedBg: '#0c0e10', minRatio: 4.5, expected: SLATE_ANSI_16 },
      { pref: 'void', expectedBg: '#0c0e10', minRatio: 4.5, expected: SLATE_ANSI_16 },
      { pref: 'high-contrast', expectedBg: '#000000', minRatio: 7.0, expected: HIGH_CONTRAST_ANSI_16 },
    ] as const;
    for (const c of checks) {
      const theme = resolveTerminalTheme(c.pref);
      const spec = resolveTerminalWellSpec(c.pref);
      const palette = resolveAnsi16Palette(c.pref);
      expect(theme.background).toBe(c.expectedBg);
      expect(spec.cursorAccent).toBe(c.expectedBg);
      expect(palette).toEqual(c.expected);
      for (const [slot, hex] of Object.entries(palette)) {
        expect(bannedNeons.has(hex.toLowerCase()), `${c.pref}.${slot} neon leak ${hex}`).toBe(false);
        expect(
          computeContrastRatio(hex, c.expectedBg),
          `${c.pref}.${slot} (${hex} on ${c.expectedBg})`,
        ).toBeGreaterThanOrEqual(c.minRatio);
      }
    }
  });

  it('marks terminal-viewport with data-terminal-well + data-terminal-mono and honors __VOX_E2E_REAL_XTERM__ with cursorBlink: false', () => {
    (window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__ = [];
    const { unmount } = render(<TerminalTab tabId="tab-mock-well" sessionId="s-1" skipInitialAttach={true} />);
    const mockViewport = screen.getByTestId('terminal-viewport');
    expect(mockViewport.getAttribute('data-terminal-well')).toBe('true');
    expect(mockViewport.getAttribute('data-terminal-mono')).toBe('true');
    expect(lastTermOptions).toBeUndefined();
    unmount();

    (window as unknown as { __VOX_E2E_REAL_XTERM__?: boolean }).__VOX_E2E_REAL_XTERM__ = true;
    render(<TerminalTab tabId="tab-real-e2e" sessionId="s-1" skipInitialAttach={true} />);
    const liveViewport = screen.getByTestId('terminal-viewport');
    expect(liveViewport.getAttribute('data-terminal-well')).toBe('true');
    expect(liveViewport.getAttribute('data-terminal-mono')).toBe('true');
    expect(lastTermOptions).toMatchObject({ cursorBlink: false });
  });
});
