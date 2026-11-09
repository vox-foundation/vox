import React from 'react';
import type { RouteExplanation, RoutingHealth } from '../../../types/tauri';
import { modeLabel, modeObjective } from '../../../lib/turnEvents';
import { exclusionLabel, priceLabel, qualityLabel } from '../../../lib/routingLabels';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';

interface Props {
  explanation: RouteExplanation | null;
  health: RoutingHealth | null;
  loading?: boolean;
}

const SEGMENTS: Array<{ key: 'quality' | 'efficiency' | 'latency' | 'other'; color: string }> = [
  { key: 'quality', color: '#D4AF37' },
  { key: 'efficiency', color: '#B45309' },
  { key: 'latency', color: '#92400E' },
  { key: 'other', color: '#78350F' },
];

export function RoutingExplainer({ explanation, health, loading = false }: Props) {
  // <!-- AMENDED: R8 — skeleton only before the first explanation; reloads keep the old content (no layout
  // shift); `aria-busy` follows `loading`. -->
  if (!explanation) {
    return (
      <div
        data-testid="routing-explainer-loading"
        className="min-h-36 rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-4 flex items-center justify-center shadow-inner"
        aria-busy={loading}
      >
        {!loading && <p className="text-xs text-text-muted">Routing explanation unavailable</p>}
      </div>
    );
  }
  const excludedTotal = explanation.excluded.reduce((n, g) => n + g.count, 0);
  const problems = health?.violations ?? [];
  const fallback = health && (health.quality_scale === 'fallback' || health.price_bands === 'fallback');
  const healthState = problems.length > 0 ? 'warn' : 'ok';
  return (
    <section
      data-testid="routing-explainer"
      aria-labelledby="routing-explainer-title"
      aria-busy={loading}
      className="flex min-h-36 min-w-0 flex-col gap-3 rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-4 shadow-[0_4px_16px_rgba(0,0,0,0.2)]"
    >
      <header className="space-y-1">
        <div className="flex items-center justify-between">
          <h2 id="routing-explainer-title" className="text-sm font-display tracking-wide text-text-primary font-bold">
            Why {explanation.chosen ?? 'no model'}
          </h2>
          <RomanWaxSealBadge label="CALIBRATED" tone="gold" />
        </div>
        <p className="text-xs text-text-secondary font-medium">How task dispatch would choose now</p>
        <p className="text-xs text-text-muted">
          {modeLabel(explanation.mode)}: {modeObjective(explanation.mode)}
        </p>
        {explanation.only_candidate && (
          <p role="note" className="text-xs text-[#FCA5A5] bg-[#7F1D1D]/20 border border-[#FCA5A5]/30 rounded px-2.5 py-1">
            Only this model fits; every model this mode prefers is unavailable.
          </p>
        )}
      </header>
      <div className="overflow-x-auto rounded-lg border border-[#C59B27]/25 bg-bg-elevated/60">
        <table className="w-full min-w-[30rem] table-fixed text-xs tabular-nums">
          <caption className="sr-only">Candidates ranked by routing score</caption>
          <thead>
            <tr className="text-left text-text-muted uppercase tracking-wider border-b border-[#C59B27]/25 bg-bg-elevated/80">
              <th scope="col" className="w-8 py-2 px-2.5">#</th>
              <th scope="col" className="w-48 sm:w-60 py-2 px-2">Model</th>
              <th scope="col" className="hidden w-20 sm:table-cell py-2 px-2">Tier</th>
              <th scope="col" className="w-28 sm:w-44 py-2 px-2">Quality</th>
              <th scope="col" className="w-20 py-2 px-2">Price</th>
              <th scope="col" className="w-24 sm:w-40 py-2 px-2">Score</th>
            </tr>
          </thead>
          <tbody>
            {explanation.candidates.map((c, i) => {
              const chosen = c.id === explanation.chosen;
              return (
                <tr
                  key={c.id}
                  data-chosen={chosen ? 'true' : undefined}
                  className={`border-b border-[#D4AF37]/15 transition-colors ${
                    chosen ? 'bg-[#D4AF37]/10' : 'hover:bg-[#D4AF37]/5'
                  }`}
                >
                  <td className="py-2 px-2.5 text-text-muted">{i + 1}</td>
                  <td className="truncate font-mono py-2 px-2 text-text-primary" title={c.id}>
                    {c.id} {chosen && (
                      <span className="ml-1 inline-flex items-center gap-1">
                        <RomanWaxSealBadge label="Chosen" tone="gold" className="text-[8px] py-0 px-1.5" />
                      </span>
                    )}
                  </td>
                  <td className="hidden sm:table-cell py-2 px-2 text-text-secondary">{c.tier}</td>
                  <td className="truncate whitespace-nowrap py-2 px-2 text-text-primary" title={qualityLabel(c.quality)}>
                    {(c.quality.value * 100).toFixed(0)} · <span className="text-text-muted">{qualityLabel(c.quality)}</span>
                  </td>
                  <td className="py-2 px-2 text-text-primary" title={c.price_out_per_m == null && !c.is_free ? 'price not published' : undefined}>
                    {priceLabel(c.price_out_per_m, c.is_free)}
                  </td>
                  <td className="py-2 px-2 text-text-primary">
                    <span className="mr-2 font-mono">{c.score.toFixed(2)}</span>
                    <span aria-hidden="true" className="hidden h-2 w-24 overflow-hidden rounded-full border border-[#D4AF37]/30 bg-bg-elevated align-middle sm:inline-flex">
                      {SEGMENTS.map(s => (
                        <span
                          key={s.key}
                          style={{ width: `${Math.max(0, c.parts[s.key]) * 100}%` }}
                          className={`h-full ${
                            s.key === 'quality'
                              ? 'bg-gradient-to-r from-[#B45309] via-[#D4AF37] to-[#F3E5AB]'
                              : s.key === 'efficiency'
                                ? 'bg-[#D4AF37]'
                                : s.key === 'latency'
                                  ? 'bg-[#92400E]'
                                  : 'bg-[#78350F]'
                          }`}
                        />
                      ))}
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <details data-testid="routing-excluded" className="text-xs rounded-lg border border-[#D4AF37]/25 bg-bg-elevated/80 p-2.5">
        <summary className="cursor-pointer font-display text-[10px] uppercase tracking-wider text-brass hover:text-text-primary">
          Not considered ({excludedTotal})
        </summary>
        <ul className="mt-2 flex flex-col gap-1 pl-2">
          {explanation.excluded.map(g => (
            <li key={g.reason} className="text-text-secondary">
              {exclusionLabel(g.reason)} ({g.count})
              <span className="ml-1 font-mono text-text-muted">{g.examples.join(', ')}</span>
            </li>
          ))}
        </ul>
      </details>
      <p data-testid="routing-scope-note" className="text-xs text-[#8C806B]">
        Doesn’t reflect today’s exploration budget or provider usage limits. Telemetry as of the last scoreboard
        refresh.
      </p>
      {health && (
        <footer
          data-testid="routing-health"
          data-state={healthState}
          className="text-xs text-[#8C806B] pt-2 border-t border-[#D4AF37]/20"
          style={healthState === 'warn' ? { color: 'var(--color-status-warn)' } : undefined}
        >
          {health.benchmarked} of {health.cloud_models} cloud models benchmarked, {health.inherited} inherited
          {fallback ? ' · using built-in fallback scales' : ' · scales from the live catalog'}
          {problems.map(v => <div key={v.invariant} className="text-[#FCA5A5] mt-0.5">{v.detail}</div>)}
        </footer>
      )}
    </section>
  );
}
