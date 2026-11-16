import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ChatTranscript, renderStructuredMessageText } from './ChatTranscript';
import type { ChatMessage } from '../../../lib/chatCorrelation';
import { voxTransport } from '../../../transport';

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
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders ConsensusGridView when Consensus (2-4) succeeds and opens LCA BranchComparisonModal', async () => {
    vi.spyOn(voxTransport, 'invokeMcpTool').mockImplementation(async (tool) => {
      if (tool === 'vox_consensus_run') {
        return {
          is_error: false,
          result: {
            success: true,
            data: {
              prompt: 'How does VoxIdAllocatorV2 derive deterministic IDs?',
              models: ['anthropic/claude-sonnet', 'openai/gpt', 'mens/qwen3-8b'],
              synthesis: 'All legs converged on deterministic ID derivation.',
              total_elapsed_ms: 310,
              rich_results: [
                {
                  model_id: 'anthropic/claude-sonnet',
                  status: 'completed',
                  content: 'Leg 1 response',
                  response_time_ms: 190,
                  quality: { quality_score: 92 },
                },
                {
                  model_id: 'openai/gpt',
                  status: 'completed',
                  content: 'Leg 2 response',
                  response_time_ms: 220,
                  quality: { quality_score: 89 },
                },
              ],
            },
          },
        };
      }
      return { is_error: false, result: { success: true, data: { issues: [] } } };
    });

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

  it('renders honest consensus-error-banner instead of fake legs when live consensus fails', async () => {
    vi.spyOn(voxTransport, 'invokeMcpTool').mockImplementation(async (tool) => {
      if (tool === 'vox_consensus_run') {
        return { is_error: true, result: 'Provider quota exhausted' };
      }
      return { is_error: false, result: { success: true, data: { issues: [] } } };
    });

    const messages = makeMessages(4);
    render(<ChatTranscript messages={messages} sessionId="session-wave3-err" />);

    fireEvent.click(screen.getByTestId('chat-consensus-trigger'));

    await waitFor(() => {
      expect(screen.getByTestId('consensus-error-banner').textContent).toContain(
        'Provider quota exhausted',
      );
    });
    expect(screen.queryByTestId('consensus-grid-view')).toBeNull();
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
