import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Icon } from '../../ui/Icons';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { invokeCmd, listenScientiaQueue } from '../../../transport';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import {
  listReviewQueue,
  recordDecision,
  nanopublish,
  suggestEvidence,
  type ClaimAwaitingReview,
  type ReviewDecision,
  type EvidenceSuggestion,
} from './discoveryReviewApi';
import { getNoveltyAssessment, type NoveltyAssessment } from './noveltyApi';
import { NoveltyEvidencePanel } from './NoveltyEvidencePanel';
import { WorthinessRadarPanel } from './WorthinessRadarPanel';
import { DirectPublicationDrawer } from './DirectPublicationDrawer';
import { computeWorthinessAxes } from '../../../api/scientia';

function verdictTone(verdict: string | null): string {
  const v = (verdict ?? '').toLowerCase();
  if (v.includes('support')) return 'text-(--color-status-pass)';
  if (v.includes('refut') || v.includes('contradict')) return 'text-(--color-status-fail)';
  if (v.includes('novel')) return 'text-brass';
  return 'text-text-muted';
}

/**
 * P3 Discovery Review — human-gated review of extracted claims before
 * nanopublication. Master/detail: the left pane lists claims awaiting review for
 * the entered publication id; the right pane reviews the selected claim and,
 * once approved in-session, exposes an offline nanopublish action.
 *
 * Post-approval vs queue-drop: an approved claim is terminal and the refetch
 * drops it from the queue. We keep `approvedIds` (a Set) plus a cache of the
 * last-seen detail for each claim, so the just-approved claim's detail and its
 * brass post-approval zone remain visible for the rest of the session.
 */
