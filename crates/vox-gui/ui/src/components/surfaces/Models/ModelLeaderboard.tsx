import React, { useEffect, useState } from 'react';
import { Glass } from '../../ui/Glass';
import { getModelLeaderboard, type BenchRow, type ModeRoute, type ModelLeaderboard as Board } from '../../../transport';
import { RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';

const MODE_ORDER = ['free', 'efficiency', 'balanced', 'genius'];

function provenance(r: ModeRoute): string {
  switch (r.quality_source) {
    case 'measured': return `measured on ${r.bench_cases} cases`;
    case 'benchmark': return 'public index';
    case 'inherited': return 'index inherited from base model';
    case 'estimate': return 'estimated quality';
    default: return 'no quality signal';
  }
}

const SOURCE_TONE: Record<string, string> = {
  measured: 'border-emerald-600/50 text-emerald-700 bg-emerald-500/15',
  benchmark: 'border-[#D4AF37]/50 text-text-primary bg-[#D4AF37]/10',
  inherited: 'border-[#D4AF37]/30 text-brass bg-bg-elevated',
  estimate: 'border-amber-500/50 text-amber-700 bg-amber-500/15',
};

function RoutingChip({ route }: { route: ModeRoute }) {
  if (!route.model_id) {
    return (
      <span aria-label={`${route.mode} ${route.category} routes nowhere (no reachable model)`} className="text-[10px] text-text-muted">
        no route
      </span>
    );
  }
  const label = `${route.mode} ${route.category} routes to ${route.model_id}, ${provenance(route)}${route.only_candidate ? ", only candidate outside the mode's tiers" : ''}`;
  const tone = SOURCE_TONE[route.quality_source] ?? 'border-[#D4AF37]/30 text-text-secondary';
  return (
    <span
      aria-label={label}
      title={`${label} · quality ${route.quality != null ? route.quality.toFixed(2) : '—'} · ${route.candidates} candidates`}
      className={`inline-flex max-w-full items-center gap-1 rounded-full border px-2 py-0.5 font-mono text-[10px] ${tone} ${route.only_candidate ? 'border-dashed' : ''}`}
    >
      <span className="truncate">{route.model_id}</span>
      {route.quality_source === 'measured' && <span className="shrink-0 opacity-70">·{route.bench_cases}</span>}
      {route.only_candidate && <RomanWaxSealBadge label="PINNED" tone="gold" className="text-[8px] py-0 px-1.5" />}
    </span>
  );
}

function fmt(v: number | null, digits = 0, prefix = '', suffix = ''): string {
  return v == null ? '—' : `${prefix}${v.toFixed(digits)}${suffix}`;
}

function BenchTable({ rows }: { rows: BenchRow[] }) {
  return (
    <table aria-label="Benchmark results" className="w-full text-[10px] font-mono">
      <thead className="text-text-muted uppercase tracking-widest border-b border-[#C59B27]/25 bg-bg-elevated/80">
        <tr>
          <th className="text-left font-normal py-1.5 px-2">model</th>
          <th className="text-right font-normal py-1.5 px-2">pass</th>
          <th className="text-right font-normal py-1.5 px-2">score</th>
          <th className="text-right font-normal py-1.5 px-2">ttft p50 ms</th>
          <th className="text-right font-normal py-1.5 px-2">p95 ms</th>
          <th className="text-right font-normal py-1.5 px-2">tok/s</th>
          <th className="text-right font-normal py-1.5 px-2">$/solve</th>
          <th className="text-right font-normal py-1.5 px-2">tok/solve</th>
          <th className="text-right font-normal py-1.5 px-2">routing</th>
        </tr>
      </thead>
      <tbody>
        {rows.map(r => (
          <tr key={r.model_id} aria-label={r.model_id} className="border-t border-[#D4AF37]/15 hover:bg-[#D4AF37]/5 transition-colors">
            <td className="py-2 px-2 text-text-primary">
              <div className="truncate max-w-[22rem] font-medium" title={r.model_id}>{r.model_id}</div>
              {r.served_as && <div className="text-text-muted text-[9px]">served as {r.served_as}</div>}
            </td>
            <td className="text-right py-2 px-2 text-text-secondary">{r.passed}/{r.cases}</td>
            <td className="text-right py-2 px-2 text-text-secondary">{r.mean_score.toFixed(2)}{r.error_rate > 0 ? ` · ${Math.round(r.error_rate * 100)}% err` : ''}</td>
            <td className="text-right py-2 px-2 text-text-secondary">{fmt(r.p50_ttft_ms)}</td>
            <td className="text-right py-2 px-2 text-text-secondary">{fmt(r.p95_latency_ms)}</td>
            <td className="text-right py-2 px-2 text-text-secondary">{fmt(r.tokens_per_sec, 1)}</td>
            <td className="text-right py-2 px-2 text-text-secondary">{r.cost_per_solve_usd === 0 ? 'free' : fmt(r.cost_per_solve_usd, 4, '$')}</td>
            <td className="text-right py-2 px-2 text-text-secondary">{fmt(r.tokens_per_solve)}</td>
            <td className="text-right py-2 px-2">
              <span className={`inline-flex items-center gap-1 ${r.confidence === 'confirmed' ? 'text-emerald-700' : r.confidence === 'shadowed' ? 'text-rose-700' : 'text-text-muted'}`}>
                <RomanWaxSealBadge
                  label={r.confidence}
                  tone={r.confidence === 'confirmed' ? 'emerald' : r.confidence === 'shadowed' ? 'crimson' : 'gold'}
                  className="text-[8px] py-0 px-1.5"
                />
              </span>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Mode × category routing chips plus the graded bench table that feeds them. */
export function ModelLeaderboard({ board }: { board: Board }) {
  const [benchFilter, setBenchFilter] = useState<'all' | 'free'>('all');
  const categories = [...new Set(board.routes.map(r => r.category))];
  const modes = [...new Set(board.routes.map(r => r.mode))].sort(
    (a, b) => MODE_ORDER.indexOf(a) - MODE_ORDER.indexOf(b),
  );
  const cell = (mode: string, category: string) =>
    board.routes.find(r => r.mode === mode && r.category === category);

  const displayBench = benchFilter === 'free'
    ? board.bench.filter(r => r.cost_per_solve_usd === 0)
    : board.bench;

  return (
    <Glass className="p-4 flex flex-col gap-4 border-[#D4AF37]/35 bg-bg-surface shadow-[0_4px_20px_rgba(0,0,0,0.2)]">
      <div>
        <div className="flex items-center justify-between">
          <div className="font-display text-[11px] tracking-[0.2em] uppercase text-brass">Routing by mode</div>
          <RomanWaxSealBadge label="VERIFIED" tone="emerald" />
        </div>
        <div className="text-[10px] text-text-secondary mt-1">
          What each mode dispatches per task category right now. Green chips are ranked on graded bench results.
        </div>
      </div>
      <div className="overflow-x-auto rounded-lg border border-[#C59B27]/25 bg-bg-elevated/60">
        <table aria-label="Routing by mode" className="w-full text-[10px]">
          <thead>
            <tr className="text-text-muted uppercase tracking-widest border-b border-[#C59B27]/25 bg-bg-elevated/80">
              <th className="text-left font-normal py-2 px-2.5">mode</th>
              {categories.map(c => <th key={c} className="text-left font-normal px-2 py-2">{c}</th>)}
            </tr>
          </thead>
          <tbody>
            {modes.map(m => (
              <tr key={m} className="border-t border-[#D4AF37]/15 hover:bg-[#D4AF37]/5 transition-colors">
                <th scope="row" className="text-left font-normal py-2 px-2.5 text-text-primary capitalize font-display">{m}</th>
                {categories.map(c => {
                  const r = cell(m, c);
                  return <td key={c} className="px-2 py-2 max-w-[14rem]">{r ? <RoutingChip route={r} /> : null}</td>;
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div>
        <div className="flex flex-wrap items-center justify-between gap-2 mb-2">
          <div className="font-display text-[11px] tracking-[0.2em] uppercase text-brass">
            Benchmarks · last {board.window_days} days
          </div>
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={() => setBenchFilter('all')}
              className={`rounded-full border px-2.5 py-0.5 font-display text-[9px] uppercase tracking-wider transition-all ${
                benchFilter === 'all'
                  ? 'border-[#D4AF37] bg-[#D4AF37]/20 text-text-primary shadow-sm font-semibold'
                  : 'border-[#D4AF37]/30 bg-bg-elevated text-text-secondary hover:text-text-primary'
              }`}
            >
              All Models
            </button>
            <button
              type="button"
              onClick={() => setBenchFilter('free')}
              className={`rounded-full border px-2.5 py-0.5 font-display text-[9px] uppercase tracking-wider transition-all ${
                benchFilter === 'free'
                  ? 'border-[#D4AF37] bg-[#D4AF37]/20 text-text-primary shadow-sm font-semibold'
                  : 'border-[#D4AF37]/30 bg-bg-elevated text-text-secondary hover:text-text-primary'
              }`}
            >
              Free Tier Only
            </button>
          </div>
        </div>
        {displayBench.length === 0 ? (
          <div className="mt-2 text-[11px] text-text-secondary p-3 rounded-lg border border-[#D4AF37]/20 bg-bg-elevated/60">
            No graded runs yet. Measure models with <code className="font-mono text-brass">vox model bench --max-usd 1</code>.
          </div>
        ) : (
          <div className="mt-2 overflow-x-auto rounded-lg border border-[#D4AF37]/25 bg-bg-elevated/60"><BenchTable rows={displayBench} /></div>
        )}
      </div>
    </Glass>
  );
}

/** Fetching wrapper for the Models surface; renders nothing without a backend. */
export function ModelLeaderboardPanel() {
  const [board, setBoard] = useState<Board | null>(null);
  useEffect(() => {
    let live = true;
    getModelLeaderboard()
      .then(b => { if (live && b && Array.isArray(b.routes)) setBoard(b); })
      .catch(() => {});
    return () => { live = false; };
  }, []);
  if (!board || board.routes.length === 0) return null;
  return <ModelLeaderboard board={board} />;
}
