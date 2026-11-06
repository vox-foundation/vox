import React from 'react';
import { describe, expect, it } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ChatTranscript, renderStructuredMessageText } from './ChatTranscript';
import type { ChatMessage } from '../../../lib/chatCorrelation';

function makeMessages(count: number): ChatMessage[] {
  return Array.from({ length: count }, (_, idx) => ({
    id: `msg-${idx + 1}`,
    role: idx % 2 === 0 ? 'user' : 'assistant',
    text:
      idx % 2 === 0
        ? `User prompt #${idx + 1}: How does VoxIdAllocatorV2 derive deterministic IDs?`
        : `Assistant reply #${idx + 1}: First, VoxIdAllocatorV2 encodes each segment.`,
    status: 'done',
    modelId: idx % 2 === 1 ? 'anthropic/claude-sonnet' : undefined,
    latencyMs: idx % 2 === 1 ? 210 : undefined,
  }));
}

describe('ChatTranscript Wave 3 Integration (T3-CHAT-UI-WIRE-01..04)', () => {
  it('renders ConsensusGridView when Consensus (2-4) is triggered and opens LCA BranchComparisonModal', async () => {
    const messages = makeMessages(4);
    render(<ChatTranscript messages={messages} sessionId="session-wave3" />);

    const consensusBtn = screen.getByTestId('chat-consensus-trigger');
    fireEvent.click(consensusBtn);

    await waitFor(() => {
      expect(screen.getByTestId('consensus-grid-view')).toBeTruthy();
    });

    const branchBtn = screen.getByTestId('chat-branch-compare-trigger');
    fireEvent.click(branchBtn);

    expect(screen.getByTestId('branch-comparison-modal')).toBeTruthy();
  });

  it('applies CHUNK_SIZE = 16 chunked lazy loading and deferred code block highlighting on > 20,000 char blocks', () => {
    const messages = makeMessages(22);
    render(<ChatTranscript messages={messages} sessionId="session-chunked" />);

    const loadOlderBtn = screen.getByTestId('chat-load-older-chunks');
    expect(loadOlderBtn.textContent).toContain('6 earlier');
    fireEvent.click(loadOlderBtn);
    expect(screen.queryByTestId('chat-load-older-chunks')).toBeNull();

    const largeCode = 'x'.repeat(20_500);
    const { container } = render(
      <div>{renderStructuredMessageText(`\`\`\`rust\n${largeCode}\n\`\`\``)}</div>,
    );
    const block = container.querySelector('[data-testid="chat-code-block"]');
    expect(block?.getAttribute('data-highlight-deferred')).toBe('true');
    expect(block?.getAttribute('data-highlight-idle-delay-ms')).toBe('300');
  });
});
