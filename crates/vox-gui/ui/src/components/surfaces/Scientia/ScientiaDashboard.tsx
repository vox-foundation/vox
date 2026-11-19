import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useLabel } from '../../../hooks/useLanguage';
import { invokeCmd, listenScientiaQueue } from '../../../transport';
import { fetchCostRollup, providerRows, quarterlyRows } from './costRollup';
import type { CostRollup } from './costRollup';
import { ArchiveStatusSummary } from './ArchiveStatusSummary';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { BentoCard, BentoGrid, BentoScrollRegion } from '../../ui/BentoGrid';
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
    <div className="vox-marble-tablet min-w-0 rounded-lg border border-brass/50 bg-bg-elevated px-3 py-2.5 shadow-xs">
      <div className={`font-display text-xl font-bold leading-none ${tone ?? 'text-text-primary'}`}>{value}</div>
      <div className="mt-1.5 break-words font-sans text-[11px] font-semibold uppercase tracking-wider text-text-muted">{label}</div>
    </div>
  );
}

export interface ScientiaDashboardProps extends SurfaceDecoratorProps {
  hideHeader?: boolean;
}

/**
 * Phase H structured dashboard: a QueueSnapshot from the native
 * `scientia_dashboard_snapshot` command, served by the daemon
 * (`GET /api/v2/scientia/queue`) so it reflects whichever Vox server Axis is
 * connected to, local or remote.
 */
