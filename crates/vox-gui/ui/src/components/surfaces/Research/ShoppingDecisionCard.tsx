import React from 'react';

export interface ShoppingCriterion {
  name: string;
  weight: number;
}

export interface ShoppingOption {
  name: string;
  total_score?: number;
  overall_score?: number;
  pros?: string[];
  cons?: string[];
  criterion_scores?: Record<string, number>;
  criteria_scores?: Record<string, number>;
  citation_urls?: string[];
}

export interface ShoppingMatrixDto {
  query?: string;
  criteria?: Array<ShoppingCriterion | string>;
  options?: ShoppingOption[];
  recommendation?: string;
  recommended_option?: string;
  confidence?: number;
}

export type ShoppingDecisionPayload = ShoppingMatrixDto;

export function ShoppingDecisionCard({
  matrix,
  decision,
}: {
  matrix?: ShoppingMatrixDto;
  decision?: ShoppingMatrixDto;
}) {
  const payload = matrix ?? decision;
  if (!payload) return null;
  const criteria = payload.criteria ?? [];
  const options = payload.options ?? [];
  const rec = payload.recommendation ?? payload.recommended_option;
  if (criteria.length === 0 && options.length === 0 && !rec) {
    return null;
  }

  return (
    <div
      data-testid="shopping-decision-card"
      className="mt-3 rounded-xl border border-brass/30 bg-brass/[0.04] p-4 space-y-3"
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="rounded bg-brass/20 px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider text-brass">
            Shopping Matrix
          </span>
          <span className="font-display text-xs uppercase tracking-wider text-text-primary">
            Multi-Criteria Decision Analysis
          </span>
        </div>
        {rec && (
          <span className="font-mono text-xs text-emerald-300">
            ★ Pick: {rec}
          </span>
        )}
      </div>

      {criteria.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {criteria.map((c) => {
            const label = typeof c === 'string' ? c : c.name;
            const weight = typeof c === 'string' ? null : c.weight;
            return (
              <span
                key={label}
                className="rounded border border-border-subtle bg-black/30 px-2 py-0.5 font-mono text-[10px] text-text-secondary"
              >
                {label}
                {weight != null && (
                  <>
                    {' '}
                    <span className="text-brass">{Math.round(weight * 100)}%</span>
                  </>
                )}
              </span>
            );
          })}
        </div>
      )}

      {options.length > 0 && (
        <div className="grid gap-2 sm:grid-cols-2">
          {options.map((opt, idx) => (
            <div
              key={opt.name}
              className={`rounded-lg border p-3 ${
                idx === 0
                  ? 'border-emerald-500/40 bg-emerald-500/[0.06]'
                  : 'border-border-subtle bg-black/30'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium text-xs text-text-primary">
                  {idx === 0 ? '🏆 ' : ''}
                  {opt.name}
                </span>
                <span className="font-mono text-xs text-brass">
                  {((opt.total_score ?? opt.overall_score ?? 0) * 100).toFixed(0)} pts
                </span>
              </div>
              {opt.pros && opt.pros.length > 0 && (
                <div className="mt-1.5 text-[11px] text-emerald-300/90">
                  + {opt.pros.slice(0, 2).join(' · ')}
                </div>
              )}
              {opt.cons && opt.cons.length > 0 && (
                <div className="mt-0.5 text-[11px] text-rose-300/80">
                  − {opt.cons.slice(0, 2).join(' · ')}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