export function DiscoveryReview({ pushToast }: SurfaceDecoratorProps) {
  const embedded = useIsEmbeddedSurface();
  // Seed the publication id from a cross-surface deep-link (Discovery Inbox's
  // "Open review" stashes it in localStorage before switching here). Consumed
  // once so a manual edit later isn't clobbered.
  const [pubId, setPubId] = useState(() => {
    try {
      const seed = window.localStorage.getItem('vox_discovery_review_seed');
      if (seed) {
        window.localStorage.removeItem('vox_discovery_review_seed');
        return seed;
      }
    } catch { /* localStorage unavailable */ }
    return '';
  });
  const [queue, setQueue] = useState<ClaimAwaitingReview[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [reason, setReason] = useState('');
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [approvedIds, setApprovedIds] = useState<Set<number>>(new Set());
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [suggestions, setSuggestions] = useState<EvidenceSuggestion[]>([]);
  const [novelty, setNovelty] = useState<NoveltyAssessment | null>(null);
  const [noveltyLoading, setNoveltyLoading] = useState(false);
  const [noveltyError, setNoveltyError] = useState<string | null>(null);
  const [replayScore, setReplayScore] = useState<number | undefined>(undefined);
  // Cache of last-seen claim detail so approved (terminal) claims stay viewable
  // after the refetch removes them from the live queue.
  const detailCache = useRef<Map<number, ClaimAwaitingReview>>(new Map());
  const fetchingRef = useRef(false);
  const lastLoadedPubRef = useRef('');
  const [recentManifests, setRecentManifests] = useState<
    Array<{ publication_id: string; title?: string | null }>
  >([]);

  useEffect(() => {
    if (embedded) return;
    let cancelled = false;
    invokeCmd<Array<{ publication_id: string; title?: string | null }> | null>(
      'list_publication_manifests',
      { limit: 6 },
    )
      .then((rows) => {
        if (!cancelled && Array.isArray(rows)) setRecentManifests(rows);
      })
      .catch(() => {
        /* ignore offline or mock */
      });
    return () => {
      cancelled = true;
    };
  }, [embedded]);

  useEffect(() => {
    const onOpenStudio = (ev: Event) => {
      const pid = (ev as CustomEvent<{ publicationId?: string }>).detail?.publicationId?.trim();
      if (pid) {
        setPubId(pid);
      }
    };
    window.addEventListener('vox://scientia-open-studio', onOpenStudio as EventListener);
    return () =>
      window.removeEventListener('vox://scientia-open-studio', onOpenStudio as EventListener);
  }, []);

  const openInPublicationsStudio = useCallback(() => {
    const id = pubId.trim();
    if (!id) return;
    try {
      window.localStorage.setItem('vox_selected_publication_id', id);
    } catch {
      /* localStorage unavailable */
    }
    window.dispatchEvent(
      new CustomEvent('vox://navigate-surface', {
        detail: { view: 'publications', publicationId: id },
      }),
    );
  }, [pubId]);

  const worthinessAxes = useMemo(() => {
    const supportedCount = queue.filter((c) => (c.verdict ?? '').toLowerCase().includes('support')).length;
    const contradictedCount = queue.filter((c) => {
      const v = (c.verdict ?? '').toLowerCase();
      return v.includes('refut') || v.includes('contradict');
    }).length;
    return computeWorthinessAxes({
      claimCount: queue.length,
      supportedCount,
      contradictedCount,
      noveltyVerdict: novelty?.verdict_kind,
      replayabilityScore: replayScore,
    });
  }, [queue, novelty?.verdict_kind, replayScore]);

  const refresh = useCallback(async (opts?: { silent?: boolean }) => {
    const id = pubId.trim();
    if (!id) {
      setQueue([]);
      lastLoadedPubRef.current = '';
      return;
    }
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    const isInitialForPub = lastLoadedPubRef.current !== id;
    const showSpinner = !opts?.silent && isInitialForPub;
    if (showSpinner) setLoading(true);
    try {
      const rows = await listReviewQueue(id);
      for (const r of rows) detailCache.current.set(r.claim_id, r);
      lastLoadedPubRef.current = id;
      setQueue(rows);
    } catch (err) {
      if (!opts?.silent) {
        pushToast({ tone: 'warn', title: 'Review queue', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
      if (isInitialForPub) setQueue([]);
    } finally {
      if (showSpinner) setLoading(false);
      fetchingRef.current = false;
    }
  }, [pubId, pushToast]);

  // Initial fetch + visibility-gated 10 s interval fallback; cleared on unmount / pub change.
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
    const t = setInterval(() => {
      if (document.visibilityState !== 'hidden') {
        void refresh({ silent: true });
      }
    }, 10_000);
    return () => {
      clearInterval(t);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [refresh, embedded]);

  // Event-driven refresh — refetch on the F2 scientia-queue ping. Interval above
  // covers the non-Tauri / no-bridge case. Cleans up on unmount.
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
        /* not in Tauri — interval fallback covers it */
      });
    return () => unlisten?.();
  }, [refresh, embedded]);

  const selected: ClaimAwaitingReview | null = useMemo(() => {
    if (selectedId == null) return null;
    return queue.find((c) => c.claim_id === selectedId) ?? detailCache.current.get(selectedId) ?? null;
  }, [selectedId, queue]);

  const isApproved = selectedId != null && approvedIds.has(selectedId);

  // Lazily load the novelty assessment for the selected claim's publication.
  // Keyed on the publication id (the bundle is per-publication, not per-claim);
  // only fetches once a claim is selected so the detail pane is on screen.
  useEffect(() => {
    const id = pubId.trim();
    if (!id || selectedId == null) {
      setNovelty(null);
      setNoveltyError(null);
      setNoveltyLoading(false);
      return;
    }
    let cancelled = false;
    setNoveltyLoading(true);
    setNoveltyError(null);
    getNoveltyAssessment(id)
      .then((a) => {
        if (!cancelled) setNovelty(a);
      })
      .catch((err) => {
        if (!cancelled) {
          setNovelty(null);
          setNoveltyError(sanitizeErrorForToast(err));
        }
      })
      .finally(() => {
        if (!cancelled) setNoveltyLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [pubId, selectedId]);

  const decide = useCallback(
    async (decision: ReviewDecision) => {
      if (selectedId == null || !pubId.trim()) return;
      setBusy(true);
      try {
        await recordDecision(pubId.trim(), selectedId, decision, reason.trim() || undefined);
        pushToast({ tone: 'ok', title: `Claim #${selectedId} ${decision}`, body: `Publication ${pubId.trim()}`, cause: 'backend-ok' });
        if (decision === 'approved') {
          setApprovedIds((prev) => new Set(prev).add(selectedId));
        }
        setReason('');
        await refresh();
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Review decision failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      } finally {
        setBusy(false);
      }
    },
    [selectedId, pubId, reason, pushToast, refresh],
  );

  const doNanopublish = useCallback(async () => {
    if (selectedId == null || !pubId.trim()) return;
    setConfirmOpen(false);
    setBusy(true);
    try {
      const res = await nanopublish(pubId.trim(), selectedId);
      pushToast({
        tone: 'ok',
        title: `Nanopublished claim #${selectedId}`,
        body: `${res.published_state} · ${res.validated_offline ? 'validated offline' : 'unvalidated'} · ${res.trusty_uri}`,
        cause: 'backend-ok',
      });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Nanopublish failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  }, [selectedId, pubId, pushToast]);

  const doSuggest = useCallback(async () => {
    if (selectedId == null || !pubId.trim()) return;
    setBusy(true);
    try {
      const out = await suggestEvidence(pubId.trim(), selectedId);
      setSuggestions(out);
      if (out.length === 0) {
        pushToast({ tone: 'info', title: 'No evidence suggestions', body: 'The model returned nothing to act on.', cause: 'backend-ok' });
      }
    } catch (err) {
      setSuggestions([]);
      pushToast({ tone: 'warn', title: 'Evidence assist unavailable', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  }, [selectedId, pubId, pushToast]);

  // Esc closes the nanopublish confirm overlay.
  useEffect(() => {
    if (!confirmOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setConfirmOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [confirmOpen]);

  return (
    <section className="space-y-4">
      {/* header + publication input */}
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="font-display text-lg tracking-wider text-text-primary uppercase">Discovery Review</h2>
          <p className="font-mono text-xs text-text-muted">
            Human-gated review of extracted claims before nanopublication. Nothing is published to any network.
          </p>
        </div>
        <label className="flex flex-wrap items-center gap-2">
          <span className="font-sans text-[11px] font-semibold uppercase tracking-wider text-text-muted">Publication</span>
          <input
            value={pubId}
            onChange={(e) => setPubId(e.target.value)}
            placeholder="publication id"
            className="h-9 rounded-lg border border-border-subtle bg-bg-elevated px-3 font-mono text-xs text-text-primary placeholder:text-text-muted focus:border-brass/50 focus:outline-hidden"
          />
          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            data-testid="open-publications-studio-btn"
            onClick={openInPublicationsStudio}
            disabled={!pubId.trim()}
            title="Advance lifecycle stages, run 12-point arXiv readiness gates, and export tar.gz bundle in Publications Studio"
            className="vox-brass-plaque-btn inline-flex h-9 items-center gap-1.5 rounded-lg px-3.5 text-xs font-semibold transition-colors disabled:opacity-40"
          >
            <Icon.book className="size-3.5 shrink-0" aria-hidden="true" />
            <span>Open in Publications Studio</span>
          </button>
        </label>
      </div>

      {recentManifests.length > 0 && (
        <div
          data-testid="review-manifest-quick-picker"
          role="group"
          aria-label="Recent manuscripts quick picker"
          className="flex flex-wrap items-center gap-1.5"
        >
          <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted mr-1">
            Quick Pick
          </span>
          {recentManifests.map((m) => (
            <button
              key={m.publication_id}
              type="button"
              data-slot="button"
              data-roman-plaque="true"
              onClick={() => setPubId(m.publication_id)}
              className={`vox-brass-plaque-btn inline-flex h-8 max-w-[260px] items-center truncate rounded px-2.5 font-mono text-[11px] transition-colors ${
                pubId.trim() === m.publication_id ? 'ring-2 ring-brass/70' : 'opacity-85'
              }`}
            >
              <span className="truncate">{m.title ?? m.publication_id}</span>
            </button>
          ))}
        </div>
      )}

      {/* 6-Axis Worthiness Radar & Direct Publication Flywheel Drawer */}
      <WorthinessRadarPanel
        axes={worthinessAxes}
        noveltyVerdict={novelty?.verdict_kind}
      />
      <DirectPublicationDrawer
        publicationId={pubId}
        pushToast={pushToast}
        onReplayVerified={(score) => setReplayScore(score)}
        onFlywheelAdvanced={() => {
          void refresh();
        }}
      />

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[360px_1fr]">
        {/* LIST */}
        <div className="flex flex-col overflow-hidden rounded-xl border border-border-subtle bg-bg-surface">
          <div className="flex items-center justify-between border-b border-border-subtle px-4 py-3">
            <span className="font-display text-xs uppercase tracking-[0.16em] text-text-muted">
              Awaiting Review ({queue.length})
            </span>
          </div>
          <div className="flex flex-col" aria-live="polite">
            {!pubId.trim() && (
              <div className="px-4 py-8 text-center text-xs text-text-muted">
                Enter a publication id to load its review queue.
              </div>
            )}
            {pubId.trim() && queue.length === 0 && !loading && (
              <div className="px-4 py-8 text-center text-xs text-text-muted">
                No claims awaiting review.
              </div>
            )}
            {queue.map((c) => {
              const active = c.claim_id === selectedId;
              return (
                <button
                  type="button"
                  data-slot="claim-row"
                  key={c.claim_id}
                  aria-pressed={active}
                  onClick={() => {
                    setSelectedId(c.claim_id);
                    setReason('');
                    setSuggestions([]);
                  }}
                  className={`relative border-b border-border-subtle px-4 py-3 text-left transition-colors ${
                    active ? 'bg-bg-elevated' : 'hover:bg-bg-elevated/60'
                  }`}
                >
                  {active && (
                    <span
                      className="absolute left-0 top-1/2 h-6 w-[2px] -translate-y-1/2 rounded-r bg-brass"
                    />
                  )}
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-[11px] text-text-muted">#{c.claim_id}</span>
                    <span className={`font-mono text-[11px] font-medium ${verdictTone(c.verdict)}`}>{c.verdict ?? 'unverified'}</span>
                  </div>
                  <div className="mt-1 font-serif text-[13px] leading-snug text-text-primary">{c.text}</div>
                  <div className="mt-1.5 flex items-center gap-2 font-mono text-[11px] text-text-muted">
                    {c.confidence != null && (
                      <span className="inline-flex items-center gap-1 text-brass">
                        <Icon.star className="size-3 shrink-0" aria-hidden="true" />
                        <span>{c.confidence.toFixed(2)}</span>
                      </span>
                    )}
                    {c.is_numeric && <span>numeric</span>}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* DETAIL */}
        <div className="flex flex-col overflow-hidden rounded-xl border border-border-subtle bg-overlay-subtle">
          <div className="flex items-center justify-between border-b border-border-subtle px-5 py-3">
            <span className="font-display text-[10px] uppercase tracking-[0.2em] text-text-muted">Claim Detail</span>
            {selected && <span className="font-mono text-[10px] text-text-muted">claim #{selected.claim_id}</span>}
          </div>

          {!selected && (
            <div className="px-5 py-10 text-center font-mono text-[11px] text-text-muted">
              Select a claim to review.
            </div>
          )}

          {selected && (
            <div className="flex flex-col gap-4 p-5">
              <blockquote className="border-l-2 border-brass/40 pl-4 text-[15px] leading-relaxed text-text-primary">
                “{selected.text}”
              </blockquote>

              <div className="grid grid-cols-2 gap-x-8 gap-y-2 font-mono text-[11px]">
                <div className="flex justify-between border-b border-border-subtle pb-1">
                  <span className="text-text-muted">Verdict</span>
                  <span className={verdictTone(selected.verdict)}>{selected.verdict ?? 'unverified'}</span>
                </div>
                <div className="flex justify-between border-b border-border-subtle pb-1">
                  <span className="text-text-muted">Confidence</span>
                  <span className="text-brass">{selected.confidence != null ? selected.confidence.toFixed(2) : '—'}</span>
                </div>
                <div className="flex justify-between border-b border-border-subtle pb-1">
                  <span className="text-text-muted">Numeric</span>
                  <span className="text-text-secondary">{selected.is_numeric ? 'yes' : 'no'}</span>
                </div>
                <div className="flex justify-between border-b border-border-subtle pb-1">
                  <span className="text-text-muted">Verifier</span>
                  <span className="text-text-secondary">{selected.verifier_model ?? '—'}</span>
                </div>
              </div>

              {noveltyLoading && (
                <div className="font-mono text-[11px] text-text-muted">Loading novelty…</div>
              )}
              {!noveltyLoading && noveltyError && (
                <div className="font-mono text-[11px] text-text-muted">
                  Novelty evidence unavailable.
                </div>
              )}
              {!noveltyLoading && !noveltyError && novelty && (
                <NoveltyEvidencePanel assessment={novelty} />
              )}

              <div>
                <label className="font-sans text-[11px] font-semibold uppercase tracking-wider text-text-muted">Reason (optional)</label>
                <textarea
                  rows={2}
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  placeholder="Why approve / reject / defer…"
                  className="mt-1 w-full rounded-lg border border-border-subtle bg-bg-elevated px-3 py-2 text-xs text-text-primary placeholder:text-text-muted focus:border-brass/50 focus:outline-hidden"
                />
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  data-slot="button"
                  disabled={busy}
                  onClick={() => decide('approved')}
                  className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-(--color-status-pass)/40 bg-bg-elevated px-4 text-xs font-semibold text-(--color-status-pass) hover:border-(--color-status-pass) disabled:opacity-40 transition-colors"
                >
                  <Icon.check className="size-3.5 shrink-0" aria-hidden="true" /> Approve
                </button>
                <button
                  type="button"
                  data-slot="button"
                  disabled={busy}
                  onClick={() => decide('rejected')}
                  className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-(--color-status-fail)/40 bg-bg-elevated px-4 text-xs font-semibold text-(--color-status-fail) hover:border-(--color-status-fail) disabled:opacity-40 transition-colors"
                >
                  <Icon.close className="size-3.5 shrink-0" aria-hidden="true" /> Reject
                </button>
                <button
                  type="button"
                  data-slot="button"
                  disabled={busy}
                  onClick={() => decide('deferred')}
                  className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-border-subtle bg-bg-elevated px-4 text-xs font-medium text-text-secondary hover:text-text-primary disabled:opacity-40 transition-colors"
                >
                  <Icon.pause className="size-3.5 shrink-0" aria-hidden="true" /> Defer
                </button>
                <button
                  type="button"
                  data-slot="button"
                  disabled={busy}
                  onClick={doSuggest}
                  className="ml-auto inline-flex h-9 items-center gap-1.5 rounded-lg border border-brass/45 bg-bg-elevated px-4 text-xs font-medium text-brass hover:bg-brass/15 disabled:opacity-40 transition-colors"
                >
                  <Icon.spark className="size-3.5 shrink-0" aria-hidden="true" /> Suggest evidence improvements (LLM)
                </button>
              </div>

              {suggestions.length > 0 && (
                <div className="rounded-xl border border-brass/35 bg-bg-elevated p-4">
                  <div className="mb-2 font-display text-xs uppercase tracking-wider text-brass">
                    Evidence suggestions
                  </div>
                  <div className="space-y-2">
                    {suggestions.map((s, i) => (
                      <div key={i} className="text-xs leading-relaxed">
                        <span className="font-mono text-brass">[{s.kind}]</span>{' '}
                        <span className="font-medium text-text-primary">{s.summary}</span>
                        <div className="text-text-secondary">{s.rationale}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {isApproved && (
                <div className="rounded-xl border border-brass/35 bg-bg-elevated p-4">
                  <div className="inline-flex items-center gap-1.5 font-mono text-xs text-(--color-status-pass)">
                    <Icon.check className="size-3.5 shrink-0" aria-hidden="true" />
                    <span>Approved by you · approval token bound to this claim</span>
                  </div>
                  <div className="mt-3 flex flex-wrap items-center gap-3">
                    <button
                      type="button"
                      data-slot="button"
                      disabled={busy}
                      onClick={() => setConfirmOpen(true)}
                      className="inline-flex h-9 items-center gap-2 rounded-lg border border-brass/45 bg-brass/15 px-4 text-xs font-semibold text-brass hover:bg-brass/25 disabled:opacity-40"
                    >
                      <Icon.arrowUp className="size-3.5 shrink-0" aria-hidden="true" /> Nanopublish (offline)
                    </button>
                    <span className="text-xs text-text-muted">
                      builds, signs, and offline-validates locally with no network transfer
                    </span>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* confirm dialog */}
      {confirmOpen && selected && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center"
          style={{ background: 'rgba(0,0,0,.6)' }}
          onClick={() => setConfirmOpen(false)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label={`Nanopublish claim #${selected.claim_id}`}
            onClick={(e) => e.stopPropagation()}
            className="w-[460px] rounded-xl border border-border-subtle bg-bg-base/90 p-5 backdrop-blur-xl"
          >
            <div className="mb-1 font-display text-[11px] uppercase tracking-[0.2em] text-brass">
              Nanopublish claim #{selected.claim_id} (offline)
            </div>
            <p className="text-[12.5px] leading-relaxed text-text-secondary">
              Builds + signs + offline-validates, stores locally. Nothing is sent to any network.
            </p>
            <div className="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 font-mono text-[11px]">
              <span className="text-text-muted">Publication</span>
              <span className="text-text-secondary">{pubId.trim()}</span>
              <span className="text-text-muted">Claim</span>
              <span className="text-text-secondary">#{selected.claim_id}</span>
            </div>
            <div className="mt-5 flex items-center justify-end gap-2">
              <button
                type="button"
                onClick={() => setConfirmOpen(false)}
                className="rounded-lg border border-border-subtle bg-overlay-subtle px-4 py-2 text-[12px] text-text-secondary hover:bg-overlay-subtle"
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={doNanopublish}
                className="rounded-lg border border-brass/40 bg-brass/20 px-4 py-2 text-[12px] text-brass hover:bg-brass/30 disabled:opacity-40"
              >
                Build &amp; sign locally
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
