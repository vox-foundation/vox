// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import { RoutingExplainer } from './RoutingExplainer';
import type { RouteExplanation, RoutingHealth } from '../../../types/tauri';

const parts = { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 };
const explanation: RouteExplanation = {
  mode: 'efficiency', task: 'codegen', complexity: 7, chosen: 'acme/widget-5.5', only_candidate: false,
  total_models: 120,
  candidates: [
    { id: 'acme/widget-5.5', provider: 'acme', tier: 'Fast', is_free: false, price_out_per_m: 0.9, score: 0.65,
      quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' }, parts },
    { id: 'acme/gadget-2', provider: 'acme', tier: 'Pro', is_free: false, price_out_per_m: null, score: 0.61,
      quality: { value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null }, parts },
  ],
  excluded: [
    { reason: 'flagship_excluded_by_mode', count: 4, examples: ['acme/flagship-9', 'acme/flagship-8', 'acme/flagship-7'] },
    { reason: 'superseded', count: 1, examples: ['acme/widget-4.8'] },
  ],
};
const healthy: RoutingHealth = {
  schema_version: 1, checked_at_unix: 0, models: 120, cloud_models: 100, benchmarked: 40, inherited: 7,
  unknown_tier_cloud: 0, quality_scale: 'derived', price_bands: 'derived', violations: [],
};

describe('RoutingExplainer', () => {
  it('leads with the chosen model and the mode in plain language', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    expect(screen.getByRole('heading', { name: /why acme\/widget-5\.5/i })).toBeTruthy();
    expect(screen.getByText(/^Efficient/)).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/intel=|eff=|lat=|_/);
  });

  // <!-- AMENDED: G8, G9, G13 — the panel explains task dispatch (not chat) and says what it cannot see. -->
  it('says what it explains and what it does not see', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    expect(screen.getByText(/How task dispatch would choose now/)).toBeTruthy();
    const scope = screen.getByTestId('routing-scope-note').textContent ?? '';
    expect(scope).toMatch(/exploration budget or provider usage limits/);
    expect(scope).toMatch(/last scoreboard refresh/);
  });

  it('ranks candidates in a real table and marks the choice in text', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    const table = screen.getByRole('table', { name: /candidates ranked by routing score/i });
    const rows = within(table).getAllByRole('row').slice(1);
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText('Chosen')).toBeTruthy();
    expect(within(rows[0]).getByText('from acme/widget-5.0 (38.2)')).toBeTruthy();
    expect(within(rows[1]).getByText('benchmark 46.3')).toBeTruthy();
    expect(within(rows[1]).getByText('—')).toBeTruthy();
    expect(within(table).getAllByRole('columnheader').every(h => h.getAttribute('scope') === 'col')).toBe(true);
  });

  it('keeps the excluded models behind a collapsed disclosure with counts', () => {
    render(<RoutingExplainer explanation={explanation} health={healthy} />);
    const details = screen.getByTestId('routing-excluded') as HTMLDetailsElement;
    expect(details.open).toBe(false);
    expect(within(details).getByText(/not considered \(5\)/i)).toBeTruthy();
    expect(within(details).getByText(/flagship.*\(4\)/i)).toBeTruthy();
  });

  it('says so when only a fallback model fits', () => {
    render(<RoutingExplainer explanation={{ ...explanation, only_candidate: true }} health={healthy} />);
    expect(screen.getByRole('note').textContent).toMatch(/only/i);
  });

  it('draws attention to health only when something is wrong', () => {
    const { rerender } = render(<RoutingExplainer explanation={explanation} health={healthy} />);
    expect(screen.getByTestId('routing-health').getAttribute('data-state')).toBe('ok');
    rerender(<RoutingExplainer explanation={explanation} health={{ ...healthy, price_bands: 'fallback',
      violations: [{ invariant: 'tiers_known', detail: '30 of 100 cloud models have no tier' }] }} />);
    const footer = screen.getByTestId('routing-health');
    expect(footer.getAttribute('data-state')).toBe('warn');
    expect(footer.textContent).toContain('30 of 100 cloud models have no tier');
    expect(footer.textContent).toMatch(/built-in fallback/);
  });

  it('renders a placeholder only before the first explanation, and keeps content while reloading', () => {
    const { rerender } = render(<RoutingExplainer explanation={null} health={null} loading />);
    expect(screen.getByTestId('routing-explainer-loading').getAttribute('aria-busy')).toBe('true');
    rerender(<RoutingExplainer explanation={explanation} health={healthy} loading />);
    expect(screen.queryByTestId('routing-explainer-loading')).toBeNull();
    expect(screen.getByTestId('routing-explainer').getAttribute('aria-busy')).toBe('true');
  });
});
