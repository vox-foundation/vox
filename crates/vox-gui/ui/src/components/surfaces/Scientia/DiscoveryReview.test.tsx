import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';

const invokeMock = vi.fn((cmd: string, _args?: unknown) => {
  if (cmd === 'scientia_publication_flywheel_advance') {
    return Promise.resolve({
      publication_id: 'pub-studio-1',
      stage_completed: 'replay_verify',
      stages_done: ['extract_claims', 'novelty_check', 'replay_verify'],
      artifacts: { replayability: 0.97, ro_crate_digest: 'sha3-256:999' },
    });
  }
  return Promise.resolve([]);
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));

// Mock the transport event bridge: listenScientiaQueue rejects (no Tauri), so
// the component falls back to its interval — exactly the browser-dev path.
vi.mock('../../../transport', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../transport')>()),
  listenScientiaQueue: vi.fn().mockRejectedValue(new Error('not in tauri')),
}));

import { DiscoveryReview } from './DiscoveryReview';

describe('DiscoveryReview', () => {
  beforeEach(() => cleanup());

  it('renders the Discovery Review heading', () => {
    render(<DiscoveryReview pushToast={vi.fn()} />);
    expect(screen.getByText('Discovery Review')).toBeTruthy();
  });

  it('shows the empty-state prompt before a publication id is entered', () => {
    render(<DiscoveryReview pushToast={vi.fn()} />);
    expect(
      screen.getByText('Enter a publication id to load its review queue.'),
    ).toBeTruthy();
  });

  it('every button carries an explicit type="button"', () => {
    render(<DiscoveryReview pushToast={vi.fn()} />);
    for (const b of screen.getAllByRole('button')) {
      expect(b.getAttribute('type')).toBe('button');
    }
  });

  it('marks the awaiting-review queue as an aria-live region', () => {
    const { container } = render(<DiscoveryReview pushToast={vi.fn()} />);
    expect(container.querySelector('[aria-live="polite"]')).toBeTruthy();
  });

  it('renders the 6-axis WorthinessRadarPanel SVG, Two-Stage Novelty x Utility Gate pill, and DirectPublicationDrawer', async () => {
    const pushToast = vi.fn();
    render(<DiscoveryReview pushToast={pushToast} />);

    expect(screen.getByTestId('scientia-worthiness-meter')).toBeTruthy();
    expect(screen.getByTestId('worthiness-radar-svg')).toBeTruthy();
    expect(screen.getByTestId('novelty-utility-gate-pill')).toBeTruthy();
    expect(screen.getByTestId('direct-publication-drawer')).toBeTruthy();

    fireEvent.change(screen.getByPlaceholderText('publication id'), {
      target: { value: 'pub-studio-1' },
    });
    fireEvent.click(screen.getByTestId('flywheel-advance-btn'));

    await waitFor(() => {
      expect(
        invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_flywheel_advance'),
      ).toBe(true);
    });
  });
});
