import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act, cleanup } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { LanguageProvider } from '../../../hooks/useLanguage';
import {
  onConsoleBlockFocus,
  publishTerminalSnapshot,
  resetTerminalBridgeForTests,
  type ConsoleFocusTarget,
} from '../../../lib/terminalBridge';
import type { TermAttachmentDto, TermBlockDto } from '../../../transport';

const { promoteBgMock, attachMock, openMock, listMock, invokeMock } = vi.hoisted(() => ({
  promoteBgMock: vi.fn().mockResolvedValue(true),
  attachMock: vi.fn<(...args: unknown[]) => Promise<TermAttachmentDto | null>>(),
  openMock: vi.fn().mockResolvedValue('s-dock'),
  listMock: vi.fn().mockResolvedValue([]),
  invokeMock: vi.fn().mockResolvedValue(null),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock('../../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../transport')>();
  return {
    ...actual,
    termSessionPromoteBg: (...args: unknown[]) => promoteBgMock(...args),
    termSessionAttach: (...args: unknown[]) => attachMock(...args),
    termSessionOpen: (...args: unknown[]) => openMock(...args),
    termSessionList: (...args: unknown[]) => listMock(...args),
    listenTermEvent: vi.fn(async () => () => {}),
    listenTermOutput: vi.fn(async () => () => {}),
    listenSecretaryProposed: vi.fn(async () => () => {}),
  };
});

import {
  ChatTerminalBlockCard,
  isTerminalBlockToolName,
  isTerminalToolName,
} from './ChatTerminalBlockCard';
import { ChatTurnEventRow } from './ChatTurnEventRow';
import { TurnTrace } from './TurnTrace';
import { ChatTranscript } from './ChatTranscript';
import { ChatSurface, TerminalDockPanel } from './ChatSurface';
import { resetSessionPaneAttachRegistryForTests } from '../Console/TerminalSessionPane';

describe('ChatTerminalBlockCard & Chat <-> Console unification (TERM-PANE-01)', () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
    resetTerminalBridgeForTests();
    resetSessionPaneAttachRegistryForTests();
    window.localStorage.clear();
    invokeMock.mockResolvedValue(null);
    listMock.mockResolvedValue([]);
    attachMock.mockResolvedValue({
      session_id: 's-dock',
      blocks: [],
      replay_utf8: '',
      seq: 1,
      alt_screen: false,
      cwd: '/Users/dev/vox',
      git_branch: 'main',
    });
  });

  afterEach(() => {
    resetTerminalBridgeForTests();
  });

  it('matches vox_terminal_* tool names without touching TURN_EVENT_KINDS', () => {
    expect(isTerminalToolName('vox_terminal_exec')).toBe(true);
    expect(isTerminalToolName('vox_terminal_read')).toBe(true);
    expect(isTerminalToolName('vox_terminal_send_input')).toBe(true);
    expect(isTerminalToolName('vox_terminal_sessions')).toBe(true);
    expect(isTerminalToolName('vox_git_status')).toBe(false);
    expect(isTerminalToolName(null)).toBe(false);
    expect(isTerminalBlockToolName('vox_terminal_exec')).toBe(true);
    expect(isTerminalBlockToolName('vox_terminal_send_input')).toBe(true);
    expect(isTerminalBlockToolName('vox_terminal_sessions')).toBe(false);
    expect(isTerminalBlockToolName('vox_terminal_read')).toBe(false);
  });

  it('renders live vox_terminal_* card inside collapsed TurnTrace with sgrToSpans ANSI formatting and zero raw escape or script leaks', () => {
    const agentBlock: TermBlockDto = { id: 7, kind: 'Shell', status: 'Ok', origin: { kind: 'agent', agent_id: 'a-1' }, input: 'cargo check -p vox-cli', output: [{ stream: 'Stdout', text: '\x1b[32mFinished\x1b[0m dev profile <script>window.__xss = 1</script>\n', ts_ms: 10 }], exit_code: 0, started_ms: 1, ended_ms: 15 };
    publishTerminalSnapshot({ activeSessionId: 's-42', blocks: [agentBlock] });
    const { container } = render(<TurnTrace events={[{ kind: 'tool_receipt', tool: 'vox_terminal_exec', receipt_id: '01920000-aaaa-7bbb-8ccc-000000000099', fulfilled: true, verified: true, block_id: 7 }]} verbosity="normal" />);

    const card = screen.getByTestId('chat-term-block-card');
    expect(card).toBeVisible();
    expect(card).toHaveTextContent('$ cargo check -p vox-cli');
    expect(card).toHaveTextContent('term:s-42');
    expect(card).toHaveTextContent('exit 0');
    expect(card).toHaveTextContent('Finished dev profile <script>window.__xss = 1</script>');
    expect(card.textContent).not.toContain('\x1b[32m');
    expect(container.querySelectorAll('script')).toHaveLength(0);
    const pre = screen.getByTestId('chat-term-block-output');
    expect(pre.style.fontFamily).toContain('JetBrains Mono');
  });

  it('promotes AwaitingInput blocks to background and dispatches Console focus + navigation on Open in Console', () => {
    const awaitingBlock: TermBlockDto = {
      id: 9,
      kind: 'Shell',
      status: 'AwaitingInput',
      origin: { kind: 'agent', agent_id: 'a-2' },
      input: 'pnpm init',
      output: [{ stream: 'Stdout', text: 'package name: ', ts_ms: 20 }],
      exit_code: null,
      started_ms: 10,
      ended_ms: null,
    };
    publishTerminalSnapshot({ activeSessionId: 's-9', blocks: [awaitingBlock] });

    const onNavigate = vi.fn();
    const focusTargets: ConsoleFocusTarget[] = [];
    const unsubFocus = onConsoleBlockFocus((t) => focusTargets.push(t));
    const navEvents: Array<CustomEvent> = [];
    const onSurfaceEvent = (ev: Event) => navEvents.push(ev as CustomEvent);
    window.addEventListener('vox://navigate-surface', onSurfaceEvent);

    try {
      render(
        <ChatTerminalBlockCard
          event={{ kind: 'tool_receipt', tool: 'vox_terminal_exec', block_id: 9 }}
          onNavigate={onNavigate}
        />,
      );

      const card = screen.getByTestId('chat-term-block-card');
      expect(card).toHaveTextContent('awaiting input');
      expect(card).toHaveTextContent('Process awaiting input');

      fireEvent.click(screen.getByRole('button', { name: 'Promote to Background' }));
      expect(promoteBgMock).toHaveBeenCalledWith('s-9', 9);

      fireEvent.click(screen.getByTestId('chat-term-open-console'));
      expect(focusTargets).toEqual([{ sessionId: 's-9', blockId: 9 }]);
      expect(onNavigate).toHaveBeenCalledWith('console');
      expect(navEvents).toHaveLength(1);
      expect(navEvents[0]?.detail).toEqual({ view: 'console', sessionId: 's-9', blockId: 9 });
    } finally {
      unsubFocus();
      window.removeEventListener('vox://navigate-surface', onSurfaceEvent);
    }
  });

  it('adds Open in Console inside ChatTurnEventRow for vox_terminal_* receipts while keeping missing-field guard intact', () => {
    publishTerminalSnapshot({ activeSessionId: 's-row' });
    const onNavigate = vi.fn();

    render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          tool: 'vox_terminal_read',
          receipt_id: '01920000-aaaa-7bbb-8ccc-000000000077',
          fulfilled: true,
          verified: true,
        }}
        onNavigate={onNavigate}
      />,
    );

    const row = screen.getByTestId('chat-turn-receipt-row');
    expect(row).toHaveTextContent('receipt · vox_terminal_read · verified');
    fireEvent.click(screen.getByRole('button', { name: 'Open in Console' }));
    expect(onNavigate).toHaveBeenCalledWith('console');

    cleanup();
    const { container } = render(
      <ChatTurnEventRow
        event={{
          kind: 'tool_receipt',
          tool: 'vox_terminal_read',
          fulfilled: true,
          verified: true,
        }}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('streams live agent terminal block inside ChatTranscript streaming assistant bubble before tool_receipt arrives', () => {
    const runningAgentBlock: TermBlockDto = { id: 15, kind: 'Shell', status: 'Running', origin: { kind: 'agent', agent_id: 'a-1' }, input: 'vitest run src/Chat', output: [{ stream: 'Stdout', text: '\x1b[36mRUN\x1b[0m v4.0\n', ts_ms: 5 }], exit_code: null, started_ms: 5, ended_ms: null };
    publishTerminalSnapshot({ activeSessionId: 's-live', blocks: [runningAgentBlock] });
    const onNavigate = vi.fn();
    render(<ChatTranscript sessionId="chat-1" onNavigate={onNavigate} messages={[{ id: 'm-stream', role: 'assistant', text: 'Executing verification suite...', status: 'streaming' }]} />);

    const card = screen.getByTestId('chat-term-block-card');
    expect(card).toHaveTextContent('$ vitest run src/Chat');
    expect(card).toHaveTextContent('running');
    expect(card).toHaveTextContent('RUN v4.0');

    act(() => {
      publishTerminalSnapshot({
        activeSessionId: 's-live',
        blocks: [
          {
            ...runningAgentBlock,
            output: [
              ...runningAgentBlock.output,
              { stream: 'Stdout', text: '\x1b[32mPASS\x1b[0m 4 suites\n', ts_ms: 15 },
            ],
          },
        ],
      });
    });

    expect(screen.getByTestId('chat-term-block-card')).toHaveTextContent('PASS 4 suites');
    fireEvent.click(screen.getByTestId('chat-term-open-console'));
    expect(onNavigate).toHaveBeenCalledWith('console');
  });

  it('mounts TerminalSessionPane compact inside TerminalDockPanel and never resurrects a closed terminal panel in ChatSurface', async () => {
    publishTerminalSnapshot({ activeSessionId: 's-dock' });
    const onNavigate = vi.fn();
    const mockApi = {
      width: 420,
      onDidDimensionsChange: vi.fn(() => ({ dispose: vi.fn() })),
    } as any;

    const { unmount } = render(
      <TerminalDockPanel
        api={mockApi}
        containerApi={{} as any}
        params={{ onNavigate }}
      />,
    );

    const dockRoot = screen.getByTestId('chat-dock-terminal');
    expect(dockRoot).toBeInTheDocument();
    expect(screen.getByTestId('terminal-session-pane')).toBeInTheDocument();
    expect(screen.getByTestId('block-stream-view')).toBeInTheDocument();
    expect(screen.getByLabelText('terminal')).toBeInTheDocument();
    expect(screen.getByLabelText('dock terminal input')).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('chat-term-open-console'));
    expect(onNavigate).toHaveBeenCalledWith('console');
    unmount();

    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const renderChat = (text: string) => (
      <LanguageProvider>
        <QueryClientProvider client={client}>
          <ChatSurface
            pushToast={vi.fn()}
            onNavigate={onNavigate}
            messages={text ? [{ id: 'u1', role: 'user', text, status: 'done' }] : []}
            composer={<div>composer</div>}
          />
        </QueryClientProvider>
      </LanguageProvider>
    );

    const { rerender } = render(renderChat(''));
    fireEvent.click(screen.getByRole('button', { name: /panels/i }));
    fireEvent.click(screen.getByRole('checkbox', { name: /^terminal$/i }));
    await screen.findByTestId('chat-dock-terminal');

    const termTab = screen
      .getAllByText('Terminal')
      .map((el) => el.closest('.dv-default-tab'))
      .find((el): el is HTMLElement => el !== null) as HTMLElement;
    fireEvent.click(termTab.querySelector('.dv-default-tab-action') as HTMLElement);
    await waitFor(() => expect(screen.queryByTestId('chat-dock-terminal')).toBeNull());

    act(() => {
      publishTerminalSnapshot({
        activeSessionId: 's-bg',
        blocks: [
          {
            id: 21,
            kind: 'Shell',
            status: 'Running',
            origin: { kind: 'agent', agent_id: 'a-9' },
            input: 'echo bg',
            output: [],
            exit_code: null,
            started_ms: 1,
            ended_ms: null,
          },
        ],
      });
    });
    rerender(renderChat('follow-up message'));

    expect(screen.queryByTestId('chat-dock-terminal')).toBeNull();
  });

  it('keeps historical unindexed receipts immutable, restricts TurnTrace cards to exec/send_input, and resolves existing daemon sessions in TerminalDockPanel (CR-04, WR-03)', async () => {
    const newerAgentBlock: TermBlockDto = {
      id: 99,
      kind: 'Shell',
      status: 'Running',
      origin: { kind: 'agent', agent_id: 'a-9' },
      input: 'cargo test -p future-turn',
      output: [{ stream: 'Stdout', text: 'future stdout\n', ts_ms: 99 }],
      exit_code: null,
      started_ms: 99,
      ended_ms: null,
    };
    publishTerminalSnapshot({ activeSessionId: 's-hist', blocks: [newerAgentBlock] });

    render(
      <TurnTrace
        events={[
          {
            kind: 'tool_receipt',
            tool: 'vox_terminal_exec',
            receipt_id: '01920000-aaaa-7bbb-8ccc-000000000001',
            fulfilled: true,
            verified: true,
          },
          {
            kind: 'tool_receipt',
            tool: 'vox_terminal_sessions',
            receipt_id: '01920000-aaaa-7bbb-8ccc-000000000002',
            fulfilled: true,
            verified: true,
          },
        ]}
        verbosity="verbose"
      />,
    );

    // Only vox_terminal_exec renders a ChatTerminalBlockCard; vox_terminal_sessions stays a receipt chip
    const cards = screen.getAllByTestId('chat-term-block-card');
    expect(cards).toHaveLength(1);
    expect(cards[0]).toHaveTextContent('$ vox_terminal_exec');
    expect(cards[0]).toHaveTextContent('completed');
    expect(cards[0]).not.toHaveTextContent('cargo test -p future-turn');
    expect(cards[0]).not.toHaveTextContent('future stdout');
    expect(screen.getAllByTestId('chat-turn-receipt-row').some((r) => r.textContent?.includes('vox_terminal_sessions'))).toBe(true);

    cleanup();
    // Streaming turn with vox_terminal_sessions receipt does NOT suppress live activeAgentBlock card
    render(
      <ChatTranscript
        sessionId="chat-2"
        messages={[
          {
            id: 'm-live-2',
            role: 'assistant',
            text: 'Checking sessions then running tests...',
            status: 'streaming',
            events: [
              {
                kind: 'tool_receipt',
                tool: 'vox_terminal_sessions',
                receipt_id: '01920000-aaaa-7bbb-8ccc-000000000003',
                fulfilled: true,
                verified: true,
              },
            ],
          },
        ]}
      />,
    );
    expect(screen.getByTestId('chat-term-block-card')).toHaveTextContent('$ cargo test -p future-turn');

    cleanup();
    // WR-03: TerminalDockPanel resolves termSessionList() before falling back to termSessionOpen()
    resetTerminalBridgeForTests();
    listMock.mockResolvedValueOnce([{ id: 's-existing', blocks: 1 }]);
    attachMock.mockResolvedValueOnce({
      session_id: 's-existing',
      blocks: [],
      replay_utf8: '',
      seq: 2,
    });
    render(
      <TerminalDockPanel
        api={{ width: 400, onDidDimensionsChange: vi.fn(() => ({ dispose: vi.fn() })) } as any}
        containerApi={{} as any}
        params={{}}
      />,
    );
    await waitFor(() => expect(listMock).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(attachMock).toHaveBeenCalledWith('s-existing'));
    expect(openMock).not.toHaveBeenCalled();
  });

  it('renders inline TerminalApprovalCard compact inside ChatTerminalBlockCard and TurnTrace with 4-action resolution (TERM-AGENT-03)', async () => {
    invokeMock.mockImplementation(async (cmd: string, args: unknown) => {
      if (cmd !== 'invoke_mcp_tool') return null;
      const req = args as { tool?: string };
      if (req.tool === 'vox_pending_approvals') {
        return { is_error: false, result: { success: true, data: { approvals: [{ approval_id: 'AP-chat-1', tool: 'vox_terminal_exec', summary: 'Co-Pilot review - L3 Workspace Write', requested_at_ms: 1000, session_id: 's-chat', command: 'cargo test -p vox-terminal-core', control_mode: 'co_pilot' }] } } };
      }
      if (req.tool === 'vox_resolve_approval') return { is_error: false, result: { success: true, data: { resolved: true } } };
      return null;
    });
    publishTerminalSnapshot({ activeSessionId: 's-chat', blocks: [] });
    render(<TurnTrace events={[{ kind: 'tool_receipt', tool: 'vox_terminal_exec', receipt_id: '01920000-aaaa-7bbb-8ccc-000000000044', session_id: 's-chat', command: 'cargo test -p vox-terminal-core', fulfilled: false, verified: false }]} verbosity="normal" />);
    const card = await screen.findByTestId('term-approval-card');
    expect(card).toBeVisible();
    expect(card.querySelector('[data-roman-wax-seal="true"]')?.textContent).toBe('CO-PILOT GATE');
    expect(screen.queryByTestId('term-approval-edit-input')).toBeNull();
    expect(screen.queryByTestId('term-approval-steer-input')).toBeNull();
    fireEvent.click(screen.getByTestId('term-approval-approve'));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('invoke_mcp_tool', expect.objectContaining({ tool: 'vox_resolve_approval', args: { approval_id: 'AP-chat-1', outcome: 'approved' } })));
  });

  it('never matches completed blocks to unrelated approvals and surfaces inline Edit & Run validation errors (WR-06)', async () => {
    const unrelatedPending = [
      {
        approval_id: 'AP-unrelated',
        tool: 'vox_terminal_exec',
        summary: 'Danger command',
        requested_at_ms: 2000,
        session_id: 's-chat',
        command: 'git reset --hard',
      },
    ];
    const completedOk: TermBlockDto = {
      id: 40,
      kind: 'Shell',
      status: 'Ok',
      origin: { kind: 'agent', agent_id: 'a-1' },
      input: '',
      output: [],
      exit_code: 0,
      started_ms: 10,
      ended_ms: 20,
    };

    const { rerender } = render(
      <ChatTerminalBlockCard
        event={{ kind: 'tool_receipt', tool: 'vox_terminal_exec', session_id: 's-chat', fulfilled: true, verified: true }}
        liveBlock={completedOk}
        pendingApprovals={unrelatedPending}
      />,
    );
    expect(screen.queryByTestId('term-approval-card')).toBeNull();

    rerender(
      <ChatTerminalBlockCard
        event={{ kind: 'tool_receipt', tool: 'vox_terminal_exec', session_id: 's-chat', command: 'cargo check', fulfilled: false }}
        pendingApprovals={unrelatedPending}
      />,
    );
    expect(screen.queryByTestId('term-approval-card')).toBeNull();

    invokeMock.mockResolvedValueOnce({
      is_error: true,
      result: { success: false, error: 'Edited command blocked by ExecPolicy: subshell forbidden.' },
    });
    rerender(
      <ChatTerminalBlockCard
        event={{ kind: 'tool_receipt', tool: 'vox_terminal_exec', session_id: 's-chat', command: 'git reset --hard', fulfilled: false }}
        pendingApprovals={unrelatedPending}
      />,
    );
    expect(screen.getByTestId('term-approval-card')).toBeVisible();
    fireEvent.click(screen.getByTestId('term-approval-edit-toggle'));
    fireEvent.change(screen.getByTestId('term-approval-edit-input'), {
      target: { value: 'git status && rm -rf /' },
    });
    fireEvent.click(screen.getByTestId('term-approval-edit-submit'));
    expect(await screen.findByTestId('term-approval-edit-error')).toHaveTextContent('ExecPolicy');
  });
});
