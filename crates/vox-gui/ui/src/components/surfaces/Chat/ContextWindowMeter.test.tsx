import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ContextWindowMeter } from './ContextWindowMeter';

describe('ContextWindowMeter', () => {
  it('renders percentage label', () => {
    render(
      <ContextWindowMeter
        usedTokens={64_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
      />
    );
    // 64000/128000 = 50%
    expect(screen.getByText('50%')).toBeInTheDocument();
  });

  it('shows strategy name', () => {
    render(
      <ContextWindowMeter
        usedTokens={0}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="aggressive"
      />
    );
    expect(screen.getByText('aggressive')).toBeInTheDocument();
  });

  it('applies green class when under 70%', () => {
    const { container } = render(
      <ContextWindowMeter
        usedTokens={50_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
      />
    );
    // The fill bar should have data-zone="safe"
    expect(container.querySelector('[data-zone="safe"]')).toBeTruthy();
  });

  it('applies amber class at 80% usage', () => {
    const { container } = render(
      <ContextWindowMeter
        usedTokens={102_400}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
      />
    );
    expect(container.querySelector('[data-zone="warn"]')).toBeTruthy();
  });

  it('applies red class above 90% usage', () => {
    const { container } = render(
      <ContextWindowMeter
        usedTokens={120_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
      />
    );
    expect(container.querySelector('[data-zone="danger"]')).toBeTruthy();
  });

  it('clamps percent to 100 and clamps role="meter" aria-valuenow/aria-valuemax', () => {
    const { rerender } = render(
      <ContextWindowMeter
        usedTokens={999_999}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
      />
    );
    expect(screen.getByText('100%')).toBeInTheDocument();
    const meter = screen.getByRole('meter');
    expect(meter).toHaveAttribute('aria-valuemax', '128000');
    expect(meter).toHaveAttribute('aria-valuenow', '128000');

    rerender(
      <ContextWindowMeter
        usedTokens={50}
        maxTokens={0}
        thresholdTokens={0}
        strategy="balanced"
      />
    );
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuemax', '1');
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuenow', '1');
  });

  it('renders 0% and 0% cached when usedTokens is 0 (no fake static cache hit rate)', () => {
    render(
      <ContextWindowMeter
        usedTokens={0}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="conservative"
      />
    );
    expect(screen.getByText('0%')).toBeInTheDocument();
    expect(screen.getByTestId('context-meter-kv-cache')).toHaveTextContent('0% cached');
    expect(screen.getByTestId('context-meter-slice-system')).toHaveTextContent('Sys0');
    expect(screen.getByTestId('context-meter-slice-kb')).toHaveTextContent('KB0');
    expect(screen.getByTestId('context-meter-slice-ast')).toHaveTextContent('AST0');
    expect(screen.getByTestId('context-meter-slice-history')).toHaveTextContent('Hist0');
  });

  it('computes empirical KV cache hit percentage from cacheReadTokens or cacheHitPct', () => {
    const { rerender } = render(
      <ContextWindowMeter
        usedTokens={20_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
        cacheReadTokens={60_000}
      />
    );
    // 60_000 / (20_000 + 60_000) = 75%
    expect(screen.getByTestId('context-meter-kv-cache')).toHaveTextContent('75% cached');

    rerender(
      <ContextWindowMeter
        usedTokens={20_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
        cacheHitPct={88.6}
      />
    );
    expect(screen.getByTestId('context-meter-kv-cache')).toHaveTextContent('89% cached');
  });

  it('renders BPE token slice breakdown, interactive slice selection, and segmented strategy buttons', () => {
    const onStrategyChange = vi.fn();
    const onSliceSelect = vi.fn();
    render(
      <ContextWindowMeter
        usedTokens={64_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
        sliceBreakdown={{
          systemTokens: 8_000,
          kbTokens: 20_000,
          astTokens: 16_000,
          historyTokens: 20_000,
        }}
        onSliceSelect={onSliceSelect}
        onStrategyChange={onStrategyChange}
      />
    );
    const slices = screen.getByTestId('context-meter-bpe-slices');
    expect(slices).toBeInTheDocument();
    expect(slices.tagName.toLowerCase()).toBe('svg');
    expect(screen.getByTestId('context-meter-kv-cache')).toHaveTextContent('0% cached');

    const kbBtn = screen.getByTestId('context-meter-slice-kb');
    expect(kbBtn).toHaveTextContent('20,000');
    expect(kbBtn).toHaveAttribute('aria-pressed', 'false');

    fireEvent.click(kbBtn);
    expect(onSliceSelect).toHaveBeenCalledWith('kb');
    expect(kbBtn).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByTestId('context-meter-slice-inspector')).toHaveTextContent(
      'Pinned Files & Cache',
    );

    fireEvent.click(kbBtn);
    expect(onSliceSelect).toHaveBeenLastCalledWith(null);
    expect(kbBtn).toHaveAttribute('aria-pressed', 'false');

    fireEvent.click(screen.getByTestId('context-meter-strategy-conservative'));
    expect(onStrategyChange).toHaveBeenCalledWith('conservative');
    expect(screen.queryByTestId('context-meter-strategy-btn')).toBeNull();
    expect(slices.querySelector('rect')?.getAttribute('fill')).toBe('var(--color-overlay-subtle)');
    expect(slices.querySelector('line')?.getAttribute('stroke')).toBe('var(--color-text-muted)');
  });

  it('renders 1-click surgical Clear Tools action button and reclaimed token counter (CONTEXT-SURGERY-03)', () => {
    const onClearToolOutputs = vi.fn();
    render(
      <ContextWindowMeter
        usedTokens={64_000}
        maxTokens={128_000}
        thresholdTokens={102_400}
        strategy="balanced"
        reclaimedToolTokens={1_420}
        onClearToolOutputs={onClearToolOutputs}
      />
    );
    const clearBtn = screen.getByTestId('context-meter-clear-tools-btn');
    expect(clearBtn).toHaveTextContent('Clear Tools (-1,420)');
    fireEvent.click(clearBtn);
    expect(onClearToolOutputs).toHaveBeenCalledTimes(1);
  });
});
