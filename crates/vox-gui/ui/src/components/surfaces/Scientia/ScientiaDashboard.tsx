import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useLabel } from '../../../hooks/useLanguage';
import { listenScientiaQueue } from '../../../transport';
import { fetchCostRollup, providerRows, quarterlyRows } from './costRollup';
import type { CostRollup } from './costRollup';
import { ArchiveStatusSummary } from './ArchiveStatusSummary';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { RomanSurfaceHeader, RomanWaxSealBadge } from '../../ui/RomanSurfaceHeader';
import { Icon } from '../../ui/Icons';

interface CandidateRow {
  candidate_id: string;
  candidate_class: string;
  confidence: number;
  state: string;
  created_at_ms: number;
  updated_at_ms: number;
}

interface QueueSnapshot {
  candidates: {
    total: number;
    by_class: Record<string, number>;
    top_5_by_confidence: CandidateRow[];
  };
  claims_pending: { verifiable: number; abstained: number; extraction_running: number };
  manifests_in_reply_window: string[];
  retraction_queue: string[];
  stalls: { candidate_id: string; class: string; stuck_for_ms: number }[];
}

function Kpi({ label, value, tone }: { label: string; value: number; tone?: string }) {
  return (
    <div
      style={{
        backgroundImage:
          'linear-gradient(180deg, rgba(250,246,238,0.96) 0%, rgba(239,230,213,0.95) 100%), url(/assets/romanesque/carrara-seamless-stone.jpg)',
        backgroundSize: 'cover',
        backgroundPosition: 'center',
      }}
      className="vox-marble-tablet rounded-lg border-2 border-[#C59B27] px-2.5 py-2 shadow-[0_3px_10px_rgba(28,20,12,0.16),inset_0_1px_0_rgba(255,255,255,0.9)]"
    >
      <div className={`font-display text-xl font-bold leading-none ${tone ?? 'text-[#1C140C]'}`}>{value}</div>
      <div className="mt-1 font-mono text-[9px] font-bold uppercase tracking-wider text-[#4A3718]">{label}</div>
    </div>
  );
}

/**
 * Phase H structured dashboard: a QueueSnapshot from the native
 * `scientia_dashboard_snapshot` command, served by the daemon
 * (`GET /api/v2/scientia/queue`) so it reflects whichever Vox server Axis is
 * connected to, local or remote.
 */
