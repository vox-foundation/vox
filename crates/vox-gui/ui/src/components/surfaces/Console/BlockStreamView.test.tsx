import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import { BlockStreamView } from './BlockStreamView';
import type { TermBlockDto } from '../../../transport';

const sampleBlocks: TermBlockDto[] = [
  { id: 1, kind: 'Shell', status: 'Ok', origin: { kind: 'user' }, input: 'cargo check', output: [{ stream: 'Stdout', text: 'Finished dev\n', ts_ms: 100 }], exit_code: 0, started_ms: 100, ended_ms: 350 },
  { id: 2, kind: 'Shell', status: 'Failed', origin: { kind: 'agent', agent_id: 'agent-7' }, input: 'cargo test -p broken', output: [{ stream: 'Stderr', text: 'error: test failed\n', ts_ms: 400 }], exit_code: 101, started_ms: 400, ended_ms: 900 },
];

describe('BlockStreamView', () => {
  beforeEach(() => cleanup());

  it('renders block cards with origin badges, exit codes, duration, and output', () => {
    render(<BlockStreamView blocks={sampleBlocks} pinnedBlockIds={[1]} onCopyCommand={vi.fn()} onCopyOutput={vi.fn()} onRerun={vi.fn()} onPin={vi.fn()} onAskAgent={vi.fn()} />);
    const cards = screen.getAllByTestId('term-block-card');
    expect(cards).toHaveLength(2);
    expect(screen.getByText('$ cargo check')).toBeTruthy();
    expect(screen.getByText('user')).toBeTruthy();
    expect(screen.getByText('exit 0')).toBeTruthy();
    expect(screen.getByText('250ms')).toBeTruthy();
    expect(screen.getByText('agent:agent-7')).toBeTruthy();
    expect(screen.getByText('exit 101')).toBeTruthy();
    expect(screen.getByText('500ms')).toBeTruthy();
  });

  it('invokes action callbacks for Copy cmd, Copy output, Re-run, Pin, and Ask Agent', () => {
    const onCopyCommand = vi.fn();
    const onCopyOutput = vi.fn();
    const onRerun = vi.fn();
    const onPin = vi.fn();
    const onAskAgent = vi.fn();

    render(
      <BlockStreamView
        blocks={[sampleBlocks[1]]}
        onCopyCommand={onCopyCommand}
        onCopyOutput={onCopyOutput}
        onRerun={onRerun}
        onPin={onPin}
        onAskAgent={onAskAgent}
      />,
    );

    fireEvent.click(screen.getByText('Copy cmd'));
    expect(onCopyCommand).toHaveBeenCalledWith(sampleBlocks[1]);

    fireEvent.click(screen.getByText('Copy output'));
    expect(onCopyOutput).toHaveBeenCalledWith(sampleBlocks[1]);

    fireEvent.click(screen.getByText('Re-run'));
    expect(onRerun).toHaveBeenCalledWith(sampleBlocks[1]);

    fireEvent.click(screen.getByText('Pin'));
    expect(onPin).toHaveBeenCalledWith(sampleBlocks[1]);

    fireEvent.click(screen.getByText('Ask Agent'));
    expect(onAskAgent).toHaveBeenCalledWith(sampleBlocks[1]);
  });

  it('renders structured json badge and formats JSON output for !nu commands or JSON responses', () => {
    const structuredBlocks: TermBlockDto[] = [
      {
        id: 10,
        kind: 'Shell',
        status: 'Ok',
        origin: { kind: 'user' },
        input: '!nu ls | to json',
        output: [{ stream: 'Stdout', text: '[{"name":"src","type":"dir"}]', ts_ms: 100 }],
        exit_code: 0,
        started_ms: 100,
        ended_ms: 200,
      },
      {
        id: 11,
        kind: 'Shell',
        status: 'Ok',
        origin: { kind: 'user' },
        input: 'curl http://localhost/api',
        output: [{ stream: 'Stdout', text: '{"status":"ok"}', ts_ms: 300 }],
        exit_code: 0,
        started_ms: 300,
        ended_ms: 450,
      },
    ];

    render(
      <BlockStreamView
        blocks={structuredBlocks}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );

    const badges = screen.getAllByTestId('term-block-structured-badge');
    expect(badges).toHaveLength(2);
    expect(badges[0].textContent).toBe('structured json');
    expect(badges[1].textContent).toBe('structured json');

    expect(screen.getByText(/"name": "src"/)).toBeTruthy();
    expect(screen.getByText(/"status": "ok"/)).toBeTruthy();
  });

  it('renders Focus in Chat button and handles block focus selection', () => {
    const onFocusInChat = vi.fn();
    const onSelectBlock = vi.fn();

    render(
      <BlockStreamView
        blocks={[sampleBlocks[0]]}
        focusedBlockId={1}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
        onFocusInChat={onFocusInChat}
        onSelectBlock={onSelectBlock}
      />,
    );

    const card = screen.getByTestId('term-block-card');
    expect(card.getAttribute('data-focused')).toBe('true');

    fireEvent.click(card);
    expect(onSelectBlock).toHaveBeenCalledWith(1);

    const focusBtn = screen.getByTestId('term-block-focus-chat');
    expect(focusBtn).toBeTruthy();
    fireEvent.click(focusBtn);
    expect(onFocusInChat).toHaveBeenCalledWith(sampleBlocks[0], undefined);
  });

  it('renders cwd basename, git_branch, Pinned header badge vs Unpin toggle, and collapse/expand output', () => {
    const richBlock = {
      ...sampleBlocks[0]!,
      cwd: '/Users/brbrainerd/dev/vox',
      git_branch: 'feat/warp-blocks',
      output: [
        { stream: 'Stdout' as const, text: '\x1b[32mline one\x1b[0m\nline two\nline three\n', ts_ms: 120 },
      ],
    };

    const { rerender } = render(
      <BlockStreamView
        blocks={[richBlock]}
        pinnedBlockIds={[1]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );

    expect(screen.getByText('vox')).toBeTruthy();
    expect(screen.getByTitle('/Users/brbrainerd/dev/vox')).toBeTruthy();
    expect(screen.getByText('feat/warp-blocks')).toBeTruthy();
    expect(screen.getByText('Pinned')).toBeTruthy();
    expect(screen.getByTestId('term-block-pin-1').textContent).toBe('Unpin');
    expect(screen.getByText('line one')).toBeTruthy();

    const collapseBtn = screen.getByLabelText('Collapse command output');
    fireEvent.click(collapseBtn);
    expect(screen.getByText('3 lines hidden')).toBeTruthy();
    expect(screen.queryByText('line one')).toBeNull();

    const expandBtn = screen.getByLabelText('Expand command output');
    fireEvent.click(expandBtn);
    expect(screen.getByText('line one')).toBeTruthy();

    fireEvent.click(screen.getByLabelText('Collapse command output'));
    expect(screen.getByText('3 lines hidden')).toBeTruthy();

    rerender(
      <BlockStreamView
        blocks={[{ ...richBlock, status: 'Failed', exit_code: 1 }]}
        pinnedBlockIds={[1]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );
    expect(screen.getByText('line one')).toBeTruthy();
  });

  it('unmounts stdin bar when not AwaitingInput and handles credential_prompt + Ctrl+B Promote to Background', () => {
    const onSendStdin = vi.fn();
    const onPromoteBg = vi.fn();

    const { rerender } = render(
      <BlockStreamView
        blocks={[sampleBlocks[0]!]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
        onSendStdin={onSendStdin}
        onPromoteBg={onPromoteBg}
      />,
    );

    expect(screen.queryByTestId('term-block-stdin-bar')).toBeNull();
    expect(screen.queryByLabelText('Command stdin input')).toBeNull();

    const awaitingBlock: TermBlockDto = {
      id: 77,
      kind: 'Shell',
      status: 'AwaitingInput',
      origin: { kind: 'user' },
      credential_prompt: true,
      input: 'sudo apt update',
      output: [{ stream: 'Stdout', text: '[sudo] password: ', ts_ms: 200 }],
      exit_code: null,
      started_ms: 150,
      ended_ms: null,
    };

    rerender(
      <BlockStreamView
        blocks={[awaitingBlock]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
        onSendStdin={onSendStdin}
        onPromoteBg={onPromoteBg}
      />,
    );

    expect(screen.getByTestId('term-block-stdin-bar')).toBeTruthy();
    const stdinInput = screen.getByLabelText('Command stdin input') as HTMLInputElement;
    expect(stdinInput.type).toBe('password');
    expect(stdinInput.getAttribute('autocomplete')).toBe('off');

    fireEvent.change(stdinInput, { target: { value: 'hunter2' } });
    fireEvent.click(screen.getByText('Send Stdin Line'));
    expect(onSendStdin).toHaveBeenCalledWith(awaitingBlock, 'hunter2');
    expect(stdinInput.value).toBe('');

    fireEvent.click(screen.getByTestId('term-block-promote-bg'));
    expect(onPromoteBg).toHaveBeenCalledTimes(1);
    expect(onPromoteBg).toHaveBeenCalledWith(awaitingBlock);

    fireEvent.keyDown(stdinInput, { key: 'b', ctrlKey: true });
    expect(onPromoteBg).toHaveBeenCalledTimes(2);
  });

  it('falls back to direct rendering in zero-height jsdom and virtualizes >24 blocks when clientHeight > 0', () => {
    const manyBlocks: TermBlockDto[] = Array.from({ length: 100 }, (_, i) => ({
      id: i + 1,
      kind: 'Shell',
      status: 'Ok',
      origin: { kind: 'user' },
      input: `echo ${i + 1}`,
      output: [{ stream: 'Stdout', text: `row ${i + 1}\n`, ts_ms: 100 + i }],
      exit_code: 0,
      started_ms: 100 + i,
      ended_ms: 120 + i,
    }));

    const { unmount } = render(
      <BlockStreamView
        blocks={manyBlocks}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );
    expect(screen.getAllByTestId('term-block-card')).toHaveLength(100);
    unmount();

    const clientHeightSpy = vi
      .spyOn(HTMLElement.prototype, 'clientHeight', 'get')
      .mockReturnValue(360);
    const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      width: 640,
      height: 360,
      top: 0,
      left: 0,
      bottom: 360,
      right: 640,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });

    try {
      render(
        <BlockStreamView
          blocks={manyBlocks}
          onCopyCommand={vi.fn()}
          onCopyOutput={vi.fn()}
          onRerun={vi.fn()}
          onPin={vi.fn()}
          onAskAgent={vi.fn()}
        />,
      );
      const renderedCards = screen.getAllByTestId('term-block-card');
      expect(renderedCards.length).toBeGreaterThan(0);
      expect(renderedCards.length).toBeLessThan(35);
    } finally {
      clientHeightSpy.mockRestore();
      rectSpy.mockRestore();
    }
  });

  it('renders EmptyState when blocks=[], applies compact p-2 density, and exposes WCAG 2.5.3 Interrupt Process button', () => {
    const onInterrupt = vi.fn();
    const { rerender } = render(
      <BlockStreamView
        blocks={[]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );
    expect(screen.getByText('Terminal session ready')).toBeTruthy();

    const runningBlock: TermBlockDto = {
      id: 88,
      kind: 'Shell',
      status: 'Running',
      origin: { kind: 'user' },
      input: 'sleep 10',
      output: [],
      exit_code: null,
      started_ms: 100,
      ended_ms: null,
    };

    rerender(
      <BlockStreamView
        blocks={[runningBlock]}
        compact
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
        onInterrupt={onInterrupt}
      />,
    );

    const card = screen.getByTestId('term-block-card');
    expect(card.className).toContain('p-2');
    const interruptBtn = screen.getByRole('button', { name: 'Interrupt Process' });
    fireEvent.click(interruptBtn);
    expect(onInterrupt).toHaveBeenCalledWith(runningBlock);
  });

  it('defaults structured blocks to JSON view with term-ledger-filter unmounted and toggles Table ledger view', () => {
    const structuredBlock: TermBlockDto = {
      id: 42,
      kind: 'Shell',
      status: 'Ok',
      origin: { kind: 'user' },
      input: '!nu ls | to json',
      output: [
        {
          stream: 'Stdout',
          text: '[{"name":"src","size":120},{"name":"Cargo.toml","size":40}]',
          ts_ms: 100,
        },
      ],
      exit_code: 0,
      started_ms: 100,
      ended_ms: 140,
    };

    render(
      <BlockStreamView
        blocks={[structuredBlock]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );

    expect(screen.getByTestId('term-ledger-view-toggle')).toBeTruthy();
    const jsonTab = screen.getByTestId('term-ledger-tab-json');
    const tableTab = screen.getByTestId('term-ledger-tab-table');
    expect(jsonTab.getAttribute('aria-selected')).toBe('true');
    expect(tableTab.getAttribute('aria-selected')).toBe('false');
    expect(screen.queryByTestId('term-ledger-table')).toBeNull();
    expect(screen.queryByTestId('term-ledger-filter')).toBeNull();
    expect(screen.getByText(/"name": "src"/)).toBeTruthy();

    fireEvent.click(tableTab);
    expect(tableTab.getAttribute('aria-selected')).toBe('true');
    expect(jsonTab.getAttribute('aria-selected')).toBe('false');
    expect(screen.getByTestId('term-ledger-table')).toBeTruthy();
    expect(screen.getByTestId('term-ledger-filter')).toBeTruthy();

    fireEvent.click(jsonTab);
    expect(jsonTab.getAttribute('aria-selected')).toBe('true');
    expect(screen.queryByTestId('term-ledger-table')).toBeNull();
    expect(screen.getByText(/"name": "src"/)).toBeTruthy();
  });

  it('shows Explain / Fix Failed Block only on failed non-credential non-agent blocks and hides MENS feedback on credential blocks (TERM-AGENT-02, TERM-AGENT-04)', () => {
    const onExplainFix = vi.fn();
    const onFeedback = vi.fn();
    const credFailed: TermBlockDto = { id: 91, kind: 'Shell', status: 'Failed', origin: { kind: 'user' }, credential_prompt: true, input: 'sudo ls', output: [{ stream: 'Stderr', text: 'Authentication failure', ts_ms: 500 }], exit_code: 1, started_ms: 500, ended_ms: 600 };
    const agentTurnFailed: TermBlockDto = { id: 92, kind: 'AgentTurn', status: 'Failed', origin: { kind: 'agent', agent_id: 'terminal-agent' }, input: '? explain failure', output: [{ stream: 'Agent', text: '[stream-error] disconnected', ts_ms: 700 }], exit_code: 1, started_ms: 700, ended_ms: 800 };

    const { rerender } = render(
      <BlockStreamView blocks={[sampleBlocks[0]!, sampleBlocks[1]!, credFailed, agentTurnFailed]} onCopyCommand={vi.fn()} onCopyOutput={vi.fn()} onRerun={vi.fn()} onPin={vi.fn()} onAskAgent={vi.fn()} onExplainFixBlock={onExplainFix} onFeedback={onFeedback} />,
    );

    const explainBtns = screen.getAllByTestId('term-explain-fix-block');
    expect(explainBtns).toHaveLength(1);
    expect(explainBtns[0]?.textContent).toContain('Explain / Fix Failed Block');
    fireEvent.click(explainBtns[0]!);
    expect(onExplainFix).toHaveBeenCalledWith(sampleBlocks[1]);
    expect(screen.getAllByTestId('term-feedback-accept')).toHaveLength(3);

    rerender(<BlockStreamView blocks={[sampleBlocks[1]!]} diagnosingBlockId={sampleBlocks[1]!.id} onCopyCommand={vi.fn()} onCopyOutput={vi.fn()} onRerun={vi.fn()} onPin={vi.fn()} onAskAgent={vi.fn()} onExplainFixBlock={onExplainFix} />);
    const diagnosingBtn = screen.getByTestId('term-explain-fix-block') as HTMLButtonElement;
    expect(diagnosingBtn.disabled).toBe(true);
    expect(diagnosingBtn.textContent).toContain('Diagnosing Failed Block...');
  });

  it('excludes Cancelled blocks with exit_code 130 from Explain/Fix and preserves target block context when retrying Explain/Fix turns (WR-04)', () => {
    const onExplainFix = vi.fn();
    const onRerun = vi.fn();
    const onRunProposed = vi.fn();
    const cancelledBlock: TermBlockDto = {
      id: 95,
      kind: 'Shell',
      status: 'Cancelled',
      origin: { kind: 'user' },
      input: 'sleep 30',
      output: [],
      exit_code: 130,
      started_ms: 100,
      ended_ms: 200,
    };
    const repairFailedTurn: TermBlockDto = {
      id: 96,
      kind: 'AgentTurn',
      status: 'Failed',
      origin: { kind: 'agent', agent_id: 'terminal-agent' },
      input: '? fix block #2',
      output: [
        {
          stream: 'Agent',
          text: '```sh\ncargo test -p fixed\n```\n[stream-error] upstream reset',
          ts_ms: 910,
        },
      ],
      exit_code: 1,
      started_ms: 905,
      ended_ms: 920,
    };

    render(
      <BlockStreamView
        blocks={[sampleBlocks[1]!, cancelledBlock, repairFailedTurn]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={onRerun}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
        onExplainFixBlock={onExplainFix}
        onRunProposedCommand={onRunProposed}
      />,
    );

    const explainBtns = screen.getAllByTestId('term-explain-fix-block');
    expect(explainBtns).toHaveLength(1);
    const cancelledCard = screen.getAllByTestId('term-block-card')[1]!;
    expect(cancelledCard.className).toContain('opacity-80');

    fireEvent.click(screen.getByTestId('term-proposed-fix-run'));
    expect(onRunProposed).toHaveBeenCalledWith('cargo test -p fixed', 96);

    fireEvent.click(screen.getByTestId('term-agent-retry'));
    expect(onExplainFix).toHaveBeenCalledWith(sampleBlocks[1]);
    expect(onRerun).not.toHaveBeenCalled();
  });
});

