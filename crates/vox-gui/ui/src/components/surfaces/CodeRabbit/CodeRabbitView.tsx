import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { codeRabbitPlan, codeRabbitReport, codeRabbitRunAsync, codeRabbitTokenPresent } from '../../../transport';
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

  useEffect(() => {
    codeRabbitTokenPresent().then((ok) => setTokenOk(Boolean(ok))).catch(() => setTokenOk(false));
    codeRabbitReport<Report>().then(setReport).catch(() => {});
  }, []);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    let cancelled = false;
    listen<{ status: string; error?: string }>('coderabbit://progress', (e) => {
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

  const coderabbitGauges = [
    {
      label: 'Slices',
      valueText: `${rows.length} PRs`,
      ratio: Math.min(1, rows.length / 10),
      tone: 'gold' as const,
      purpose: 'Planned CodeRabbit review slice pull requests',
    },
    {
      label: 'Files',
      valueText: `${totalFiles} Files`,
      ratio: Math.min(1, totalFiles / 150),
      tone: 'gold' as const,
      purpose: 'Total source files selected for review sweep',
    },
    {
      label: 'Token',
      valueText: tokenOk == null ? '…' : tokenOk ? 'Present' : 'Absent',
      ratio: tokenOk ? 1.0 : 0.0,
      tone: tokenOk ? ('emerald' as const) : ('crimson' as const),
      purpose: 'CodeRabbit authentication token status',
    },
  ];

  return (
    <div className="max-w-4xl space-y-4 rounded-xl border border-[#D4AF37]/40 bg-bg-surface p-5">
      <RomanSurfaceHeader
        domainTag="WORKSPACE · CODERABBIT"
        title="CodeRabbit Code Review Sweep"
        subtitle="Batch repository slices into 150-file review PRs paced to respect CodeRabbit Pro hourly quotas."
        waxSealBadge="SWEEP READY"
        icon={<Icon.shield className="size-5 text-text-primary" />}
        gauges={coderabbitGauges}
      />
      <div className="mb-4 flex items-center gap-2.5">
        <h2 className="m-0 font-display text-lg font-semibold tracking-wider text-text-primary uppercase">CodeRabbit review</h2>
        <span className="rounded-full border border-brass/30 bg-brass/10 px-2.5 py-0.5 font-mono text-[11px] text-brass">
          Pro · 150 files · 5/hr
        </span>
        <span className="ml-auto rounded border border-border-subtle bg-bg-base/70 px-2 py-0.5 font-mono text-xs text-text-secondary">
          token: {tokenOk == null ? '…' : tokenOk ? 'present ✓' : 'absent'}
        </span>
      </div>

      <div className="mb-4 flex flex-wrap items-end gap-3.5">
        <label className="text-xs">
          <div className="mb-1 text-text-secondary">Scope</div>
          <label className="inline-flex h-8 items-center gap-2 rounded-md border border-border-subtle bg-black/30 px-2.5 text-xs text-text-primary">
            <input type="checkbox" checked={fullRepo} onChange={(e) => setFullRepo(e.target.checked)} />
            Full repo
          </label>
        </label>
        <label className="text-xs">
          <div className="mb-1 text-text-secondary">Modified since</div>
          <input
            type="date"
            value={since}
            disabled={fullRepo}
            onChange={(e) => setSince(e.target.value)}
            className="h-8 rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary [color-scheme:dark] disabled:opacity-50"
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
            className="h-8 w-[90px] rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary"
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
            className="h-8 w-[90px] rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary placeholder:text-text-muted"
          />
        </label>
        <label className="text-xs">
          <div className="mb-1 text-text-secondary">Rank weights (r,c,g)</div>
          <input
            type="text"
            value={weights}
            onChange={(e) => setWeights(e.target.value)}
            className="h-8 w-[110px] rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary"
          />
        </label>
        <button
          type="button"
          onClick={plan}
          disabled={busy || (!fullRepo && !since)}
          className="h-8 rounded-md border border-brass/30 bg-brass/15 px-3 text-xs font-medium text-brass transition hover:bg-brass/25 disabled:opacity-50"
        >
          {busy ? 'Planning…' : 'Plan sweep'}
        </button>
        <button
          type="button"
          onClick={run}
          disabled={running || !rows.length}
          className="h-8 rounded-md border border-border-subtle bg-overlay-subtle px-3 text-xs font-medium text-text-secondary transition hover:text-text-primary disabled:opacity-50"
        >
          {running ? 'Running…' : 'Run'}
        </button>
      </div>

      <div className="mb-2 text-xs text-text-secondary" style={{ color: 'var(--color-text-secondary)' }}>
        {rows.length
          ? `Planned ${rows.length} PRs · ${totalFiles} files · est ~${Math.ceil(rows.length / 5)}h at 5/hr`
          : 'No plan yet — pick a date (or Full repo) and click “Plan sweep”.'}
      </div>

      {rows.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-border-subtle">
          {rows.map((r, i) => {
            const pill = PILL[r.status] ?? PILL.pending;
            return (
              <div
                key={r.name}
                className={`flex items-center gap-3 px-3.5 py-2.5 ${i ? 'border-t border-border-subtle' : ''}`}
              >
                <span className="flex-1 font-mono text-xs text-text-primary">{r.name}</span>
                <span className="w-[70px] text-right font-mono text-xs text-text-muted">{r.files} files</span>
                {r.pr ? <span className="font-mono text-xs text-text-muted">#{r.pr}</span> : <span className="w-7" />}
                <span
                  style={{ background: pill.bg, color: pill.fg }}
                  className="w-[84px] rounded-full px-2.5 py-0.5 text-center font-mono text-[11px]"
                >
                  {r.status}
                </span>
              </div>
            );
          })}
        </div>
      ) : (
        <div
          data-testid="coderabbit-sweep-deck"
          className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3"
        >
          <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-brass uppercase">Slice Ranking</span>
              <span className="font-mono text-[9px] text-brass">r,c,g weights</span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Prioritizes files by recency (<code className="text-[#D4AF37]">r</code>), cyclomatic churn (<code className="text-[#D4AF37]">c</code>), and crate-graph centrality (<code className="text-[#D4AF37]">g</code>).
            </p>
          </div>
          <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-[#D4AF37] uppercase">Rate-Limit Guard</span>
              <span className="font-mono text-[9px] text-emerald-300">5 PRs / hr</span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Batches repository slices into 150-file review PRs paced to respect CodeRabbit Pro hourly quotas.
            </p>
          </div>
          <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">Progress Stream</span>
              <span className="font-mono text-[9px] text-text-muted">coderabbit://</span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed text-text-secondary">
              Streams real-time slice completion events and persists findings into the local review report database.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
