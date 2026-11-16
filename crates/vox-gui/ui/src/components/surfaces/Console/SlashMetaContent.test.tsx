import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import { SlashMetaContent, TermBlockFeedbackStrip } from './SlashMetaContent';
import type { TermBlockDto } from '../../../transport';

function makeSlashBlock(input: string, text: string, kind: unknown = 'SlashCommand'): TermBlockDto {
  return {
    id: 50,
    kind,
    status: 'Ok',
    origin: { kind: 'user' },
    input,
    output: [{ stream: 'Agent', text, ts_ms: 100 }],
    exit_code: 0,
    started_ms: 100,
    ended_ms: 150,
  };
}

describe('SlashMetaContent & TermBlockFeedbackStrip (TERM-AGENT-04)', () => {
  beforeEach(() => {
    cleanup();
  });

  it('renders /compact digest card and /budget metric grid on 4px grid without .5 spacing', () => {
    const { container, rerender } = render(
      <SlashMetaContent
        block={makeSlashBlock(
          '/compact',
          '[compact digest - 6 blocks]\n- #1 `cargo check` (exit 0): Finished\n- #2 `cargo test` (exit 1): 1 failed',
          'Meta',
        )}
      />,
    );

    expect(screen.getByTestId('term-slash-meta-card')).toBeTruthy();
    const digest = screen.getByTestId('term-compact-digest-card');
    expect(digest.textContent).toContain('COMPACTED DIGEST');
    expect(digest.textContent).toContain('cargo check');
    expect(container.innerHTML).not.toMatch(/\b(?:px|py|p|gap|size|mr|ml|mt|mb|pt|pb|space-y)-\d+\.5\b/);

    rerender(
      <SlashMetaContent
        block={makeSlashBlock(
          '/budget 2.50',
          '[budget]\nupdated_per_session_budget_usd: 2.50\ncontext_tokens: 420 / 16000 (max 32000, threshold 24000, strategy balanced)\nsession_spend_usd: 0.0420 / 2.50\ndaily_spend_usd: 0.1200 / 5.00\ntotal_spend_usd: 1.2500',
        )}
      />,
    );

    const budget = screen.getByTestId('term-slash-budget');
    expect(budget.textContent).toContain('Updated session token ceiling');
    expect(budget.textContent).toContain('420 / 16000');
    expect(budget.textContent).toContain('balanced');
    expect(budget.textContent).toContain('$0.0420');
    expect(screen.getByTestId('term-slash-budget-alert').textContent).toBe('Healthy');

    rerender(
      <SlashMetaContent
        block={makeSlashBlock(
          '/budget',
          '[budget]\ncontext_tokens: 13000 / 16000 (max 32000, threshold 12800, strategy balanced)\nsession_spend_usd: 1.0000 / 2.50',
        )}
      />,
    );
    expect(screen.getByTestId('term-slash-budget-alert').textContent).toBe('Warning');

    rerender(
      <SlashMetaContent
        block={makeSlashBlock(
          '/budget',
          '[budget]\ncontext_tokens: 420 / 16000 (max 32000, threshold 24000, strategy balanced)\nsession_spend_usd: 2.5000 / 2.50',
        )}
      />,
    );
    expect(screen.getByTestId('term-slash-budget-alert').textContent).toBe('Ceiling Reached');
  });

  it('renders /model, /context, and empty states for /skills and /memory', () => {
    const { rerender } = render(
      <SlashMetaContent
        block={makeSlashBlock(
          '/model',
          '[model]\nactive_model: openrouter/claude-sonnet\ndefault_route: ollama/qwen3-coder-14b (local)',
        )}
      />,
    );
    expect(screen.getByTestId('term-slash-model').textContent).toContain('openrouter/claude-sonnet');

    rerender(
      <SlashMetaContent
        block={makeSlashBlock(
          '/context',
          '[context]\nsession_id: s-1\nblocks: 4 total (1 pinned, 1 agent)\ncwd: /repo\ngit_branch: main\ntoken_budget: 300 / 16000 usable',
        )}
      />,
    );
    expect(screen.getByTestId('term-slash-context').textContent).toContain('/repo');

    rerender(<SlashMetaContent block={makeSlashBlock('/skills zzz', '[skills]\nNo matching skills found.')} />);
    expect(screen.getByText('No matching skills found')).toBeTruthy();

    rerender(<SlashMetaContent block={makeSlashBlock('/memory zzz', '[memory]\nNo recall hits for `zzz`.')} />);
    expect(screen.getByText('No memory entries recalled')).toBeTruthy();
  });

  it('keeps term-feedback-correct-input null when idle, submits Accept/Reject/Correct with WCAG 2.5.3 labels, guards failed persistence, and hides on credential_prompt', async () => {
    const onFeedback = vi.fn().mockResolvedValue(true);
    const onRunCommand = vi.fn();
    const block = makeSlashBlock('/help', 'help text', 'Shell');
    block.input = 'cargo tset';

    const { rerender } = render(
      <TermBlockFeedbackStrip
        block={block}
        onFeedback={onFeedback}
        onRunCommand={onRunCommand}
      />,
    );

    expect(screen.queryByTestId('term-feedback-correct-input')).toBeNull();
    expect(screen.queryAllByRole('textbox')).toHaveLength(0);

    onFeedback.mockResolvedValueOnce(false);
    fireEvent.click(screen.getByTestId('term-feedback-accept'));
    await waitFor(() => expect(onFeedback).toHaveBeenCalledWith(block, 'accepted'));
    expect(screen.getByTestId('term-feedback-accept').getAttribute('aria-pressed')).toBe('false');
    expect(screen.getByTestId('term-feedback-accept').textContent).toBe('Accept');

    fireEvent.click(screen.getByTestId('term-feedback-accept'));
    await waitFor(() => expect(onFeedback).toHaveBeenCalledTimes(2));
    const acceptBtn = screen.getByTestId('term-feedback-accept');
    expect(acceptBtn.getAttribute('aria-pressed')).toBe('true');
    expect(acceptBtn.getAttribute('aria-label')).toMatch(/^Accepted/);

    fireEvent.click(screen.getByTestId('term-feedback-reject'));
    await waitFor(() => expect(onFeedback).toHaveBeenCalledWith(block, 'rejected'));
    const rejectBtn = screen.getByTestId('term-feedback-reject');
    expect(rejectBtn.getAttribute('aria-pressed')).toBe('true');
    expect(rejectBtn.getAttribute('aria-label')).toMatch(/^Rejected/);

    fireEvent.click(screen.getByTestId('term-feedback-correct'));
    const input = screen.getByTestId('term-feedback-correct-input') as HTMLInputElement;
    expect(input.value).toBe('cargo tset');
    fireEvent.change(input, { target: { value: 'cargo test' } });
    fireEvent.click(screen.getByTestId('term-feedback-correct-submit'));

    await waitFor(() => expect(onFeedback).toHaveBeenCalledWith(block, 'corrected', 'cargo test'));
    expect(screen.queryByTestId('term-feedback-correct-input')).toBeNull();
    expect(screen.getByTestId('term-feedback-correct').getAttribute('aria-label')).toMatch(/^Corrected/);

    fireEvent.click(screen.getByTestId('term-feedback-run-corrected'));
    expect(onRunCommand).toHaveBeenCalledWith('cargo test');

    rerender(
      <TermBlockFeedbackStrip
        block={{ ...block, credential_prompt: true }}
        onFeedback={onFeedback}
      />,
    );
    expect(screen.queryByTestId('term-feedback-accept')).toBeNull();
    expect(screen.queryByTestId('term-feedback-reject')).toBeNull();
    expect(screen.queryByTestId('term-feedback-correct')).toBeNull();
  });
});
