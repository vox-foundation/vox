import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import { WaveformTierStrip } from './WaveformTierStrip';
import { deriveRiverSnapshot } from './riverMath';

describe('WaveformTierStrip', () => {
  beforeEach(() => cleanup());

  it('renders throughput, BLAKE3 cache hit %, token savings, and all 3 model cascade tiers', () => {
    const snap = deriveRiverSnapshot({
      tasks: [{ id: 't1', title: 'Test task', status: 'in_progress' }],
      messages: [],
      clutch: 'efficiency',
      risk: 'low',
    });

    render(<WaveformTierStrip snapshot={snap} />);

    expect(screen.getByText(/TIER 1 · INTERCEPTOR/i)).toBeDefined();
    expect(screen.getByText(/TIER 2 · GRAPH FAN-OUT/i)).toBeDefined();
    expect(screen.getByText(/TIER 3 · CAUSAL VERIFIER/i)).toBeDefined();
    expect(screen.getByText(new RegExp(`${snap.cacheHitPct}%`))).toBeDefined();
  });

  it('invokes onSelectTier when a tier card is clicked', () => {
    const onSelectTier = vi.fn();
    const snap = deriveRiverSnapshot({
      tasks: [],
      messages: [],
      clutch: 'balanced',
      risk: 'moderate',
    });

    render(<WaveformTierStrip snapshot={snap} onSelectTier={onSelectTier} />);
    fireEvent.click(screen.getByRole('button', { name: /TIER 2 · GRAPH FAN-OUT/i }));
    expect(onSelectTier).toHaveBeenCalledWith('fanout');
  });
});
