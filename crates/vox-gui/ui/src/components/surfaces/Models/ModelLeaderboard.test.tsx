// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import React from 'react';
import { ModelLeaderboard } from './ModelLeaderboard';
import type { ModelLeaderboard as Board } from '../../../transport';

const board: Board = {
  window_days: 30,
  routes: [
    { mode: 'balanced', category: 'chat', model_id: 'acme/fast-1', only_candidate: false, candidates: 6, quality: 0.91, quality_source: 'measured', bench_cases: 17 },
    { mode: 'balanced', category: 'codegen', model_id: 'acme/coder-2', only_candidate: false, candidates: 4, quality: 0.7, quality_source: 'benchmark', bench_cases: 0 },
    { mode: 'free', category: 'chat', model_id: null, only_candidate: false, candidates: 0, quality: null, quality_source: 'none', bench_cases: 0 },
    { mode: 'free', category: 'codegen', model_id: 'acme/free:free', only_candidate: true, candidates: 1, quality: 0.4, quality_source: 'estimate', bench_cases: 0 },
  ],
  bench: [
    { model_id: 'acme/fast-1', served_as: 'acme/fast-1-2026-09', cases: 17, passed: 15, mean_score: 0.9, error_rate: 0, p50_ttft_ms: 310, p95_latency_ms: 2400, tokens_per_sec: 88.5, cost_per_solve_usd: 0.0012, tokens_per_solve: 210, confidence: 'confirmed', last_run_ms: 1 },
    { model_id: 'acme/flaky', served_as: null, cases: 12, passed: 2, mean_score: 0.2, error_rate: 0.25, p50_ttft_ms: null, p95_latency_ms: null, tokens_per_sec: null, cost_per_solve_usd: null, tokens_per_solve: null, confidence: 'shadowed', last_run_ms: 1 },
  ],
};

describe('ModelLeaderboard', () => {
  it('renders one routing chip per mode and category with quality provenance', () => {
    render(<ModelLeaderboard board={board} />);
    const grid = screen.getByRole('table', { name: /routing by mode/i });
    expect(within(grid).getByRole('columnheader', { name: /chat/i })).toBeInTheDocument();
    expect(within(grid).getByLabelText(/balanced chat routes to acme\/fast-1, measured on 17 cases/i)).toHaveTextContent('acme/fast-1');
    expect(within(grid).getByLabelText(/balanced codegen routes to acme\/coder-2, public index/i)).toBeInTheDocument();
    expect(within(grid).getByLabelText(/free chat routes nowhere/i)).toHaveTextContent(/no route/i);
    expect(within(grid).getByLabelText(/free codegen .*outside the mode's tiers/i)).toBeInTheDocument();
  });

  it('lists bench results with responsiveness, efficiency and routing status', () => {
    render(<ModelLeaderboard board={board} />);
    const table = screen.getByRole('table', { name: /benchmark results/i });
    const fast = within(table).getByRole('row', { name: /acme\/fast-1/ });
    expect(fast).toHaveTextContent('15/17');
    expect(fast).toHaveTextContent('310');
    expect(fast).toHaveTextContent('$0.0012');
    expect(fast).toHaveTextContent(/served as acme\/fast-1-2026-09/);
    expect(fast).toHaveTextContent(/confirmed/);
    const flaky = within(table).getByRole('row', { name: /acme\/flaky/ });
    expect(flaky).toHaveTextContent(/shadowed/);
    expect(flaky).toHaveTextContent(/25% err/);
  });

  it('points to the bench command when nothing has been measured', () => {
    render(<ModelLeaderboard board={{ ...board, bench: [] }} />);
    expect(screen.getByText(/vox model bench/)).toBeInTheDocument();
  });
});
