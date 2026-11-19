import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, waitFor, fireEvent } from '@testing-library/react';
import React from 'react';

const MANIFESTS = [
  {
    publication_id: 'pub-alpha-01',
    content_type: 'benchmark_report',
    state: 'draft',
    title: 'Alpha Benchmark',
    candidate_class: 'algorithmic_improvement',
    publication_tier: 'arxiv_preprint',
    arxiv_primary_category: 'cs.LG',
    arxiv_secondary_categories: ['cs.PL', 'stat.ML'],
    license_spdx: 'CC-BY-4.0',
    readiness_score: 75,
    arxiv_ready: false,
  },
];

const invokeMock = vi.fn((cmd: string, args?: Record<string, unknown>) => {
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
      candidate_class: args?.candidateClass ?? 'algorithmic_improvement',
      recommended_venues: ['arxiv_cs_se'],
      reply_window_days: 14,
      negative_result_quota: 0.15,
      critic_allowed: true,
      atlas_gate_applies: true,
    });
  }
  if (cmd === 'scientia_publication_arxiv_readiness') {
    return Promise.resolve({
      publication_id: 'pub-alpha-01',
      title: 'Alpha Benchmark',
      current_stage: 'draft',
      next_recommended_stage: 'doi_reserved',
      candidate_class: 'algorithmic_improvement',
      candidate_class_label: 'Algorithmic Improvement',
      publication_tier: 'arxiv_preprint',
      publication_tier_label: 'arXiv.org Preprint',
      arxiv_primary_category: 'cs.LG',
      arxiv_secondary_categories: ['cs.PL'],
      keywords: ['cs.LG'],
      acm_ccs: [],
      msc_classes: [],
      license_spdx: 'CC-BY-4.0',
      readiness_score: 83,
      arxiv_ready: true,
      todo_stub_count: 0,
      citation_count: 1,
      results_row_count: 1,
      bundle_tar_gz_bytes: 2048,
      bundle_validation_passed: true,
      validation_findings: [],
      checks: [
        {
          id: 'zero_todo_stubs',
          category: 'LaTeX',
          label: 'Zero % TODO narrative placeholders',
          passed: true,
          blocking: true,
          detail: '0 placeholders',
        },
      ],
      stage_gates: [],
      main_tex_preview: '\\documentclass[11pt,a4paper]{article}',
      references_bib_preview: '@misc{vox_artifact_pub_alpha_01, title={Alpha}}',
      arxiv_metadata_json: '{"primary_category":"cs.LG"}',
      upload_checklist_md: '# Checklist',
    });
  }
  if (cmd === 'scientia_publication_classify') {
    return Promise.resolve({
      publication_id: 'pub-alpha-01',
      title: 'Alpha Benchmark',
      current_stage: 'draft',
      candidate_class: 'empirical_science',
      candidate_class_label: 'Empirical Science',
      publication_tier: 'arxiv_preprint',
      publication_tier_label: 'arXiv.org Preprint',
      arxiv_primary_category: 'cs.SE',
      arxiv_secondary_categories: ['stat.ML'],
      keywords: ['cs.SE'],
      acm_ccs: [],
      msc_classes: [],
      license_spdx: 'CC-BY-4.0',
      readiness_score: 92,
      arxiv_ready: true,
      todo_stub_count: 0,
      citation_count: 1,
      results_row_count: 1,
      bundle_tar_gz_bytes: 2400,
      bundle_validation_passed: true,
      validation_findings: [],
      checks: [],
      stage_gates: [],
      main_tex_preview: '\\documentclass[11pt,a4paper]{article}\n% Zero stubs',
      references_bib_preview: '@misc{vox_artifact_pub_alpha_01}',
      arxiv_metadata_json: '{}',
      upload_checklist_md: '# Checklist',
    });
  }
  if (cmd === 'scientia_publication_stage_advance') {
    return Promise.resolve({
      publication_id: 'pub-alpha-01',
      title: 'Alpha Benchmark',
      current_stage: args?.targetStage ?? 'doi_reserved',
      candidate_class: 'algorithmic_improvement',
      candidate_class_label: 'Algorithmic Improvement',
      publication_tier: 'arxiv_preprint',
      publication_tier_label: 'arXiv.org Preprint',
      arxiv_primary_category: 'cs.LG',
      arxiv_secondary_categories: ['cs.PL'],
      keywords: ['cs.LG'],
      acm_ccs: [],
      msc_classes: [],
      license_spdx: 'CC-BY-4.0',
      prereserved_doi: '10.48550/arXiv.2611.12345',
      readiness_score: 100,
      arxiv_ready: true,
      todo_stub_count: 0,
      citation_count: 1,
      results_row_count: 1,
      bundle_tar_gz_bytes: 2560,
      bundle_validation_passed: true,
      validation_findings: [],
      checks: [],
      stage_gates: [],
      main_tex_preview: '\\documentclass[11pt,a4paper]{article}',
      references_bib_preview: '@misc{vox_artifact_pub_alpha_01}',
      arxiv_metadata_json: '{}',
      upload_checklist_md: '# Checklist',
    });
  }
  if (cmd === 'scientia_publication_arxiv_bundle') {
    return Promise.resolve({
      publication_id: 'pub-alpha-01',
      bytes_len: 2560,
      entries: [{ path: 'references.bib', bytes: 120 }],
      main_tex: '\\documentclass[11pt,a4paper]{article}',
      references_bib: '@misc{vox_artifact_pub_alpha_01, title={Alpha}}',
      arxiv_metadata_json: '{}',
      upload_checklist_md: '# Checklist',
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
    expect(screen.getByTestId('publication-classification-tablet')).toBeTruthy();
    expect(screen.getByTestId('publication-stage-controller')).toBeTruthy();
    expect(screen.getByText('arxiv_cs_se')).toBeTruthy();
    expect(
      invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_claims'),
    ).toBe(true);
    expect(
      invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_venue_recommend'),
    ).toBe(true);
    expect(
      invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_arxiv_readiness'),
    ).toBe(true);
    expect(invokeMock.mock.calls.some(([cmd]) => cmd.startsWith('execute_'))).toBe(false);

    fireEvent.click(screen.getByTestId('pub-autofill-imrad-btn'));
    await waitFor(() =>
      expect(
        invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_classify'),
      ).toBe(true),
    );

    fireEvent.click(screen.getByTestId('stage-advance-doi_reserved-btn'));
    await waitFor(() =>
      expect(
        invokeMock.mock.calls.some(([cmd]) => cmd === 'scientia_publication_stage_advance'),
      ).toBe(true),
    );

    fireEvent.click(screen.getByTestId('pub-preview-bibtex-btn'));
    await waitFor(() =>
      expect(screen.getByTestId('publication-artifact-preview')).toBeTruthy(),
    );
  }, 20_000);
});
