import { describe, it, expect } from 'vitest';
import { exclusionLabel, qualityLabel, priceLabel } from './routingLabels';

describe('routingLabels', () => {
  it('names every exclusion reason in plain English and never shows a code name', () => {
    for (const kind of ['penalized', 'free_in_performance_mode', 'over_request_cost_cap', 'over_task_budget',
      'route_policy', 'privacy_local_only', 'strength_mismatch', 'filtered', 'no_provider_key',
      'flagship_excluded_by_mode', 'not_free', 'exploration_budget_spent', 'provider_budget_exhausted',
      'superseded']) {
      const label = exclusionLabel(kind);
      expect(label).not.toContain('_');
      expect(label.length).toBeGreaterThan(3);
    }
    expect(exclusionLabel('something_new')).toBe('Other');
  });

  it('says where a quality number came from', () => {
    expect(qualityLabel({ value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null })).toBe('benchmark 46.3');
    expect(qualityLabel({ value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' }))
      .toBe('from acme/widget-5.0 (38.2)');
    expect(qualityLabel({ value: 0.4, source: 'estimate', index: null, inherited_from: null })).toBe('estimate');
    expect(qualityLabel({ value: 0, source: 'unknown', index: null, inherited_from: null })).toBe('—');
  });

  it('formats price per million and never shows 0 for an unknown price', () => {
    expect(priceLabel(0.9, false)).toBe('$0.90/M');
    expect(priceLabel(20, false)).toBe('$20.00/M');
    expect(priceLabel(0, true)).toBe('free');
    expect(priceLabel(null, false)).toBe('—');
  });
});
