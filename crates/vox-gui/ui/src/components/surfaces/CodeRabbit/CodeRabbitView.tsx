import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { codeRabbitPlan, codeRabbitReport, codeRabbitRunAsync, codeRabbitTokenPresent } from '../../../transport';
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

  return (
    <div className="max-w-4xl rounded-xl border border-border-subtle bg-bg-surface/60 p-5">
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: '1rem' }}>
        <h2 className="font-display text-lg font-semibold tracking-wider text-text-primary uppercase" style={{ margin: 0 }}>CodeRabbit review</h2>
        <span
          className="rounded-full border border-brass/30 bg-brass/10 px-2.5 py-0.5 font-mono text-[11px] text-brass"
          style={{ background: 'var(--bg-pro, rgba(201, 162, 74, 0.12))', color: 'var(--text-pro, rgb(var(--brass)))' }}
        >
          Pro · 150 files · 5/hr
        </span>
        <span style={{ marginLeft: 'auto', fontSize: 12, color: 'var(--color-text-muted)' }}>
          token: {tokenOk == null ? '…' : tokenOk ? 'present ✓' : 'absent'}
        </span>
      </div>

      <div style={{ display: 'flex', gap: 14, flexWrap: 'wrap', alignItems: 'end', marginBottom: '1rem' }}>
        <label style={{ fontSize: 13 }}>
          <div style={{ color: 'var(--color-text-secondary)', marginBottom: 4 }}>Scope</div>
          <label className="inline-flex h-8 items-center gap-2 rounded-md border border-border-subtle bg-black/30 px-2.5 text-xs text-text-primary">
            <input type="checkbox" checked={fullRepo} onChange={(e) => setFullRepo(e.target.checked)} />
            Full repo
          </label>
        </label>
        <label style={{ fontSize: 13 }}>
          <div style={{ color: 'var(--color-text-secondary)', marginBottom: 4 }}>Modified since</div>
          <input
            type="date"
            value={since}
            disabled={fullRepo}
            onChange={(e) => setSince(e.target.value)}
            className="h-8 rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary [color-scheme:dark] disabled:opacity-50"
          />
        </label>
        <label style={{ fontSize: 13 }}>
          <div style={{ color: 'var(--color-text-secondary)', marginBottom: 4 }}>Max files / PR</div>
          <input
            type="number"
            min={1}
            max={150}
            value={cap}
            onChange={(e) => { const n = Number(e.target.value); setCap(Number.isFinite(n) ? Math.max(1, Math.min(150, Math.trunc(n))) : 1); }}
            style={{ width: 90 }}
            className="h-8 rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary"
          />
        </label>
        <label style={{ fontSize: 13 }}>
          <div style={{ color: 'var(--color-text-secondary)', marginBottom: 4 }}>Top N (blank = all)</div>
          <input
            type="number"
            min={1}
            value={top}
            placeholder="all"
            onChange={(e) => setTop(e.target.value)}
            style={{ width: 90 }}
            className="h-8 rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary placeholder:text-text-muted"
          />
        </label>
        <label style={{ fontSize: 13 }}>
          <div style={{ color: 'var(--color-text-secondary)', marginBottom: 4 }}>Rank weights (r,c,g)</div>
          <input
            type="text"
            value={weights}
            onChange={(e) => setWeights(e.target.value)}
            style={{ width: 110 }}
            className="h-8 rounded-md border border-border-subtle bg-black/30 px-2.5 font-mono text-xs text-text-primary"
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

      <div style={{ fontSize: 13, color: 'var(--color-text-secondary)', marginBottom: 8 }}>
        {rows.length
          ? `Planned ${rows.length} PRs · ${totalFiles} files · est ~${Math.ceil(rows.length / 5)}h at 5/hr`
          : 'No plan yet — pick a date (or Full repo) and click “Plan sweep”.'}
      </div>

      {rows.length > 0 ? (
        <div style={{ border: '0.5px solid var(--color-border-subtle)', borderRadius: 8, overflow: 'hidden' }}>
          {rows.map((r, i) => {
            const pill = PILL[r.status] ?? PILL.pending;
            return (
              <div key={r.name} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '10px 14px', borderTop: i ? '0.5px solid var(--color-border-subtle)' : 'none' }}>
                <span style={{ fontFamily: 'var(--font-mono, monospace)', fontSize: 13, flex: 1 }}>{r.name}</span>
                <span style={{ fontSize: 12, color: 'var(--color-text-muted)', width: 70, textAlign: 'right' }}>{r.files} files</span>
                {r.pr ? <span style={{ fontSize: 12, color: 'var(--color-text-muted)' }}>#{r.pr}</span> : <span style={{ width: 28 }} />}
                <span style={{ background: pill.bg, color: pill.fg, fontSize: 11, padding: '3px 9px', borderRadius: 20, width: 84, textAlign: 'center' }}>{r.status}</span>
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
            <p className="mt-1.5 text-[11px] leading-relaxed" style={{ color: 'var(--color-text-secondary)' }}>
              Prioritizes files by recency (<code className="text-cyan">r</code>), cyclomatic churn (<code className="text-cyan">c</code>), and crate-graph centrality (<code className="text-cyan">g</code>).
            </p>
          </div>
          <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-cyan uppercase">Rate-Limit Guard</span>
              <span className="font-mono text-[9px] text-emerald-300">5 PRs / hr</span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed" style={{ color: 'var(--color-text-secondary)' }}>
              Batches repository slices into 150-file review PRs paced to respect CodeRabbit Pro hourly quotas.
            </p>
          </div>
          <div className="rounded-lg border border-border-subtle bg-bg-base/80 p-3">
            <div className="flex items-center justify-between">
              <span className="font-display text-[10px] tracking-widest text-text-primary uppercase">Progress Stream</span>
              <span className="font-mono text-[9px]" style={{ color: 'var(--color-text-muted)' }}>coderabbit://</span>
            </div>
            <p className="mt-1.5 text-[11px] leading-relaxed" style={{ color: 'var(--color-text-secondary)' }}>
              Streams real-time slice completion events and persists findings into the local review report database.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
