import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';

const MANIFESTS = [
  {
    publication_id: 'pub-alpha-01',
    content_type: 'benchmark_report',
    state: 'draft',
    title: 'Alpha Benchmark',
  },
];

const invokeMock = vi.fn((cmd: string, _args?: Record<string, unknown>) => {
  if (cmd === 'list_publication_manifests') return Promise.resolve(MANIFESTS);
  if (cmd === 'scientia_publication_claims') {
    return Promise.resolve({
      publication_id: 'pub-alpha-01',
      claim_count: 1,
      claims: [
        {
          claim_id: 1,
          id: 1,
          text: 'Throughput improved by 24%',
          verdict: 'Supported',
          verdict_label: 'supported',
          confidence: 0.94,
          is_numeric: true,
          verifiability_score: 0.91,
          verifier_model: 'calibrated-hybrid:lexical',
          created_at_ms: 1,
        },
      ],
    });
  }
  if (cmd === 'scientia_publication_venue_recommend') {
    return Promise.resolve({
      candidate_class: 'benchmark_report',
      recommended_venues: ['arxiv_cs_se'],
      reply_window_days: 14,
      negative_result_quota: 0.15,
      critic_allowed: true,
      atlas_gate_applies: true,
    });
  }
  if (cmd === 'scientia_publication_flywheel_advance') {
    return Promise.resolve({
      publication_id: 'pub-alpha-01',
      stage_completed: 'replay_verify',
      stages_done: ['extract_claims', 'novelty_check', 'replay_verify'],
      artifacts: { replayability: 0.96, ro_crate_digest: 'sha3-256:abc123' },
    });
  }
  return Promise.resolve(null);
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => invokeMock(cmd, args),
}));

import { PublicationsView } from './PublicationsView';

describe('PublicationsView (AXIS-BENTO-03 & AXIS-BENTO-04 & STUD-01/02)', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockClear();
  });

  it('renders @container/surface BentoGrid stage pipeline, overview triad, focusable scroll regions, and typed Scientia Studio drawer without CLI spawns', async () => {
    const { container } = render(<PublicationsView pushToast={vi.fn()} />);

    await waitFor(() => expect(screen.getByText('pub-alpha-01')).toBeTruthy());

    expect(container.querySelector('[data-slot="bento-grid"]')).not.toBeNull();
    expect(screen.getByTestId('publications-pipeline-overview')).toBeTruthy();

    const stageRegion = screen.getByRole('region', { name: /draft stage manifests/i });
    expect(stageRegion.getAttribute('tabindex')).toBe('0');

    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }

    fireEvent.click(screen.getByText('pub-alpha-01'));
    await waitFor(() =>
      expect(screen.getByRole('region', { name: /extracted manuscript claims/i })).toBeTruthy(),
    );

    expect(screen.getByTestId('worthiness-radar-svg')).toBeTruthy();
    expect(screen.getByTestId('novelty-utility-gate-pill')).toBeTruthy();
    expect(screen.getByTestId('direct-publication-drawer')).toBeTruthy();
    expect(screen.getByText('arxiv_cs_se')).toBeTruthy();
    expect(
      invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_claims'),
    ).toBe(true);
    expect(
      invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_venue_recommend'),
    ).toBe(true);
    expect(invokeMock.mock.calls.some(([cmd]) => cmd.startsWith('execute_'))).toBe(false);
  });
});
