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
  if (cmd === 'scientia_publication_classify') {
    return Promise.resolve({
      publication_id: 'pub-studio-1',
      title: 'Studio Test',
      current_stage: 'draft',
      candidate_class: 'empirical_science',
      candidate_class_label: 'Empirical Science',
      publication_tier: 'arxiv_preprint',
      publication_tier_label: 'arXiv.org Preprint',
      arxiv_primary_category: 'cs.AI',
      arxiv_secondary_categories: ['cs.SE'],
      keywords: [],
      acm_ccs: [],
      msc_classes: [],
      license_spdx: 'CC-BY-4.0',
      readiness_score: 92,
      arxiv_ready: true,
      todo_stub_count: 0,
      citation_count: 1,
      results_row_count: 1,
      bundle_tar_gz_bytes: 2200,
      bundle_validation_passed: true,
      validation_findings: [],
      checks: [],
      stage_gates: [],
      main_tex_preview: '\\documentclass[11pt,a4paper]{article}',
      references_bib_preview: '@misc{vox_artifact_pub_studio_1}',
      arxiv_metadata_json: '{}',
      upload_checklist_md: '# Checklist',
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

    fireEvent.click(screen.getByTestId('drawer-autofill-imrad-btn'));
    await waitFor(() => {
      expect(
        invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_classify'),
      ).toBe(true);
    });
  });

  it('syncs publication id from vox://scientia-open-studio and navigates to Publications Studio', async () => {
    const events: CustomEvent[] = [];
    const onNav = (e: Event) => events.push(e as CustomEvent);
    window.addEventListener('vox://navigate-surface', onNav);
    try {
      render(<DiscoveryReview pushToast={vi.fn()} />);
      window.dispatchEvent(
        new CustomEvent('vox://scientia-open-studio', {
          detail: { publicationId: 'pub-handoff-99' },
        }),
      );
      const input = screen.getByPlaceholderText('publication id') as HTMLInputElement;
      await waitFor(() => expect(input.value).toBe('pub-handoff-99'));

      fireEvent.click(screen.getByTestId('open-publications-studio-btn'));
      expect(
        events.some(
          (ev) =>
            ev.detail?.view === 'publications' && ev.detail?.publicationId === 'pub-handoff-99',
        ),
      ).toBe(true);
    } finally {
      window.removeEventListener('vox://navigate-surface', onNav);
    }
  });
});
