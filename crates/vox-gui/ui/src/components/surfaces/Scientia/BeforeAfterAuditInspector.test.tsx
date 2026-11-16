import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import React from 'react';
import type { V76AuditDiffDto } from '../../../api/scientia';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { BeforeAfterAuditInspector } from './BeforeAfterAuditInspector';

describe('BeforeAfterAuditInspector (Phase 452 AUDT-03)', () => {
  beforeEach(() => {
    cleanup();
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(null);
  });

  it('renders 6-axis Before -> After deltas, 95% CI bounds, NonInferior/Superior pills, and 9/9 bug closure', async () => {
    render(<BeforeAfterAuditInspector />);

    await waitFor(() => {
      expect(screen.getByTestId('before-after-audit-inspector')).toBeInTheDocument();
      expect(screen.getByTestId('audit-bug-closure-pill')).toHaveTextContent('9/9 Live Bugs Closed');
    });

    for (const axis of [
      'performance',
      'quality',
      'verification',
      'integration',
      'socrates_calibration',
      'novelty_detection',
    ]) {
      expect(screen.getByTestId(`audit-axis-card-${axis}`)).toBeInTheDocument();
      expect(screen.getByTestId(`axis-ni-pill-${axis}`)).toHaveTextContent('NonInferior');
      expect(screen.getByTestId(`axis-superior-pill-${axis}`)).toHaveTextContent('Superior');
      expect(screen.getByTestId(`axis-ci-bounds-${axis}`)).toHaveTextContent(/95% CI \[/);
    }

    expect(screen.getByTestId('bug-status-gui_cli_db_spawn_count')).toHaveTextContent('Resolved');
    expect(screen.getByTestId('bug-status-session_detail_ipc_payload_bytes')).toHaveTextContent(
      'Resolved',
    );
    expect(screen.getByTestId('lessons-summary-bar')).toBeInTheDocument();
    expect(screen.getByTestId('unreviewed-ssot-mutations-pill')).toHaveTextContent(
      'unreviewed_ssot_mutations: 0',
    );
  });

  it('filters diff hunks with All | Changes Only and per-axis filter buttons', async () => {
    render(<BeforeAfterAuditInspector />);

    await waitFor(() => {
      expect(screen.getByText(/Showing 18 of 18 lines/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('diff-filter-changes'));
    expect(screen.getByText(/Showing 16 of 18 lines/i)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('axis-filter-performance'));
    expect(screen.getByTestId('audit-axis-card-performance')).toBeInTheDocument();
    expect(screen.queryByTestId('audit-axis-card-quality')).toBeNull();
    expect(screen.getByText(/Showing 2 of 18 lines/i)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('axis-filter-all'));
    fireEvent.click(screen.getByTestId('diff-filter-all'));
    expect(screen.getByText(/Showing 18 of 18 lines/i)).toBeInTheDocument();
  });

  it('virtualizes large receipt diff hunks and applies semantic delete/insert/equal tags', () => {
    const customData: V76AuditDiffDto = {
      before_stage: 'before',
      after_stage: 'after',
      before_path: 'contracts/reports/scientia-v76-before-baseline.v1.json',
      after_path: 'contracts/reports/scientia-v76-after-closure.v1.json',
      after_available: true,
      axes: [
        {
          axis: 'performance',
          label: 'Performance',
          metric: 'kv_cache_prefix_stability_ratio',
          before_value: 0.54,
          after_value: 0.96,
          delta: 0.42,
          higher_is_better: true,
          ci_low: 0.18,
          ci_high: 0.65,
          ni_verdict: 'non_inferior',
          superior: true,
        },
      ],
      bugs: [{ id: 'gui_cli_db_spawn_count', before: 3, after: 0, resolved: true }],
      hunks: Array.from({ length: 1000 }, (_, i) => ({
        index: i,
        tag: i % 3 === 0 ? 'delete' : i % 3 === 1 ? 'insert' : 'equal',
        old_line: i % 3 === 1 ? null : i + 1,
        new_line: i % 3 === 0 ? null : i + 1,
        text: `  "line_${i}": ${i}`,
      })),
      lessons_summary: {
        quarantined_findings_count: 4,
        corroborated_promotions_count: 2,
        proposed_effort_rules_count: 1,
        auto_dpo_pairs_minted: 4,
        unreviewed_ssot_mutations: 0,
      },
    };

    render(<BeforeAfterAuditInspector initialData={customData} />);
    expect(screen.getByText('Showing 1000 of 1000 lines')).toBeInTheDocument();
    const renderedRows = screen.getAllByTestId('audit-diff-hunk-row');
    expect(renderedRows.length).toBeGreaterThan(0);
    expect(renderedRows.length).toBeLessThan(40);
    expect(renderedRows[0]).toHaveAttribute('data-tag', 'delete');
    expect(renderedRows[1]).toHaveAttribute('data-tag', 'insert');
    expect(renderedRows[2]).toHaveAttribute('data-tag', 'equal');
  });

  it('prefers live daemon payload from scientia_v76_audit_diff when available', async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'scientia_v76_audit_diff') {
        return Promise.resolve({
          before_stage: 'before-live',
          after_stage: 'after-live',
          before_path: 'contracts/reports/scientia-v76-before-baseline.v1.json',
          after_path: 'contracts/reports/scientia-v76-after-closure.v1.json',
          after_available: true,
          axes: [],
          bugs: [{ id: 'mock_verifier_fallback_rate', before: 1, after: 0, resolved: true }],
          hunks: [{ index: 0, tag: 'insert', old_line: null, new_line: 1, text: '+live' }],
          lessons_summary: {
            quarantined_findings_count: 9,
            corroborated_promotions_count: 4,
            proposed_effort_rules_count: 2,
            auto_dpo_pairs_minted: 9,
            unreviewed_ssot_mutations: 0,
          },
        });
      }
      return Promise.resolve(null);
    });

    render(<BeforeAfterAuditInspector />);
    await waitFor(() => {
      expect(screen.getByTestId('audit-stage-pill')).toHaveTextContent('before-live to after-live');
      expect(screen.getByTestId('audit-bug-closure-pill')).toHaveTextContent('1/1 Live Bugs Closed');
    });
  });
});
