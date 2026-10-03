import React from 'react';
import type { RouteExplanation, RoutingHealth } from '../../../types/tauri';
import { modeLabel, modeObjective } from '../../../lib/turnEvents';
import { exclusionLabel, priceLabel, qualityLabel } from '../../../lib/routingLabels';

interface Props {
  explanation: RouteExplanation | null;
  health: RoutingHealth | null;
  loading?: boolean;
}

const SEGMENTS: Array<{ key: 'quality' | 'efficiency' | 'latency' | 'other'; color: string }> = [
  { key: 'quality', color: 'var(--color-accent-default)' },
  { key: 'efficiency', color: 'var(--color-accent-secondary)' },
  { key: 'latency', color: 'var(--color-status-info)' },
  { key: 'other', color: 'var(--color-neutral-400)' },
];

export function RoutingExplainer({ explanation, health, loading = false }: Props) {
  // <!-- AMENDED: R8 — skeleton only before the first explanation; reloads keep the old content (no layout
  // shift); `aria-busy` follows `loading`. -->
  if (!explanation) {
    return (
      <div data-testid="routing-explainer-loading" className="min-h-72 rounded-lg bg-overlay-subtle" aria-busy={loading}>
        {!loading && <p className="p-4 text-xs text-text-muted">Routing explanation unavailable</p>}
      </div>
    );
  }
  const excludedTotal = explanation.excluded.reduce((n, g) => n + g.count, 0);
  const problems = health?.violations ?? [];
  const fallback = health && (health.quality_scale === 'fallback' || health.price_bands === 'fallback');
  const healthState = problems.length > 0 ? 'warn' : 'ok';
  return (
    <section data-testid="routing-explainer" aria-labelledby="routing-explainer-title" aria-busy={loading}
      className="flex min-h-72 min-w-0 flex-col gap-3">
      <header>
        <h3 id="routing-explainer-title" className="text-sm text-text-primary">
          Why {explanation.chosen ?? 'no model'}
        </h3>
        <p className="text-xs text-text-muted">How task dispatch would choose now</p>
        <p className="text-xs text-text-muted">
          {modeLabel(explanation.mode)}: {modeObjective(explanation.mode)}
        </p>
        {explanation.only_candidate && (
          <p role="note" className="text-xs" style={{ color: 'var(--color-status-warn)' }}>
            Only this model fits; every model this mode prefers is unavailable.
          </p>
        )}
      </header>
      <div className="overflow-x-auto">
      <table className="w-full table-fixed text-xs tabular-nums">
        <caption className="sr-only">Candidates ranked by routing score</caption>
        <thead>
          <tr className="text-left text-text-muted">
            <th scope="col" className="w-8">#</th>
            <th scope="col">Model</th>
            <th scope="col" className="hidden w-16 sm:table-cell">Tier</th>
            <th scope="col" className="w-28 sm:w-48">Quality</th>
            <th scope="col" className="w-20">Price</th>
            <th scope="col" className="w-16 sm:w-40">Score</th>
          </tr>
        </thead>
        <tbody>
          {explanation.candidates.map((c, i) => {
            const chosen = c.id === explanation.chosen;
            return (
              <tr key={c.id} data-chosen={chosen ? 'true' : undefined}>
                <td>{i + 1}</td>
                <td className="truncate font-mono" title={c.id}>
                  {c.id} {chosen && <span className="ml-1 text-text-primary">Chosen</span>}
                </td>
                <td className="hidden sm:table-cell">{c.tier}</td>
                <td className="truncate whitespace-nowrap" title={qualityLabel(c.quality)}>
                  {(c.quality.value * 100).toFixed(0)} · <span className="text-text-muted">{qualityLabel(c.quality)}</span>
                </td>
                <td title={c.price_out_per_m == null && !c.is_free ? 'price not published' : undefined}>
                  {priceLabel(c.price_out_per_m, c.is_free)}
                </td>
                <td>
                  <span className="mr-2">{c.score.toFixed(2)}</span>
                  <span aria-hidden="true" className="hidden h-1.5 w-24 overflow-hidden rounded align-middle sm:inline-flex">
                    {SEGMENTS.map(s => (
                      <span key={s.key} style={{ width: `${Math.max(0, c.parts[s.key]) * 100}%`, background: s.color }} />
                    ))}
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      </div>
      <details data-testid="routing-excluded" className="text-xs">
        <summary className="cursor-pointer text-text-secondary">Not considered ({excludedTotal})</summary>
        <ul className="mt-1 flex flex-col gap-1">
          {explanation.excluded.map(g => (
            <li key={g.reason}>
              {exclusionLabel(g.reason)} ({g.count})
              <span className="ml-1 font-mono text-text-muted">{g.examples.join(', ')}</span>
            </li>
          ))}
        </ul>
      </details>
      <p data-testid="routing-scope-note" className="text-xs text-text-muted">
        Doesn’t reflect today’s exploration budget or provider usage limits. Telemetry as of the last scoreboard
        refresh.
      </p>
      {health && (
        <footer data-testid="routing-health" data-state={healthState} className="text-xs text-text-muted"
          style={healthState === 'warn' ? { color: 'var(--color-status-warn)' } : undefined}>
          {health.benchmarked} of {health.cloud_models} cloud models benchmarked, {health.inherited} inherited
          {fallback ? ' · using built-in fallback scales' : ' · scales from the live catalog'}
          {problems.map(v => <div key={v.invariant}>{v.detail}</div>)}
        </footer>
      )}
    </section>
  );
}
