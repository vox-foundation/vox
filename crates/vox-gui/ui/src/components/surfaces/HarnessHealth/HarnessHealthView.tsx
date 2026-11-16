import React from 'react';
import { useVoxQuery } from '../../../hooks/useVoxQuery';
import { voxTransport } from '../../../transport';
import { BentoCard, BentoGrid, BentoScrollRegion, MasterDetailWorkbench } from '../../ui/BentoGrid';
import { EmptyState } from '../../ui/EmptyState';
import { Icon } from '../../ui/Icons';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';

export function HarnessHealthView() {
  const [runFilter, setRunFilter] = React.useState<'all' | 'passing' | 'failing'>('all');
  const [categoryFilter, setCategoryFilter] = React.useState<string | null>(null);
  const [searchQuery, setSearchQuery] = React.useState('');
  const [selectedRunId, setSelectedRunId] = React.useState<string | null>(null);
  const [emptyFocusCard, setEmptyFocusCard] = React.useState<'benchmark' | 'regressions' | 'cli' | null>(null);

  const { data: runs, isLoading } = useVoxQuery(
    ['harnessEvalHistory'],
    () => voxTransport.harnessEvalHistory(50),
  );
  const { data: regressions } = useVoxQuery(
    ['harnessEvalRegressions'],
    () => voxTransport.harnessEvalRegressions(),
  );

  if (isLoading) {
    return <p className="p-4 text-[12px] text-text-muted">Loading harness eval history…</p>;
  }

  const emptyDeck = (
    <BentoGrid
      preset="triad"
      data-testid="harness-health-architecture-deck"
      className="w-full text-left"
    >
      <BentoCard
        className={emptyFocusCard === 'benchmark' ? 'border-brass' : 'border-border-subtle'}
      >
        <div className="flex items-center justify-between gap-2">
          <span className="font-display text-[10px] font-semibold tracking-widest text-brass uppercase">
            5-Spoke Benchmark
          </span>
          <span className="rounded border border-brass/30 bg-brass/10 px-1.5 py-0.5 font-mono text-[9px] text-brass">
            SSOT v1
          </span>
        </div>
        <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
          Evaluates chat routing, privacy guards, MCP tool-calling precision, context-window compaction, and latency budgets.
        </p>
        <div className="mt-2.5 flex flex-wrap gap-1 font-mono text-[9px] text-text-muted">
          <span className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5">chat</span>
          <span className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5">privacy</span>
          <span className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5">tool-calling</span>
        </div>
      </BentoCard>

      <BentoCard
        className={emptyFocusCard === 'regressions' ? 'border-brass' : 'border-border-subtle'}
      >
        <div className="flex items-center justify-between gap-2">
          <span className="font-display text-[10px] font-semibold tracking-widest text-cyan uppercase">
            Regression Oracles
          </span>
          <span className="rounded border border-cyan/30 bg-cyan/10 px-1.5 py-0.5 font-mono text-[9px] text-cyan">
            Active
          </span>
        </div>
        <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
          Automatically flags pass-rate drops, flipped task IDs, and suspect git commit ranges across consecutive runs.
        </p>
        <div className="mt-2.5 inline-flex items-center gap-1 font-mono text-[10px] text-emerald-300/90">
          <Icon.check className="size-3 shrink-0" aria-hidden="true" />
          <span>0 active regressions in baseline</span>
        </div>
      </BentoCard>

      <BentoCard
        className={emptyFocusCard === 'cli' ? 'border-brass' : 'border-border-subtle'}
      >
        <div className="flex items-center justify-between gap-2">
          <span className="font-display text-[10px] font-semibold tracking-widest text-text-primary uppercase">
            CLI Quick-Start
          </span>
          <span className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 font-mono text-[9px] text-text-muted">
            local / CI
          </span>
        </div>
        <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
          Trigger a deterministic smoke pass or full live model sweep from the terminal or Command Catalog:
        </p>
        <div className="mt-2 rounded border border-border-subtle bg-bg-base px-2 py-1 font-mono text-[10px] text-brass">
          $ vox harness eval --live
        </div>
      </BentoCard>
    </BentoGrid>
  );

  if (!runs || runs.length === 0) {
    const regCount = regressions?.length ?? 0;
    return (
      <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4">
        <RomanSurfaceHeader
          domainTag="DIAGNOSTICS · HARNESS HEALTH"
          title="Harness Evaluation & Regression Oracles"
          subtitle="5-SPOKE BENCHMARK · REGRESSION DETECTION · PASS/FAIL TELEMETRY"
          waxSealBadge="EVAL GATE"
          icon={<Icon.bolt className="size-4 text-[#F3E5AB]" />}
          gauges={[
            {
              label: 'Runs',
              valueText: '0',
              ratio: 0,
              tone: 'gold',
              purpose: 'Recorded chat harness evaluation runs',
              actionHint: 'Click to highlight CLI quick-start instructions',
              active: emptyFocusCard === 'cli',
              onClick: () => setEmptyFocusCard((prev) => (prev === 'cli' ? null : 'cli')),
            },
            {
              label: 'Regressions',
              valueText: `${regCount}`,
              ratio: regCount > 0 ? 1 : 0,
              tone: regCount > 0 ? 'crimson' : 'emerald',
              purpose: 'Active commit-range regression flags',
              actionHint: 'Click to highlight Regression Oracles overview',
              active: emptyFocusCard === 'regressions',
              onClick: () =>
                setEmptyFocusCard((prev) => (prev === 'regressions' ? null : 'regressions')),
            },
            {
              label: 'Spokes',
              valueText: '0/5',
              ratio: 0,
              tone: 'gold',
              purpose: 'Evaluated benchmark spoke categories',
              actionHint: 'Click to highlight 5-Spoke Benchmark specification',
              active: emptyFocusCard === 'benchmark',
              onClick: () =>
                setEmptyFocusCard((prev) => (prev === 'benchmark' ? null : 'benchmark')),
            },
          ]}
        />
        <EmptyState
          icon={<Icon.bolt className="size-8" />}
          title="No harness eval runs yet"
          headingLevel={2}
          description="Run vox harness eval --live locally, or wait for the nightly scheduled workflow, to see chat harness quality and model-selection trends here."
        />
        {emptyDeck}
      </div>
    );
  }

  const totalPass = runs.reduce((acc, r) => acc + r.pass_count, 0);
  const totalFail = runs.reduce((acc, r) => acc + r.fail_count, 0);
  const totalAssertions = totalPass + totalFail;
  const observedCategories = Array.from(
    new Set(runs.flatMap((r) => r.category_breakdown.map((c) => c.category))),
  );
  const spokeDenom = Math.max(5, observedCategories.length);
  const q = searchQuery.trim().toLowerCase();
  const displayedRuns = runs.filter((r) => {
    if (runFilter === 'passing' && r.fail_count > 0) return false;
    if (runFilter === 'failing' && r.fail_count === 0) return false;
    if (categoryFilter && !r.category_breakdown.some((c) => c.category === categoryFilter)) {
      return false;
    }
    if (
      q &&
      !r.run_id.toLowerCase().includes(q) &&
      !r.git_sha.toLowerCase().includes(q) &&
      !r.triggered_by.toLowerCase().includes(q) &&
      !r.category_breakdown.some((c) => c.category.toLowerCase().includes(q))
    ) {
      return false;
    }
    return true;
  });

  const activeRun = displayedRuns.find((r) => r.run_id === selectedRunId) ?? displayedRuns[0] ?? null;
  const correlatedRegressions = activeRun
    ? (regressions ?? []).filter(
        (reg) => reg.current_run_id === activeRun.run_id || reg.previous_run_id === activeRun.run_id,
      )
    : [];

  const masterPane = (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <input
          type="search"
          aria-label="Filter evaluation runs"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Filter by run ID, git SHA, trigger, or spoke…"
          className="min-w-[200px] flex-1 rounded-md border border-border-subtle bg-bg-surface px-2.5 py-1.5 font-mono text-[11px] text-text-primary placeholder:text-text-muted focus:border-brass focus:outline-hidden"
        />
        <div className="flex flex-wrap items-center gap-1 font-mono text-[10px]">
          <button
            type="button"
            onClick={() => setCategoryFilter(null)}
            className={`rounded border px-2 py-1 transition-colors ${
              categoryFilter === null
                ? 'border-brass/50 bg-brass/15 text-brass'
                : 'border-border-subtle bg-bg-surface text-text-muted hover:text-text-primary'
            }`}
          >
            All Spokes
          </button>
          {observedCategories.map((cat) => (
            <button
              key={cat}
              type="button"
              onClick={() => setCategoryFilter((prev) => (prev === cat ? null : cat))}
              className={`rounded border px-2 py-1 transition-colors ${
                categoryFilter === cat
                  ? 'border-brass/50 bg-brass/15 text-brass'
                  : 'border-border-subtle bg-bg-surface text-text-muted hover:text-text-primary'
              }`}
            >
              {cat}
            </button>
          ))}
        </div>
      </div>

      <BentoScrollRegion
        ariaLabel="Harness evaluation run roster"
        className="flex-1 rounded-lg border border-border-subtle bg-bg-surface/60"
      >
        <table className="w-full text-left text-[12px]">
          <caption className="sr-only">Recent chat harness eval runs</caption>
          <thead className="border-b border-border-subtle bg-overlay-subtle/50 text-text-muted">
            <tr>
              <th scope="col" className="p-2">Run</th>
              <th scope="col" className="p-2">Git SHA</th>
              <th scope="col" className="p-2">Triggered by</th>
              <th scope="col" className="p-2">Pass</th>
              <th scope="col" className="p-2">Fail</th>
              <th scope="col" className="p-2">Skip</th>
              <th scope="col" className="p-2">Cost</th>
              <th scope="col" className="p-2">By category</th>
            </tr>
          </thead>
          <tbody>
            {displayedRuns.map((r) => {
              const isSelected = activeRun?.run_id === r.run_id;
              return (
                <tr
                  key={r.run_id}
                  data-testid={`harness-run-row-${r.run_id}`}
                  aria-selected={isSelected}
                  onClick={() => setSelectedRunId(r.run_id)}
                  className={`cursor-pointer border-t border-border-subtle transition-colors ${
                    isSelected ? 'bg-brass/10' : 'hover:bg-overlay-subtle/60'
                  }`}
                >
                  <td className="p-2 font-mono text-text-secondary">{r.run_id}</td>
                  <td className="p-2 font-mono text-text-muted">{r.git_sha}</td>
                  <td className="p-2 text-text-muted">{r.triggered_by}</td>
                  <td className="p-2 text-emerald-300">{r.pass_count}</td>
                  <td className="p-2 text-red-300">{r.fail_count}</td>
                  <td className="p-2 text-text-muted">{r.skip_count}</td>
                  <td className="p-2">${r.total_cost_usd.toFixed(4)}</td>
                  <td className="p-2">
                    <div className="flex flex-wrap gap-1">
                      {r.category_breakdown.map((c) => (
                        <span
                          key={c.category}
                          className={`rounded px-1 py-0.5 font-mono text-[11px] ${
                            c.fail_count > 0
                              ? 'bg-red-950 text-red-300'
                              : 'bg-emerald-950 text-emerald-300'
                          }`}
                          title={`${c.category}: ${c.pass_count} pass, ${c.fail_count} fail`}
                        >
                          {c.category} {c.pass_count}/{c.pass_count + c.fail_count}
                        </span>
                      ))}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </BentoScrollRegion>
    </div>
  );

  const detailPane = (
    <BentoScrollRegion
      ariaLabel="Run assertion inspector details"
      data-testid="harness-run-inspector"
      className="flex-1 pr-1"
    >
      {activeRun ? (
        <div className="flex flex-col gap-3">
          <div className="rounded-lg border border-border-subtle bg-bg-surface p-3">
            <div className="flex items-center justify-between gap-2">
              <span className="font-display text-xs tracking-wider text-brass uppercase">
                {`Selected Run: ${activeRun.run_id}`}
              </span>
              <span
                className={`rounded border px-1.5 py-0.5 font-mono text-[10px] ${
                  activeRun.fail_count === 0
                    ? 'border-emerald-400/30 bg-emerald-400/10 text-emerald-300'
                    : 'border-red-400/30 bg-red-400/10 text-red-300'
                }`}
              >
                {activeRun.fail_count === 0 ? 'VERIFIED' : `${activeRun.fail_count} FAILED`}
              </span>
            </div>
            <div className="mt-2 grid grid-cols-2 gap-2 font-mono text-[11px] text-text-secondary">
              <div>{`Commit: ${activeRun.git_sha}`}</div>
              <div>{`Trigger: ${activeRun.triggered_by}`}</div>
              <div>{`Skipped: ${activeRun.skip_count}`}</div>
              <div>{`Burn: $${activeRun.total_cost_usd.toFixed(4)}`}</div>
            </div>
          </div>

          <div className="rounded-lg border border-border-subtle bg-bg-surface p-3">
            <div className="mb-2 flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">
                Category Breakdown &amp; Assertions
              </span>
              <span className="font-mono text-[10px] text-text-muted">
                {`${activeRun.category_breakdown.length} spokes`}
              </span>
            </div>
            <div className="space-y-2.5">
              {activeRun.category_breakdown.map((cat) => {
                const total = cat.pass_count + cat.fail_count;
                const pct = total > 0 ? Math.round((cat.pass_count / total) * 100) : 100;
                return (
                  <div key={cat.category} className="space-y-1">
                    <div className="flex items-center justify-between font-mono text-[11px]">
                      <span className="text-text-primary">{`Spoke · ${cat.category}`}</span>
                      <span className={cat.fail_count > 0 ? 'text-red-300' : 'text-emerald-300'}>
                        {`${cat.pass_count} pass · ${cat.fail_count} fail (${pct}%)`}
                      </span>
                    </div>
                    <div className="h-1.5 w-full overflow-hidden rounded-full bg-overlay-subtle">
                      <div
                        className={`h-full transition-[width] duration-200 ${
                          cat.fail_count > 0 ? 'bg-red-400' : 'bg-emerald-400'
                        }`}
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="rounded-lg border border-border-subtle bg-bg-surface p-3 font-mono text-[11px]">
            <div className="font-display text-[10px] tracking-widest text-cyan uppercase">
              Diagnostic Posture
            </div>
            <p className="mt-1 text-text-secondary">
              {correlatedRegressions.length > 0
                ? `${correlatedRegressions.length} commit-range regression flag(s) correlated with this run.`
                : 'Zero commit-range regressions correlated with this evaluation run.'}
            </p>
          </div>
        </div>
      ) : (
        <div className="rounded-lg border border-dashed border-border-subtle p-4 text-center font-mono text-xs text-text-muted">
          No evaluation runs match the active filter criteria.
        </div>
      )}
    </BentoScrollRegion>
  );

  return (
    <section
      className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4 p-4"
      aria-labelledby="harness-health-title"
    >
      <RomanSurfaceHeader
        domainTag="DIAGNOSTICS · HARNESS HEALTH"
        title="Harness Evaluation & Regression Oracles"
        subtitle="5-SPOKE BENCHMARK · REGRESSION DETECTION · PASS/FAIL TELEMETRY"
        waxSealBadge={totalFail === 0 ? 'ALL PASSING' : 'REGRESSIONS'}
        icon={<Icon.bolt className="size-4 text-[#F3E5AB]" />}
        gauges={[
          {
            label: 'Runs',
            valueText: `${runs.length}`,
            ratio: totalAssertions > 0 ? totalPass / totalAssertions : 0,
            tone: 'gold',
            purpose: 'Empirical assertion pass rate across loaded harness runs',
            actionHint:
              runFilter === 'passing'
                ? 'Click to show all evaluation runs'
                : 'Click to filter table to 100% passing runs',
            active: runFilter === 'passing',
            onClick: () => setRunFilter((f) => (f === 'passing' ? 'all' : 'passing')),
          },
          {
            label: 'Failures',
            valueText: `${totalFail}`,
            ratio: totalAssertions > 0 ? totalFail / totalAssertions : 0,
            tone: totalFail === 0 ? 'emerald' : 'crimson',
            purpose: 'Failed benchmark assertions and flagged commit regressions',
            actionHint:
              runFilter === 'failing'
                ? 'Click to show all evaluation runs'
                : 'Click to filter table to runs with failures',
            active: runFilter === 'failing',
            onClick: () => setRunFilter((f) => (f === 'failing' ? 'all' : 'failing')),
          },
          {
            label: 'Spokes',
            valueText: `${observedCategories.length}/${spokeDenom}`,
            ratio: spokeDenom > 0 ? observedCategories.length / spokeDenom : 0,
            tone: 'gold',
            purpose: 'Benchmark spoke categories observed in evaluation history',
            actionHint: categoryFilter
              ? `Filtering spoke: ${categoryFilter} (click to cycle)`
              : 'Click to cycle spoke category filter',
            active: categoryFilter !== null,
            onClick: () => {
              if (observedCategories.length === 0) return;
              const idx = categoryFilter ? observedCategories.indexOf(categoryFilter) : -1;
              setCategoryFilter(
                idx + 1 < observedCategories.length ? observedCategories[idx + 1]! : null,
              );
            },
          },
        ]}
      />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 id="harness-health-title" className="font-display text-lg text-text-primary tracking-wider uppercase">Harness Health</h2>
          <p className="font-mono text-[11px] text-text-muted">
            Continuous chat harness evaluation, pass/fail telemetry &amp; commit-range regression detection
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2 font-mono text-[11px]">
          <span className="rounded-lg border border-emerald-400/30 bg-emerald-400/10 px-2.5 py-1 text-emerald-300">
            Status: {totalFail === 0 ? 'All Passing' : 'Regressions Flagged'}
          </span>
          <span className="rounded-lg border border-border-subtle bg-bg-surface px-2.5 py-1 text-text-secondary">Runs: {runs.length}</span>
          <span className="rounded-lg border border-brass/30 bg-brass/10 px-2.5 py-1 text-brass">Eval Gate: Active</span>
        </div>
      </div>
      {regressions && regressions.length > 0 && (
        <div className="space-y-2" role="alert">
          {regressions.map((r) => (
            <div key={`${r.kind}-${r.previous_run_id}-${r.current_run_id}`} className="rounded-lg border border-red-400/30 bg-red-400/6 p-3 text-[12px]">
              <p className="font-medium text-red-300">Regression detected ({r.kind}): {r.detail}</p>
              <p className="mt-1 font-mono text-[10px] text-text-muted">{r.previous_git_sha}..{r.current_git_sha}</p>
              {r.flipped_task_ids.length > 0 && (
                <p className="mt-1 text-[10px] text-red-300/80">Flipped tasks: {r.flipped_task_ids.join(', ')}</p>
              )}
              {r.changed_files.length > 0 && (
                <ul className="mt-1 space-y-0.5 font-mono text-[10px] text-text-muted">
                  {r.changed_files.map((f) => <li key={f}>{f}</li>)}
                </ul>
              )}
            </div>
          ))}
        </div>
      )}

      <MasterDetailWorkbench
        storageKey="vox.layout.harness-health.v4"
        detailTitle="Run Assertion Inspector"
        detailDescription="Selected evaluation run spoke assertions, pass/fail breakdown, and regression correlation."
        hasActiveDetail={selectedRunId !== null}
        defaultMasterPct={58}
        master={masterPane}
        detail={detailPane}
      />

      {emptyDeck}
    </section>
  );
}