export function ScientiaDashboard({ pushToast }: SurfaceDecoratorProps) {
  const embedded = useIsEmbeddedSurface();
  const [snap, setSnap] = useState<QueueSnapshot | null>(null);
  const [cost, setCost] = useState<CostRollup | null>(null);
  const [loading, setLoading] = useState(false);
  // Guard against interval-triggered overlapping fetches while one is in flight.
  const fetchingRef = React.useRef(false);

  const refresh = useCallback(async () => {
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    setLoading(true);
    try {
      const nextSnap = await invoke<QueueSnapshot | null>('scientia_dashboard_snapshot');
      setSnap(nextSnap && typeof nextSnap === 'object' && 'candidates' in nextSnap ? nextSnap : null);
      // Cost rollup is an independent producer (`vox scientia cost`); a failure
      // here must not blank the queue snapshot above.
      try {
        setCost(await fetchCostRollup());
      } catch (costErr) {
        pushToast({ tone: 'warn', title: 'Scientia cost', body: sanitizeErrorForToast(costErr), cause: 'backend-error' });
        setCost(null);
      }
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Scientia dashboard', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      setSnap(null);
    } finally {
      setLoading(false);
      fetchingRef.current = false;
    }
  }, [pushToast]);

  // Initial fetch + 10 s auto-refresh; interval is cleared on unmount.
  useEffect(() => {
    refresh();
    // Embedded mini-render: one initial fetch only, no repeating poll.
    if (embedded) return;
    const id = setInterval(refresh, 10_000);
    return () => clearInterval(id);
  }, [refresh, embedded]);

  // F2: event-driven refresh — refetch immediately when the Rust DB watcher
  // pushes a "vox://scientia-queue" ping. The 10 s interval above stays as a
  // fallback (e.g. outside Tauri, where listen() rejects). Cleans up on unmount.
  useEffect(() => {
    // Embedded mini-render: no pushed subscription either — stays static.
    if (embedded) return;
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(() => {
      void refresh();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        /* not in Tauri or no event bridge — interval fallback covers it */
      });
    return () => unlisten?.();
  }, [refresh, embedded]);

  const ScientiaIcon = (Icon as any).scientia ?? Icon.beaker;

  return (
    <section className="space-y-4">
      <RomanSurfaceHeader
        domainTag="KNOWLEDGE · SCIENTIA"
        title="Scientia Epistemic Discovery & Verification Ledger"
        subtitle="Publication pipeline queue snapshot · Track machine-verified research claims, novelty evidence, and publication readiness."
        waxSealBadge="EPISTEMIC LEDGER"
        icon={<ScientiaIcon className="size-5 text-[#F3E5AB]" />}
        gauges={
          snap
            ? [
                {
                  label: 'CANDIDATES',
                  valueText: `${snap.candidates.total}`,
                  ratio: Math.min(1, snap.candidates.total / 10),
                  tone: 'gold',
                  purpose: 'Total publication candidates',
                },
                {
                  label: 'VERIFIABLE',
                  valueText: `${snap.claims_pending.verifiable}`,
                  ratio: Math.min(1, snap.claims_pending.verifiable / 10),
                  tone: 'emerald',
                  purpose: 'Claims pending verification',
                },
                {
                  label: 'STALLS',
                  valueText: `${snap.stalls.length}`,
                  ratio: Math.min(1, snap.stalls.length / 5),
                  tone: snap.stalls.length > 0 ? 'crimson' : 'emerald',
                  purpose: 'Pipeline candidate stalls',
                },
              ]
            : undefined
        }
        actions={
          <button
            type="button"
            className="rounded-lg border border-[#D4AF37]/40 bg-[#78350F]/20 px-3 py-1.5 font-display text-xs uppercase tracking-wider text-[#F3E5AB] hover:bg-[#78350F]/40 disabled:opacity-40 transition"
            disabled={loading}
            onClick={refresh}
          >
            {loading ? 'Refreshing…' : 'Refresh'}
          </button>
        }
      />

      {/* Snapshot region: refetched on a 10s interval + event ping, so announce
          updates politely to assistive tech. */}
      <div aria-live="polite" className="space-y-3">
        {loading && !snap && <div className="font-mono text-xs text-text-muted">Loading queue snapshot…</div>}
        {!loading && !snap && (
          <div className="rounded-xl border border-dashed border-[#D4AF37]/30 bg-bg-surface p-4 font-mono text-xs text-[#D6CEBF]">
            Queue snapshot unavailable — click Refresh to retry.
          </div>
        )}

        {snap && (
          <>
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4 lg:grid-cols-7">
              <Kpi label="Candidates" value={snap.candidates.total} tone="text-[#785312]" />
              <Kpi label="Verifiable claims" value={snap.claims_pending.verifiable} tone="text-[#3E4A16]" />
              <Kpi label="Abstained claims" value={snap.claims_pending.abstained} tone="text-[#3B2E1E]" />
              <Kpi label="Extraction pending" value={snap.claims_pending.extraction_running} tone="text-[#785312]" />
              <Kpi label="Reply window" value={snap.manifests_in_reply_window.length} tone="text-[#1C140C]" />
              <Kpi label="Retraction queue" value={snap.retraction_queue.length} tone="text-[#881B1B]" />
              <Kpi label="Stalls" value={snap.stalls.length} tone="text-[#881B1B]" />
            </div>

            {Object.keys(snap.candidates.by_class).length > 0 && (
              <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3">
                <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-brass">Candidates by class</div>
                <div className="flex flex-wrap gap-2">
                  {Object.entries(snap.candidates.by_class).map(([cls, n]) => (
                    <span key={cls} className="rounded border border-[#D4AF37]/25 bg-bg-elevated px-2.5 py-1 font-mono text-[11px] text-text-secondary">
                      {cls} <span className="ml-1 font-bold text-text-primary">{n}</span>
                    </span>
                  ))}
                </div>
              </div>
            )}

            {snap.candidates.top_5_by_confidence.length > 0 && (
              <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3">
                <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-brass">Top candidates</div>
                <div className="space-y-2">
                  {snap.candidates.top_5_by_confidence.map((c) => (
                    <div
                      key={c.candidate_id}
                      className="flex flex-wrap items-center gap-2.5 rounded-lg border border-[#D4AF37]/30 bg-bg-elevated px-3 py-2 font-mono text-[11px]"
                    >
                      <RomanWaxSealBadge
                        label={`${c.confidence.toFixed(2)} CONF`}
                        tone={c.confidence >= 0.8 ? 'emerald' : c.confidence >= 0.5 ? 'gold' : 'crimson'}
                      />
                      <span className="font-medium text-text-primary">{c.candidate_id}</span>
                      <span className="rounded border border-[#D4AF37]/25 bg-bg-surface px-1.5 py-0.5 text-[10px] text-text-secondary">
                        {c.candidate_class}
                      </span>
                      <span className="ml-auto rounded border border-[#D4AF37]/35 bg-[#78350F]/20 px-2 py-0.5 text-[10px] font-semibold text-brass">
                        {c.state}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {snap.stalls.length > 0 && (
              <div className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-2.5">
                <div className="mb-1.5 font-mono text-[10px] uppercase tracking-wider text-amber-700">Stalled candidates</div>
                <div className="space-y-1">
                  {snap.stalls.map((s) => (
                    <div key={s.candidate_id} className="flex items-center gap-2 rounded border border-amber-500/20 bg-bg-base/60 px-2.5 py-1 font-mono text-[11px]">
                      <span className="font-medium text-text-primary">{s.candidate_id}</span>
                      <span className="text-text-secondary">{s.class}</span>
                      <span className="ml-auto rounded border border-amber-400/30 bg-amber-400/10 px-1.5 py-0.5 text-[10px] text-amber-700">
                        {Math.round(s.stuck_for_ms / 86_400_000)}d stuck
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        <ArchiveStatusSummary
          onFetchError={(message) =>
            pushToast({ tone: 'warn', title: 'Archive rollup', body: message, cause: 'backend-error' })
          }
        />

        {cost && (
          <div className="rounded-xl border border-[#D4AF37]/35 bg-bg-surface p-3">
            <div className="mb-2 flex items-center justify-between">
              <div className="font-display text-[10px] uppercase tracking-wider text-brass">
                Cost this quarter
              </div>
              <div className="rounded border border-[#D4AF37]/40 bg-[#78350F]/20 px-2 py-0.5 font-mono text-[10px] text-text-primary">
                avg/finding{' '}
                <span className="font-bold">${cost.per_finding_average_usd.toFixed(2)}</span>
              </div>
            </div>

            {cost.this_quarter.total_usd === 0 && cost.by_provider.length === 0 ? (
              <div className="font-mono text-[11px] text-text-muted">
                No cost recorded this quarter.
              </div>
            ) : (
              <div className="grid gap-3 sm:grid-cols-2">
                <div className="space-y-1">
                  {quarterlyRows(cost).map((r) => (
                    <div
                      key={r.label}
                      className={`flex items-center justify-between rounded border px-2 py-1 font-mono text-[11px] ${
                        r.label === 'Total'
                          ? 'border-[#D4AF37]/45 bg-[#78350F]/25 font-bold text-text-primary'
                          : 'border-[#D4AF37]/20 bg-bg-elevated text-text-secondary'
                      }`}
                    >
                      <span>{r.label}</span>
                      <span>{r.usd}</span>
                    </div>
                  ))}
                </div>

                <div className="space-y-1">
                  <div className="mb-1 font-display text-[10px] uppercase tracking-wider text-brass">
                    By provider
                  </div>
                  {providerRows(cost).length > 0 ? (
                    providerRows(cost).map((p) => (
                      <div
                        key={p.provider}
                        className="flex items-center justify-between rounded border border-[#D4AF37]/20 bg-bg-elevated px-2 py-1 font-mono text-[11px] text-text-secondary"
                      >
                        <span>{p.provider}</span>
                        <span className="font-medium text-text-primary">{p.usd}</span>
                      </div>
                    ))
                  ) : (
                    <div className="font-mono text-[11px] text-text-muted">No provider spend.</div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </section>
  );
}
