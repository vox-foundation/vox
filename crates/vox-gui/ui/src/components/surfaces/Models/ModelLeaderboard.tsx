import React, { useEffect, useState } from 'react';
import { Glass } from '../../ui/Glass';
import { getModelLeaderboard, type BenchRow, type ModeRoute, type ModelLeaderboard as Board } from '../../../transport';

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
  measured: 'border-emerald-400/40 text-emerald-300',
  benchmark: 'border-sky-400/40 text-sky-300',
  inherited: 'border-sky-400/30 text-sky-300/80',
  estimate: 'border-amber-400/40 text-amber-300',
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
  const tone = SOURCE_TONE[route.quality_source] ?? 'border-border-subtle text-text-muted';
  return (
    <span
      aria-label={label}
      title={`${label} · quality ${route.quality != null ? route.quality.toFixed(2) : '—'} · ${route.candidates} candidates`}
      className={`inline-flex max-w-full items-center gap-1 rounded-full border px-2 py-0.5 font-mono text-[10px] ${tone} ${route.only_candidate ? 'border-dashed' : ''}`}
    >
      <span className="truncate">{route.model_id}</span>
      {route.quality_source === 'measured' && <span className="shrink-0 opacity-70">·{route.bench_cases}</span>}
    </span>
  );
}

function fmt(v: number | null, digits = 0, prefix = '', suffix = ''): string {
  return v == null ? '—' : `${prefix}${v.toFixed(digits)}${suffix}`;
}

function BenchTable({ rows }: { rows: BenchRow[] }) {
  return (
    <table aria-label="Benchmark results" className="w-full text-[10px] font-mono">
      <thead className="text-text-muted uppercase tracking-widest">
        <tr>
          <th className="text-left font-normal py-1">model</th>
          <th className="text-right font-normal">pass</th>
          <th className="text-right font-normal">score</th>
          <th className="text-right font-normal">ttft p50 ms</th>
          <th className="text-right font-normal">p95 ms</th>
          <th className="text-right font-normal">tok/s</th>
          <th className="text-right font-normal">$/solve</th>
          <th className="text-right font-normal">tok/solve</th>
          <th className="text-right font-normal">routing</th>
        </tr>
      </thead>
      <tbody>
        {rows.map(r => (
          <tr key={r.model_id} aria-label={r.model_id} className="border-t border-border-subtle/50">
            <td className="py-1 pr-2 text-text-primary">
              <div className="truncate max-w-[22rem]" title={r.model_id}>{r.model_id}</div>
              {r.served_as && <div className="text-text-muted">served as {r.served_as}</div>}
            </td>
            <td className="text-right">{r.passed}/{r.cases}</td>
            <td className="text-right">{r.mean_score.toFixed(2)}{r.error_rate > 0 ? ` · ${Math.round(r.error_rate * 100)}% err` : ''}</td>
            <td className="text-right">{fmt(r.p50_ttft_ms)}</td>
            <td className="text-right">{fmt(r.p95_latency_ms)}</td>
            <td className="text-right">{fmt(r.tokens_per_sec, 1)}</td>
            <td className="text-right">{r.cost_per_solve_usd === 0 ? 'free' : fmt(r.cost_per_solve_usd, 4, '$')}</td>
            <td className="text-right">{fmt(r.tokens_per_solve)}</td>
            <td className={`text-right ${r.confidence === 'confirmed' ? 'text-emerald-300' : r.confidence === 'shadowed' ? 'text-rose-300' : 'text-text-muted'}`}>{r.confidence}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Mode × category routing chips plus the graded bench table that feeds them. */
export function ModelLeaderboard({ board }: { board: Board }) {
  const categories = [...new Set(board.routes.map(r => r.category))];
  const modes = [...new Set(board.routes.map(r => r.mode))].sort(
    (a, b) => MODE_ORDER.indexOf(a) - MODE_ORDER.indexOf(b),
  );
  const cell = (mode: string, category: string) =>
    board.routes.find(r => r.mode === mode && r.category === category);
  return (
    <Glass className="p-4 flex flex-col gap-4">
      <div>
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">Routing by mode</div>
        <div className="text-[10px] text-text-muted mt-1">
          What each mode dispatches per task category right now. Green chips are ranked on graded bench results.
        </div>
      </div>
      <div className="overflow-x-auto">
        <table aria-label="Routing by mode" className="w-full text-[10px]">
          <thead>
            <tr className="text-text-muted uppercase tracking-widest">
              <th className="text-left font-normal py-1">mode</th>
              {categories.map(c => <th key={c} className="text-left font-normal px-1">{c}</th>)}
            </tr>
          </thead>
          <tbody>
            {modes.map(m => (
              <tr key={m} className="border-t border-border-subtle/50">
                <th scope="row" className="text-left font-normal py-1.5 pr-2 text-text-secondary capitalize">{m}</th>
                {categories.map(c => {
                  const r = cell(m, c);
                  return <td key={c} className="px-1 py-1.5 max-w-[14rem]">{r ? <RoutingChip route={r} /> : null}</td>;
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div>
        <div className="font-display text-[11px] tracking-[0.2em] uppercase text-text-muted">
          Benchmarks · last {board.window_days} days
        </div>
        {board.bench.length === 0 ? (
          <div className="mt-2 text-[11px] text-text-muted">
            No graded runs yet. Measure models with <code className="font-mono text-brass">vox model bench --max-usd 1</code>.
          </div>
        ) : (
          <div className="mt-2 overflow-x-auto"><BenchTable rows={board.bench} /></div>
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
