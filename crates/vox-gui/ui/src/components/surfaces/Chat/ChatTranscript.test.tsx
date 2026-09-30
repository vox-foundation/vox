// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MessageBubble, ChatTranscript } from './ChatTranscript';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import { listHarnessIssuesForSession } from '../Scientia/harnessIssuesApi';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { TurnEventDto } from '../../../types/dashboard';

const CONTRACT_KINDS: Array<{ kind: string; example: TurnEventDto }> = JSON.parse(
  readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), '../../../../../../../contracts/gui/turn-event-kinds.v1.json'),
    'utf8',
  ),
).kinds;

function example(kind: string): TurnEventDto {
  const entry = CONTRACT_KINDS.find((k) => k.kind === kind);
  if (!entry) throw new Error(`contract has no kind ${kind}`);
  return { ...entry.example };
}

vi.mock('../Scientia/harnessIssuesApi', () => ({
  listHarnessIssuesForSession: vi.fn(),
}));

function msg(overrides: Partial<ChatMessage>): ChatMessage {
  return {
    id: 'm1',
    role: 'assistant',
    text: 'reply',
    status: 'done',
    runId: 'r1',
    ...overrides,
  };
}

describe('MessageBubble role labels', () => {
  it('exposes user role as sr-only, not a visible You label', () => {
    render(<MessageBubble message={msg({ role: 'user', text: 'hi', id: 'u1' })} />);
    expect(screen.getByText('You').className).toMatch(/sr-only/);
  });
});

describe('MessageBubble grounding-check badge', () => {
  it('shows a low-confidence badge on an assistant message flagged by the grounding check', () => {
    render(<MessageBubble message={msg({ groundingFlagged: true })} />);
    expect(screen.getByText(/low confidence/i)).toBeInTheDocument();
  });

  it('does not show the badge when the message was not flagged', () => {
    render(<MessageBubble message={msg({ groundingFlagged: false })} />);
    expect(screen.queryByText(/low confidence/i)).not.toBeInTheDocument();
  });

  it('does not show the badge on user messages even if somehow flagged', () => {
    render(<MessageBubble message={msg({ role: 'user', groundingFlagged: true })} />);
    expect(screen.queryByText(/low confidence/i)).not.toBeInTheDocument();
  });
});

describe('MessageBubble routing trace (replaces ModelBadge)', () => {
  it('shows a local pick as its local id in the trace summary, with no model badge', () => {
    const local = { ...example('routing_decision'), resolved_from: 'local', resolved_id: 'mens/e2e-smoke-metal' };
    render(<MessageBubble message={msg({ modelId: 'mens/e2e-smoke-metal', events: [local] })} />);
    expect(screen.getByTestId('chat-trace-summary')).toHaveTextContent('mens/e2e-smoke-metal (local)');
    expect(screen.queryByRole('button', { name: /Completed by/ })).not.toBeInTheDocument();
  });

  it('a hydrated reply (modelId and latency, no events) keeps its attribution and the Assistant header', () => {
    render(<MessageBubble message={msg({ modelId: 'mens/e2e-smoke-metal', latencyMs: 1200 })} />);
    expect(screen.getByText('Assistant')).toBeInTheDocument();
    const summary = screen.getByTestId('chat-trace-summary');
    expect(summary).toHaveTextContent('mens/e2e-smoke-metal · 1.2s');
    expect(summary).not.toHaveTextContent('latest');
  });
});

describe('ChatTranscript harness issue summary strip', () => {
  beforeEach(() => {
    vi.mocked(listHarnessIssuesForSession).mockReset();
  });

  it('shows a detected issue fetched for the session, struck through when dismissed', async () => {
    vi.mocked(listHarnessIssuesForSession).mockResolvedValue([
      {
        id: 1,
        source: 'chat_session',
        session_key: 's1',
        target_path: null,
        detected_at_ms: 1_750_000_000_000,
        category: 'stub',
        severity: 'medium',
        summary: 'looks stubbed',
        evidence_json: '{}',
        status: 'dismissed',
      },
    ]);

    render(<ChatTranscript messages={[msg({})]} sessionId="s1" />);

    const row = await screen.findByTestId('transcript-harness-issue-1');
    expect(row).toHaveTextContent('Issue detected (dismissed): looks stubbed');
    expect(row.className).toContain('line-through');
    await waitFor(() => expect(listHarnessIssuesForSession).toHaveBeenCalledWith('s1'));
  });

  it('does not fetch or render anything when no sessionId is provided', () => {
    render(<ChatTranscript messages={[msg({})]} />);
    expect(listHarnessIssuesForSession).not.toHaveBeenCalled();
    expect(screen.queryByTestId(/transcript-harness-issue-/)).not.toBeInTheDocument();
  });
});

describe('ChatTranscript verbosity control', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.mocked(listHarnessIssuesForSession).mockReset();
  });

  it('Verbose opens a clean trace that Normal leaves collapsed, and persists the choice', () => {
    render(<ChatTranscript messages={[msg({ events: [example('routing_decision'), example('tool_receipt')] })]} />);
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
    fireEvent.click(screen.getByRole('radio', { name: 'Verbose' }));
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
    expect(localStorage.getItem('gui.chat.verbosity.v1')).toBe('"verbose"');
  });

  it('Quiet keeps a failed turn collapsed', () => {
    render(
      <ChatTranscript
        messages={[msg({ events: [example('routing_decision'), { ...example('tool_receipt'), verified: false }] })]}
      />,
    );
    expect(screen.getByTestId('chat-trace-steps')).toBeVisible();
    fireEvent.click(screen.getByRole('radio', { name: 'Quiet' }));
    expect(screen.getByTestId('chat-trace-steps')).not.toBeVisible();
  });
});