export function ScientiaDashboard({ pushToast, hideHeader = false }: ScientiaDashboardProps) {
  const embedded = useIsEmbeddedSurface();
  const [snap, setSnap] = useState<QueueSnapshot | null>(null);
  const [cost, setCost] = useState<CostRollup | null>(null);
  const [loading, setLoading] = useState(false);
  const [sectionFilter, setSectionFilter] = useState<'all' | 'candidates' | 'claims' | 'stalls'>('all');
  // Guard against interval-triggered overlapping fetches while one is in flight.
  const fetchingRef = React.useRef(false);
  const hasLoadedRef = React.useRef(false);

  const refresh = useCallback(async (opts?: { silent?: boolean }) => {
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    const showSpinner = !opts?.silent && !hasLoadedRef.current;
    if (showSpinner) setLoading(true);
    try {
      const nextSnap = await invokeCmd<QueueSnapshot | null>('scientia_dashboard_snapshot');
      setSnap(nextSnap && typeof nextSnap === 'object' && 'candidates' in nextSnap ? nextSnap : null);
      hasLoadedRef.current = true;
      // Cost rollup is an independent producer (`vox scientia cost`); a failure
      // here must not blank the queue snapshot above.
      try {
        setCost(await fetchCostRollup());
      } catch (costErr) {
        if (!opts?.silent) {
          pushToast({ tone: 'warn', title: 'Scientia cost', body: sanitizeErrorForToast(costErr), cause: 'backend-error' });
        }
        setCost(null);
      }
    } catch (err) {
      if (!opts?.silent) {
        pushToast({ tone: 'warn', title: 'Scientia dashboard', body: sanitizeErrorForToast(err), cause: 'backend-error' });
        setSnap(null);
      }
    } finally {
      if (showSpinner) setLoading(false);
      fetchingRef.current = false;
    }
  }, [pushToast]);

  // Initial fetch + visibility-gated 10 s auto-refresh; interval is cleared on unmount.
  useEffect(() => {
    void refresh();
    // Embedded mini-render: one initial fetch only, no repeating poll.
    if (embedded) return;
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        void refresh({ silent: true });
      }
    };
    document.addEventListener('visibilitychange', onVisible);
    const id = setInterval(() => {
      if (document.visibilityState !== 'hidden') {
        void refresh({ silent: true });
      }
    }, 10_000);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [refresh, embedded]);

  // F2: event-driven refresh — refetch immediately when the Rust DB watcher
  // pushes a "vox://scientia-queue" ping. The 10 s interval above stays as a
  // fallback (e.g. outside Tauri, where listen() rejects). Cleans up on unmount.
  useEffect(() => {
    // Embedded mini-render: no pushed subscription either — stays static.
    if (embedded) return;
    let unlisten: (() => void) | undefined;
    listenScientiaQueue(() => {
      void refresh({ silent: true });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        /* not in Tauri or no event bridge — interval fallback covers it */
      });
    return () => unlisten?.();
  }, [refresh, embedded]);

  const openInStudio = useCallback((publicationId: string) => {
    try {
      window.localStorage.setItem('vox_discovery_review_seed', publicationId);
    } catch {
      /* localStorage unavailable */
    }
    window.dispatchEvent(
      new CustomEvent('vox://scientia-open-studio', {
        detail: { publicationId },
      }),
    );
  }, []);

  const ScientiaIcon = (Icon as any).scientia ?? Icon.beaker;
  const totalCandidates = snap?.candidates.total ?? 0;
  const stallCount = snap?.stalls.length ?? 0;
  const verifiableClaims = snap?.claims_pending.verifiable ?? 0;
  const totalPendingClaims = snap
    ? snap.claims_pending.verifiable +
      snap.claims_pending.abstained +
      snap.claims_pending.extraction_running
    : 0;

  return (
    <section className="@container/surface flex min-h-0 min-w-0 flex-col gap-3.5">
      {!hideHeader ? (
        <RomanSurfaceHeader
          domainTag="KNOWLEDGE · SCIENTIA"
          title="Scientia Epistemic Discovery & Verification Ledger"
          subtitle="Publication pipeline queue snapshot · Track machine-verified research claims, novelty evidence, and publication readiness."
          waxSealBadge="EPISTEMIC LEDGER"
          icon={<ScientiaIcon className="size-5 text-brass-bright" />}
          gauges={
            snap
              ? [
                  {
                    label: 'CANDIDATES',
                    valueText: `${totalCandidates}`,
                    ratio:
                      totalCandidates > 0
                        ? Math.max(0, totalCandidates - stallCount) / totalCandidates
                        : 0,
                    tone: 'gold',
                    purpose: 'Non-stalled publication candidates out of total queue',
                    actionHint:
                      sectionFilter === 'candidates'
                        ? 'Click to show all Scientia panels'
                        : 'Click to focus candidate classes and top confidence list',
                    active: sectionFilter === 'candidates',
                    onClick: () =>
                      setSectionFilter((f) => (f === 'candidates' ? 'all' : 'candidates')),
                  },
                  {
                    label: 'VERIFIABLE',
                    valueText: `${verifiableClaims}`,
                    ratio:
                      totalPendingClaims > 0 ? verifiableClaims / totalPendingClaims : 0,
                    tone: 'emerald',
                    purpose: 'Verifiable claims share of pending claim extraction queue',
                    actionHint:
                      sectionFilter === 'claims'
                        ? 'Click to show all Scientia panels'
                        : 'Click to focus claims KPIs',
                    active: sectionFilter === 'claims',
                    onClick: () =>
                      setSectionFilter((f) => (f === 'claims' ? 'all' : 'claims')),
                  },
                  {
                    label: 'STALLS',
                    valueText: `${stallCount}`,
                    ratio:
                      totalCandidates > 0
                        ? Math.min(1, stallCount / totalCandidates)
                        : stallCount > 0
                          ? 1
                          : 0,
                    tone: stallCount > 0 ? 'crimson' : 'emerald',
                    purpose: 'Stalled pipeline candidates relative to total candidates',
                    actionHint:
                      sectionFilter === 'stalls'
                        ? 'Click to show all Scientia panels'
                        : 'Click to isolate stalled candidates',
                    active: sectionFilter === 'stalls',
                    onClick: () =>
                      setSectionFilter((f) => (f === 'stalls' ? 'all' : 'stalls')),
                  },
                ]
              : undefined
          }
        />
      ) : (
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-subtle bg-bg-surface px-3 py-2 text-xs">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="font-sans text-[11px] font-semibold uppercase tracking-wider text-text-muted mr-1">
              Queue Focus
            </span>
            {(['all', 'candidates', 'claims', 'stalls'] as const).map((f) => (
              <button
                key={f}
                type="button"
                data-slot="chip"
                aria-pressed={sectionFilter === f}
                onClick={() => setSectionFilter(f)}
                className={`h-8 rounded border px-2.5 text-xs font-medium capitalize transition-colors ${
                  sectionFilter === f
                    ? 'border-brass/50 bg-brass/20 text-brass font-semibold'
                    : 'border-transparent text-text-secondary hover:bg-bg-elevated hover:text-text-primary'
                }`}
              >
                {f}
              </button>
            ))}
          </div>
          <span className="font-mono text-[11px] text-text-muted">Live auto-sync</span>
        </div>
      )}

      {/* Snapshot region: refetched on visibility-aware interval + event ping, so announce
          updates politely to assistive tech. */}
      <div aria-live="polite" className="space-y-3">
        {loading && !snap && <div className="text-xs text-text-muted">Loading queue snapshot…</div>}
        {!loading && !snap && (
          <div className="rounded-xl border border-dashed border-brass/35 bg-bg-surface p-4 text-xs text-text-secondary">
            Queue snapshot unavailable — auto-retrying…
          </div>
        )}

        {snap && (
          <>
            <div className="grid grid-cols-1 gap-2 @xs/surface:grid-cols-2 @xl/surface:grid-cols-4 @5xl/surface:grid-cols-7">
              <Kpi label="Candidates" value={snap.candidates.total} tone="text-brass" />
              <Kpi label="Verifiable claims" value={snap.claims_pending.verifiable} tone="text-(--color-status-pass)" />
              <Kpi label="Abstained claims" value={snap.claims_pending.abstained} tone="text-text-secondary" />
              <Kpi label="Extraction pending" value={snap.claims_pending.extraction_running} tone="text-(--color-status-warn)" />
              <Kpi label="Reply window" value={snap.manifests_in_reply_window.length} tone="text-text-primary" />
              <Kpi label="Retraction queue" value={snap.retraction_queue.length} tone="text-(--color-status-fail)" />
              <Kpi label="Stalls" value={snap.stalls.length} tone="text-(--color-status-fail)" />
            </div>

            <BentoGrid preset="balanced">
              {(sectionFilter === 'all' || sectionFilter === 'candidates') && (
                <BentoCard
                  span={sectionFilter === 'candidates' ? 'full' : 'default'}
                  className="gap-2.5 border-brass/35"
                >
                  <div className="font-display text-xs uppercase tracking-wider text-brass">
                    Candidate queue &amp; classes
                  </div>
                  {Object.keys(snap.candidates.by_class).length > 0 ? (
                    <div className="flex flex-wrap gap-1.5">
                      {Object.entries(snap.candidates.by_class).map(([cls, n]) => (
                        <span key={cls} className="rounded border border-brass/30 bg-bg-elevated px-2.5 py-1 font-mono text-xs text-text-secondary">
                          {cls} <span className="ml-1 font-bold text-text-primary">{n}</span>
                        </span>
                      ))}
                    </div>
                  ) : (
                    <div className="text-xs text-text-muted">No candidate classes queued.</div>
                  )}
                  {snap.candidates.top_5_by_confidence.length > 0 && (
                    <BentoScrollRegion ariaLabel="Top candidates by confidence" className="max-h-56 space-y-2">
                      {snap.candidates.top_5_by_confidence.map((c) => (
                        <button
                          type="button"
                          key={c.candidate_id}
                          onClick={() => openInStudio(c.candidate_id)}
                          title="Open candidate in Studio & Review"
                          className="flex w-full flex-wrap items-center gap-2 rounded-lg border border-brass/30 bg-bg-elevated px-3 py-2 text-left font-mono text-xs transition-colors hover:border-brass"
                        >
                          <RomanWaxSealBadge
                            label={`${c.confidence.toFixed(2)} CONF`}
                            tone={c.confidence >= 0.8 ? 'emerald' : c.confidence >= 0.5 ? 'gold' : 'crimson'}
                          />
                          <span className="truncate font-medium text-text-primary">{c.candidate_id}</span>
                          <span className="rounded border border-border-subtle bg-bg-surface px-2 py-0.5 text-[11px] text-text-secondary">
                            {c.candidate_class.replace(/_/g, ' ')}
                          </span>
                          <span className="ml-auto rounded border border-brass/35 bg-bg-surface px-2 py-0.5 text-[11px] font-semibold text-brass">
                            {c.state.replace(/_/g, ' ')}
                          </span>
                        </button>
                      ))}
                    </BentoScrollRegion>
                  )}
                </BentoCard>
              )}

              {(sectionFilter === 'all' || sectionFilter === 'claims') && (
                <BentoCard
                  span={sectionFilter === 'claims' ? 'full' : 'default'}
                  className="gap-2 border-brass/35"
                >
                  <div className="font-display text-xs uppercase tracking-wider text-brass">
                    Verifiability &amp; reply gates
                  </div>
                  <div className="grid grid-cols-1 gap-2 font-mono text-xs @xs/surface:grid-cols-3">
                    <div className="rounded border border-border-subtle bg-bg-elevated px-2.5 py-2">
                      <div className="font-sans text-[11px] font-medium uppercase text-text-muted">Verifiable</div>
                      <div className="mt-0.5 text-sm font-bold text-text-primary">{snap.claims_pending.verifiable}</div>
                    </div>
                    <div className="rounded border border-border-subtle bg-bg-elevated px-2.5 py-2">
                      <div className="font-sans text-[11px] font-medium uppercase text-text-muted">Abstained</div>
                      <div className="mt-0.5 text-sm font-bold text-text-primary">{snap.claims_pending.abstained}</div>
                    </div>
                    <div className="rounded border border-border-subtle bg-bg-elevated px-2.5 py-2">
                      <div className="font-sans text-[11px] font-medium uppercase text-text-muted">Extracting</div>
                      <div className="mt-0.5 text-sm font-bold text-text-primary">{snap.claims_pending.extraction_running}</div>
                    </div>
                  </div>
                  {snap.manifests_in_reply_window.length > 0 && (
                    <BentoScrollRegion ariaLabel="Manifests in reply window" className="max-h-40 space-y-1">
                      {snap.manifests_in_reply_window.map((m) => (
                        <button
                          type="button"
                          key={m}
                          onClick={() => openInStudio(m)}
                          title="Inspect manifest reply window in Studio & Review"
                          className="block w-full truncate rounded border border-border-subtle bg-bg-elevated px-2.5 py-1.5 text-left font-mono text-[11px] text-text-secondary transition-colors hover:border-brass hover:text-text-primary"
                        >
                          {m}
                        </button>
                      ))}
                    </BentoScrollRegion>
                  )}
                </BentoCard>
              )}

              {(sectionFilter === 'all' || sectionFilter === 'stalls') && (
                <BentoCard
                  span={sectionFilter === 'stalls' ? 'full' : 'default'}
                  className={snap.stalls.length > 0 ? 'gap-2 border-(--color-status-warn)/40' : 'gap-2 border-brass/35'}
                >
                  <div className="font-display text-xs uppercase tracking-wider text-brass">
                    Stall &amp; retraction diagnostics
                  </div>
                  {snap.stalls.length === 0 && snap.retraction_queue.length === 0 ? (
                    <div className="rounded border border-(--color-status-pass)/35 bg-bg-elevated px-3 py-2 text-xs text-(--color-status-pass)">
                      0 stalled candidates · pipeline healthy
                    </div>
                  ) : (
                    <BentoScrollRegion ariaLabel="Stalled pipeline candidates" className="max-h-52 space-y-1">
                      {snap.stalls.map((s) => (
                        <button
                          type="button"
                          key={s.candidate_id}
                          onClick={() => openInStudio(s.candidate_id)}
                          title="Inspect stalled candidate in Studio & Review"
                          className="flex w-full flex-wrap items-center gap-2 rounded border border-(--color-status-warn)/30 bg-bg-elevated px-2.5 py-1.5 text-left font-mono text-xs transition-colors hover:border-brass"
                        >
                          <span className="truncate font-medium text-text-primary">{s.candidate_id}</span>
                          <span className="text-text-secondary">{s.class.replace(/_/g, ' ')}</span>
                          <span className="ml-auto rounded border border-(--color-status-warn)/35 bg-bg-surface px-2 py-0.5 text-[11px] text-(--color-status-warn)">
                            {Math.round(s.stuck_for_ms / 86_400_000)}d stuck
                          </span>
                        </button>
                      ))}
                    </BentoScrollRegion>
                  )}
                </BentoCard>
              )}
            </BentoGrid>
          </>
        )}
      </div>

      <div className={cost ? 'grid grid-cols-1 gap-3 @3xl/surface:grid-cols-2' : 'grid grid-cols-1 gap-3'}>
        <ArchiveStatusSummary
          onFetchError={(message) =>
            pushToast({ tone: 'warn', title: 'Archive rollup', body: message, cause: 'backend-error' })
          }
        />

        {cost && (
          <BentoCard className="gap-2 border-brass/35">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="font-display text-xs uppercase tracking-wider text-brass">
                Cost this quarter
              </div>
              <div className="rounded border border-brass/40 bg-bg-elevated px-2.5 py-0.5 font-mono text-[11px] text-text-primary">
                avg/finding{' '}
                <span className="font-bold">${cost.per_finding_average_usd.toFixed(2)}</span>
              </div>
            </div>

            {cost.this_quarter.total_usd === 0 && cost.by_provider.length === 0 ? (
              <div className="text-xs text-text-muted">
                No cost recorded this quarter.
              </div>
            ) : (
              <div className="grid grid-cols-1 gap-3 @lg/surface:grid-cols-2">
                <div className="space-y-1">
                  {quarterlyRows(cost).map((r) => (
                    <div
                      key={r.label}
                      className={`flex items-center justify-between rounded border px-2.5 py-1.5 font-mono text-xs ${
                        r.label === 'Total'
                          ? 'border-brass/50 bg-bg-elevated font-bold text-text-primary'
                          : 'border-border-subtle bg-bg-surface text-text-secondary'
                      }`}
                    >
                      <span>{r.label}</span>
                      <span>{r.usd}</span>
                    </div>
                  ))}
                </div>

                <div className="space-y-1">
                  <div className="mb-1 font-display text-xs uppercase tracking-wider text-brass">
                    By provider
                  </div>
                  {providerRows(cost).length > 0 ? (
                    providerRows(cost).map((p) => (
                      <div
                        key={p.provider}
                        className="flex items-center justify-between rounded border border-border-subtle bg-bg-surface px-2.5 py-1.5 font-mono text-xs text-text-secondary"
                      >
                        <span>{p.provider}</span>
                        <span className="font-medium text-text-primary">{p.usd}</span>
                      </div>
                    ))
                  ) : (
                    <div className="text-xs text-text-muted">No provider spend.</div>
                  )}
                </div>
              </div>
            )}
          </BentoCard>
        )}
      </div>
    </section>
  );
}
