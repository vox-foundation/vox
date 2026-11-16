import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { codeRabbitPlan, codeRabbitReport, codeRabbitRunAsync, codeRabbitTokenPresent, listenEvent, type UnlistenFn } from '../../../transport';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
import { RomanSurfaceHeader } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';
import type { Toast } from '../../../types/tauri';

interface CodeRabbitViewProps {
  pushToast: (t: Toast) => void;
  gamifyEnabled?: boolean;
}

interface Chunk {
  order: number;
  name: string;
  files: string[];
}
interface Manifest {
  baseline_branch?: string;
  total_files?: number;
  chunks?: Chunk[];
}
interface RunChunk {
  name: string;
  pr_number?: number | null;
  status?: string;
}
interface Report {
  run_state?: { chunks?: RunChunk[] } | null;
  db_status?: any;
}

const PILL: Record<string, { bg: string; fg: string }> = {
  completed: { bg: 'var(--bg-success, rgba(16, 185, 129, 0.14))', fg: 'var(--text-success, #34d399)' },
  failed: { bg: 'var(--bg-danger, rgba(244, 63, 94, 0.14))', fg: 'var(--text-danger, #fb7185)' },
  pending: { bg: 'var(--surface-1, rgba(255, 255, 255, 0.06))', fg: 'var(--text-pending, var(--color-text-secondary))' },
};

/** Merge the planned manifest with run-state statuses into display rows. */
export function toSliceRows(manifest: Manifest | null, report: Report | null) {
  const statusByName = new Map<string, RunChunk>();
  for (const c of report?.run_state?.chunks ?? []) statusByName.set(c.name, c);
  return (manifest?.chunks ?? []).map((c) => {
    const rs = statusByName.get(c.name);
    return {
      name: c.name,
      files: c.files?.length ?? 0,
      status: rs?.status ?? 'planned',
      pr: rs?.pr_number ?? null,
    };
  });
}

