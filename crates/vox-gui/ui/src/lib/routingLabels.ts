import type { RouteQuality } from '../types/tauri';

/** Plain-English reason for each `Exclusion` kind (Rust: `models::ranking::Exclusion`). */
const EXCLUSION_LABELS: Readonly<Record<string, string>> = Object.freeze({
  penalized: 'Recently failed this kind of task',
  free_in_performance_mode: 'Free model skipped in this mode',
  over_request_cost_cap: 'Over the per-request cost cap',
  over_task_budget: 'Over this task’s budget',
  route_policy: 'Excluded by routing policy',
  privacy_local_only: 'Cloud model, local-only privacy is on',
  strength_mismatch: 'Not suited to this task',
  filtered: 'Excluded by this request',
  no_provider_key: 'No API key for its provider',
  flagship_excluded_by_mode: 'Flagship, kept out by this mode',
  not_free: 'Paid model, Free mode is on',
  exploration_budget_spent: 'Unpriced, today’s exploration budget is spent',
  provider_budget_exhausted: 'Its provider’s usage budget is spent',
  superseded: 'A newer version is available',
});

export function exclusionLabel(kind: string): string {
  return EXCLUSION_LABELS[kind] ?? 'Other';
}

export function qualityLabel(q: RouteQuality): string {
  switch (q.source) {
    case 'benchmark': return `benchmark ${q.index?.toFixed(1)}`;
    case 'inherited': return `from ${q.inherited_from} (${q.index?.toFixed(1)})`;
    case 'estimate': return 'estimate';
    default: return '—';
  }
}

export function priceLabel(perMillion: number | null, isFree: boolean): string {
  if (isFree) return 'free';
  if (perMillion == null) return '—';
  return `$${perMillion.toFixed(2)}/M`;
}
