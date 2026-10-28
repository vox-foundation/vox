// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import { BlockStreamView } from './BlockStreamView';
import type { TermBlockDto } from '../../../transport';

const sampleBlocks: TermBlockDto[] = [
  {
    id: 1,
    kind: 'Shell',
    status: 'Ok',
    origin: { kind: 'user' },
    input: 'cargo check',
    output: [{ stream: 'Stdout', text: 'Finished dev\n', ts_ms: 100 }],
    exit_code: 0,
    started_ms: 100,
    ended_ms: 350,
  },
  {
    id: 2,
    kind: 'Shell',
    status: 'Failed',
    origin: { kind: 'agent', agent_id: 'agent-7' },
    input: 'cargo test -p broken',
    output: [{ stream: 'Stderr', text: 'error: test failed\n', ts_ms: 400 }],
    exit_code: 101,
    started_ms: 400,
    ended_ms: 900,
  },
];

describe('BlockStreamView', () => {
  beforeEach(() => cleanup());

  it('renders block cards with origin badges, exit codes, duration, and output', () => {
    render(
      <BlockStreamView
        blocks={sampleBlocks}
        pinnedBlockIds={[1]}
        onCopyCommand={vi.fn()}
        onCopyOutput={vi.fn()}
        onRerun={vi.fn()}
        onPin={vi.fn()}
        onAskAgent={vi.fn()}
      />,
    );
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
});