export function CodeRabbitView({ pushToast }: CodeRabbitViewProps): React.ReactElement {
  const [since, setSince] = useState('2026-04-01');
  const [fullRepo, setFullRepo] = useState(false);
  const [cap, setCap] = useState(150);
  const [weights, setWeights] = useState('1,1,1');
  // Empty = review everything selected (no truncation). A number keeps only the
  // top-N most important files — never default this on for a full-repo sweep.
  const [top, setTop] = useState('');
  const [manifest, setManifest] = useState<Manifest | null>(null);
  const [report, setReport] = useState<Report | null>(null);
  const [tokenOk, setTokenOk] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [running, setRunning] = useState(false);
  const [sliceFilter, setSliceFilter] = useState<'all' | 'pending' | 'failed' | 'completed'>('all');
  const [sortByFilesDesc, setSortByFilesDesc] = useState(false);

  useEffect(() => {
    codeRabbitTokenPresent().then((ok) => setTokenOk(Boolean(ok))).catch(() => setTokenOk(false));
    codeRabbitReport<Report>().then(setReport).catch(() => {});
  }, []);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    let cancelled = false;
    listenEvent<{ status: string; error?: string }>('coderabbit://progress', (e) => {
      setRunning(false);
      pushToast(
        e.payload.status === 'error'
          ? { tone: 'warn', title: 'CodeRabbit run failed', body: e.payload.error, cause: 'backend-error' }
          : { tone: 'ok', title: 'CodeRabbit run finished', cause: 'backend-ok' },
      );
      codeRabbitReport<Report>().then(setReport).catch(() => {});
    }).then((u) => {
      // If we unmounted before listen() resolved, unlisten immediately (no leak).
      if (cancelled) u();
      else un = u;
    }).catch(() => { /* event bridge unavailable — no progress toasts */ });
    return () => {
      cancelled = true;
      un?.();
    };
  }, [pushToast]);

  // Same top-N for plan and run so the preview matches what actually executes.
  const topN = top.trim() ? Math.max(1, Math.trunc(Number(top))) : null;

  const plan = useCallback(async () => {
    setBusy(true);
    try {
      const m = await codeRabbitPlan<Manifest>({ since, cap, rankWeights: weights, top: topN, fullRepo });
      setManifest(m);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Plan failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  }, [since, cap, weights, topN, fullRepo, pushToast]);

  const run = useCallback(async () => {
    setRunning(true);
    try {
      await codeRabbitRunAsync({ since, cap, rankWeights: weights, top: topN, fullRepo });
      pushToast({ tone: 'info', title: 'CodeRabbit sweep started', cause: 'backend-ok' });
    } catch (err) {
      setRunning(false);
      pushToast({ tone: 'warn', title: 'Run failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [since, cap, weights, topN, fullRepo, pushToast]);

  const rows = toSliceRows(manifest, report);
  const totalFiles = rows.reduce((a, r) => a + r.files, 0);
  const completedSlices = rows.filter((r) => r.status === 'completed').length;
  const sliceCapacity = rows.length * Math.max(1, cap);
  const fileEnvelope = Math.max(totalFiles, manifest?.total_files ?? 0, sliceCapacity);
  const visibleRows = (() => {
    const filtered = rows.filter((r) => {
      if (sliceFilter === 'all') return true;
      if (sliceFilter === 'pending') return r.status === 'pending' || r.status === 'planned';
      return r.status === sliceFilter;
    });
    return sortByFilesDesc ? [...filtered].sort((a, b) => b.files - a.files) : filtered;
  })();

  const coderabbitGauges = [
    {
      label: 'Slices',
      valueText: `${rows.length} PRs`,
      ratio:
        rows.length === 0
          ? 0
          : completedSlices > 0
            ? completedSlices / rows.length
            : Math.min(1, totalFiles / sliceCapacity),
      tone: 'gold' as const,
      purpose: 'Planned CodeRabbit review slice pull requests',
      actionHint: `Click to cycle slice status filter (currently: ${sliceFilter})`,
      active: sliceFilter !== 'all',
      onClick: () =>
        setSliceFilter((f) =>
          f === 'all' ? 'pending' : f === 'pending' ? 'failed' : f === 'failed' ? 'completed' : 'all',
        ),
    },
    {
      label: 'Files',
      valueText: `${totalFiles} Files`,
      ratio: totalFiles === 0 || fileEnvelope === 0 ? 0 : Math.min(1, totalFiles / fileEnvelope),
      tone: 'gold' as const,
      purpose: 'Total source files selected for review sweep',
      actionHint: sortByFilesDesc
        ? 'Click to restore planned manifest slice order'
        : 'Click to sort slices by largest file count',
      active: sortByFilesDesc,
      onClick: () => setSortByFilesDesc((v) => !v),
    },
    {
      label: 'Token',
      valueText: tokenOk == null ? '…' : tokenOk ? 'Present' : 'Absent',
      ratio: tokenOk === true ? 1 : 0,
      tone: tokenOk ? ('emerald' as const) : ('crimson' as const),
      purpose: 'CodeRabbit authentication token status',
      actionHint: 'Click to re-check CodeRabbit authentication token',
      active: tokenOk === true,
      onClick: () => {
        codeRabbitTokenPresent()
          .then((ok) => setTokenOk(Boolean(ok)))
          .catch(() => setTokenOk(false));
      },
    },
  ];

  const failedSlices = rows.filter((r) => r.status === 'failed').length;

  return (
    <div className="@container/surface flex w-full min-h-0 min-w-0 flex-col gap-3.5 rounded-xl border border-[#D4AF37]/40 bg-bg-surface p-3.5 @2xl/surface:p-5">
      <RomanSurfaceHeader
        domainTag="WORKSPACE · CODERABBIT"
        title="CodeRabbit Code Review Sweep"
        subtitle="Batch repository slices into 150-file review PRs paced to respect CodeRabbit Pro hourly quotas."
        waxSealBadge="SWEEP READY"
        icon={<Icon.shield className="size-5 text-text-primary" />}
        gauges={coderabbitGauges}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <span className="rounded-full border border-brass/35 bg-brass/12 px-2.5 py-0.5 font-mono text-[10px] text-brass">
              Pro · 150 files · 5/hr
            </span>
            <span className="inline-flex items-center gap-1 rounded border border-border-subtle bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-text-secondary">
              <span>token:</span>
              {tokenOk == null ? (
                <span>…</span>
              ) : tokenOk ? (
                <span className="inline-flex items-center gap-1">
                  <span>present</span>
                  <Icon.check className="size-3 shrink-0" aria-hidden="true" />
                </span>
              ) : (
                <span>absent</span>
              )}
            </span>
          </div>
        }
      />

      <BentoGrid preset="hero">
        <BentoCard span="hero-main" className="gap-3">
          <div className="flex flex-wrap items-end gap-3 rounded-lg border border-border-subtle bg-bg-elevated/60 p-3">
            <label className="text-xs">
              <div className="mb-1 text-text-secondary">Scope</div>
              <label className="inline-flex h-8 items-center gap-2 rounded-md border border-border-subtle bg-bg-elevated px-2.5 text-xs text-text-primary">
                <input type="checkbox" checked={fullRepo} onChange={(e) => setFullRepo(e.target.checked)} />
                Full repo
              </label>
            </label>
            <label className="min-w-0 text-xs">
              <div className="mb-1 text-text-secondary">Modified since</div>
              <input
                type="date"
                value={since}
                disabled={fullRepo}
                onChange={(e) => setSince(e.target.value)}
                className="h-8 max-w-full rounded-md border border-border-subtle bg-bg-elevated px-2.5 font-mono text-xs text-text-primary disabled:opacity-50"
              />
            </label>
            <label className="text-xs">
              <div className="mb-1 text-text-secondary">Max files / PR</div>
              <input
                type="number"
                min={1}
                max={150}
                value={cap}
                onChange={(e) => { const n = Number(e.target.value); setCap(Number.isFinite(n) ? Math.max(1, Math.min(150, Math.trunc(n))) : 1); }}
                className="h-8 w-[90px] rounded-md border border-border-subtle bg-bg-elevated px-2.5 font-mono text-xs text-text-primary"
              />
            </label>
            <label className="text-xs">
              <div className="mb-1 text-text-secondary">Top N (blank = all)</div>
              <input
                type="number"
                min={1}
                value={top}
                placeholder="all"
                onChange={(e) => setTop(e.target.value)}
                className="h-8 w-[90px] rounded-md border border-border-subtle bg-bg-elevated px-2.5 font-mono text-xs text-text-primary placeholder:text-text-muted"
              />
            </label>
            <label className="text-xs">
              <div className="mb-1 text-text-secondary">Rank weights (r,c,g)</div>
              <input
                type="text"
                value={weights}
                onChange={(e) => setWeights(e.target.value)}
                className="h-8 w-[110px] rounded-md border border-border-subtle bg-bg-elevated px-2.5 font-mono text-xs text-text-primary"
              />
            </label>
            <button
              type="button"
              onClick={plan}
              disabled={busy || (!fullRepo && !since)}
              className="h-8 rounded-md border border-brass/40 bg-brass/15 px-3 text-xs font-medium text-brass transition hover:bg-brass/25 disabled:opacity-50"
            >
              {busy ? 'Planning…' : 'Plan sweep'}
            </button>
            <button
              type="button"
              onClick={run}
              disabled={running || !rows.length}
              className="h-8 rounded-md border border-border-subtle bg-bg-elevated px-3 text-xs font-medium text-text-secondary transition hover:text-text-primary disabled:opacity-50"
            >
              {running ? 'Running…' : 'Run'}
            </button>
          </div>

          {/* Full-width Quota & Slice Pacing Telemetry Curve */}
          <div className="rounded-lg border border-[#C59B27]/35 bg-bg-elevated/70 px-2.5 py-2.5">
            <div className="mb-1.5 flex flex-wrap items-center justify-between gap-1 text-[11px]">
              <span className="font-display tracking-widest uppercase text-brass">
                Hourly PR Pacing &amp; Slice Capacity Envelope
              </span>
              <span className="font-mono text-text-secondary" style={{ color: 'var(--color-text-secondary)' }}>
                {rows.length
                  ? `Planned ${rows.length} PRs · ${totalFiles} files · est ~${Math.ceil(rows.length / 5)}h at 5/hr`
                  : 'No plan yet — pick a date (or Full repo) and click “Plan sweep”.'}
              </span>
            </div>
            <svg
              data-cv-chart="true"
              viewBox="0 0 600 44"
              preserveAspectRatio="none"
              className="block h-10 w-full overflow-visible"
              aria-hidden="true"
            >
              <defs>
                <linearGradient id="coderabbit-pacing-fill" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#D4AF37" stopOpacity="0.28" />
                  <stop offset="100%" stopColor="#D4AF37" stopOpacity="0.02" />
                </linearGradient>
              </defs>
              <line x1="0" y1="12" x2="600" y2="12" stroke="#C59B27" strokeOpacity="0.25" strokeDasharray="4 4" vectorEffect="non-scaling-stroke" />
              <line x1="0" y1="36" x2="600" y2="36" stroke="#C59B27" strokeOpacity="0.18" vectorEffect="non-scaling-stroke" />
              <path
                d="M 4,34 C 95,28 160,18 245,20 C 330,22 415,13 500,15 C 545,16 575,12 596,10 L 596,42 L 4,42 Z"
                fill="url(#coderabbit-pacing-fill)"
              />
              <path
                d="M 4,34 C 95,28 160,18 245,20 C 330,22 415,13 500,15 C 545,16 575,12 596,10"
                fill="none"
                stroke="#B8891E"
                strokeWidth="2"
                strokeLinecap="round"
                vectorEffect="non-scaling-stroke"
              />
            </svg>
          </div>

          {rows.length > 0 ? (
            <BentoScrollRegion
              ariaLabel="CodeRabbit review slice list"
              className="max-h-[360px] rounded-lg border border-border-subtle"
            >
              {visibleRows.map((r, i) => {
                const pill = PILL[r.status] ?? PILL.pending;
                return (
                  <div
                    key={r.name}
                    className={`flex flex-wrap items-center gap-2 px-3 py-2.5 @sm/surface:flex-nowrap @sm/surface:gap-3 ${i ? 'border-t border-border-subtle' : ''}`}
                  >
                    <span className="min-w-0 flex-1 truncate font-mono text-xs text-text-primary">{r.name}</span>
                    <span className="font-mono text-xs text-text-muted">{r.files} files</span>
                    {r.pr ? <span className="font-mono text-xs text-text-muted">#{r.pr}</span> : null}
                    <span
                      style={{ background: pill.bg, color: pill.fg }}
                      className="rounded-full px-2.5 py-0.5 text-center font-mono text-[11px]"
                    >
                      {r.status}
                    </span>
                  </div>
                );
              })}
            </BentoScrollRegion>
          ) : (
            <div className="rounded-lg border border-dashed border-border-subtle bg-bg-base/60 p-4 text-center font-mono text-xs text-text-secondary">
              No slices planned yet — configure scope above and run “Plan sweep” to preview batched review PRs.
            </div>
          )}
        </BentoCard>

        <BentoCard span="hero-rail" className="gap-2.5">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-display text-[11px] tracking-widest uppercase text-brass">
              Diff &amp; Sweep Summary
            </span>
            <span className="font-mono text-[10px] text-text-muted">
              base: {manifest?.baseline_branch ?? 'HEAD'}
            </span>
          </div>
          <div className="grid grid-cols-3 gap-1.5 text-center font-mono text-[10px]">
            <div className="rounded border border-border-subtle bg-bg-elevated px-2 py-1.5">
              <div className="text-text-muted">Done</div>
              <div className="font-bold text-text-primary">{completedSlices}</div>
            </div>
            <div className="rounded border border-border-subtle bg-bg-elevated px-2 py-1.5">
              <div className="text-text-muted">Queued</div>
              <div className="font-bold text-text-primary">{Math.max(0, rows.length - completedSlices - failedSlices)}</div>
            </div>
            <div className="rounded border border-border-subtle bg-bg-elevated px-2 py-1.5">
              <div className="text-text-muted">Failed</div>
              <div className="font-bold text-text-primary">{failedSlices}</div>
            </div>
          </div>
          <BentoScrollRegion
            ariaLabel="CodeRabbit sweep findings summary"
            className="max-h-[380px]"
          >
            <div data-testid="coderabbit-sweep-deck" className="grid grid-cols-1 gap-2.5">
              <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
                <div className="flex flex-wrap items-center justify-between gap-1">
                  <span className="font-display text-[10px] tracking-widest text-brass uppercase">Slice Ranking</span>
                  <span className="font-mono text-[9px] text-brass">r,c,g weights</span>
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                  Prioritizes files by recency (<code className="text-[#D4AF37]">r</code>), cyclomatic churn (<code className="text-[#D4AF37]">c</code>), and crate-graph centrality (<code className="text-[#D4AF37]">g</code>).
                </p>
              </div>
              <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
                <div className="flex flex-wrap items-center justify-between gap-1">
                  <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Rate-Limit Guard</span>
                  <span className="font-mono text-[9px] text-emerald-300">5 PRs / hr</span>
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                  Batches repository slices into 150-file review PRs paced to respect CodeRabbit Pro hourly quotas.
                </p>
              </div>
              <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
                <div className="flex flex-wrap items-center justify-between gap-1">
                  <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">Progress Stream</span>
                  <span className="font-mono text-[9px] text-text-muted">coderabbit://</span>
                </div>
                <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
                  Streams real-time slice completion events and persists findings into the local review report database.
                </p>
              </div>
            </div>
          </BentoScrollRegion>
        </BentoCard>
      </BentoGrid>
    </div>
  );
}
