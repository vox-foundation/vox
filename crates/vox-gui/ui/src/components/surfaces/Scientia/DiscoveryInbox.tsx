import React, { useCallback, useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useLabel } from '../../../hooks/useLanguage';
import { listenDiscoverySurfaced, listenScientiaQueue } from '../../../transport';
import {
  listDiscoveryInbox,
  acknowledgeDiscovery,
  type DiscoveryInboxRow,
} from './discoveryInboxApi';
import { promoteCandidate } from '../../../api/scientia';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { seedDiscoveryPresetForLegacyKey } from '../../../lib/navigation';

/** The intake tier we treat as "strong" — gets a highlighted badge + a toast on arrival. */
const STRONG_TIER = 'strong_candidate';

function tierLabel(tier: string): string {
  return tier.replace(/_/g, ' ');
}

function formatDiscoveryCandidateTitle(r: DiscoveryInboxRow): string {
  const cleanTokens = r.publication_id
    .replace(/^(pub|finding|commit)-/i, '')
    .split(/[-_]+/)
    .filter((tok) => tok && !/^[0-9a-f]{8,}$/i.test(tok))
    .map((tok) => tok.charAt(0).toUpperCase() + tok.slice(1));
  const subject = cleanTokens.length > 0 ? cleanTokens.join(' ') : r.publication_id;
  if (r.origin === 'research') {
    return `${subject} · Grounded Research Candidate`;
  }
  if (r.signal_codes.length > 0) {
    return `${subject} · ${r.signal_codes[0]!.replace(/[._]/g, ' ')}`;
  }
  return `${subject} · Scholarly Discovery Candidate`;
}

/** Compact relative-time string for a past epoch-ms timestamp. */
function relativeTime(ms: number): string {
  const delta = Date.now() - ms;
  if (delta < 0 || !Number.isFinite(delta)) return 'just now';
  const sec = Math.floor(delta / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}

/**
 * Task 18 — Discovery Inbox. Lists unacknowledged surfaced research candidates
 * (rows in `scientia_discovery_inbox`). Each row can be acknowledged (drops it
 * from the list) or opened in the Discovery Review surface (deep-link).
 *
 * Live updates: subscribes to `vox://scientia-discovery-surfaced` (one row per
 * emission, mirroring the orchestrator-mcp `scientia.discovery.surfaced` WS topic
 * at the Tauri boundary) and to the `vox://scientia-queue` change ping. A 10 s
 * interval covers the non-Tauri / no-bridge case.
 *
 * On a newly-arrived strong candidate we raise an in-app toast. OS notifications
 * are intentionally NOT wired: the `tauri-plugin-notification` plugin is not a
 * workspace dependency, so adding it here would risk a version-resolution mess
 * for a degradable nicety — the toast is the graceful baseline.
 */
export function DiscoveryInbox({ pushToast }: SurfaceDecoratorProps) {
  const embedded = useIsEmbeddedSurface();
  const [rows, setRows] = useState<DiscoveryInboxRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const fetchingRef = useRef(false);
  // Ids we've already toasted, so a refetch doesn't re-announce the same rows.
  const seenStrongIds = useRef<Set<number>>(new Set());
  // First load shouldn't toast the whole backlog as if it just arrived.
  const primedRef = useRef(false);

  const refresh = useCallback(async (opts?: { silent?: boolean }) => {
    if (fetchingRef.current) return;
    fetchingRef.current = true;
    const showSpinner = !opts?.silent && !primedRef.current;
    if (showSpinner) setLoading(true);
    try {
      const next = await listDiscoveryInbox();
      // Announce strong candidates that are new since the last fetch.
      if (primedRef.current) {
        for (const r of next) {
          if (r.intake_tier === STRONG_TIER && !seenStrongIds.current.has(r.id)) {
            pushToast({
              tone: 'info',
              title: 'New research candidate',
              body: `${formatDiscoveryCandidateTitle(r)} (${r.publication_id})`,
              cause: 'backend-ok',
            });
          }
        }
      }
      for (const r of next) {
        if (r.intake_tier === STRONG_TIER) seenStrongIds.current.add(r.id);
      }
      primedRef.current = true;
      setRows(next);
    } catch (err) {
      if (!opts?.silent) {
        pushToast({ tone: 'warn', title: 'Discovery inbox', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      }
      if (!primedRef.current) setRows([]);
    } finally {
      if (showSpinner) setLoading(false);
      fetchingRef.current = false;
    }
  }, [pushToast]);

  // Initial fetch + visibility-gated 10 s interval fallback.
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

  // Event-driven refresh on discovery-surfaced rows + scientia-queue ping.
  useEffect(() => {
    // Embedded mini-render: no pushed subscriptions either — stays static.
    if (embedded) return;
    let unlistenDiscovery: (() => void) | undefined;
    let unlistenQueue: (() => void) | undefined;

    listenDiscoverySurfaced((row) => {
      const incoming: DiscoveryInboxRow = {
        id: row.id,
        publication_id: row.publication_id,
        surfaced_at_ms: row.surfaced_at_ms,
        intake_tier: row.intake_tier,
        signal_codes: row.signal_codes,
        origin: row.origin,
      };
      setRows((prev) => {
        if (prev.some((r) => r.id === incoming.id)) return prev;
        return [incoming, ...prev].sort((a, b) => b.surfaced_at_ms - a.surfaced_at_ms);
      });
      if (primedRef.current && row.intake_tier === STRONG_TIER && !seenStrongIds.current.has(row.id)) {
        pushToast({
          tone: 'info',
          title: 'New research candidate',
          body: `${row.publication_id}${row.signal_codes.length ? ` · ${row.signal_codes.join(', ')}` : ''}`,
          cause: 'backend-ok',
        });
      }
      if (row.intake_tier === STRONG_TIER) seenStrongIds.current.add(row.id);
    })
      .then((fn) => {
        unlistenDiscovery = fn;
      })
      .catch(() => {
        /* not in Tauri — interval fallback covers it */
      });

    listenScientiaQueue(() => {
      void refresh();
    })
      .then((fn) => {
        unlistenQueue = fn;
      })
      .catch(() => {
        /* not in Tauri — interval fallback covers it */
      });

    return () => {
      unlistenDiscovery?.();
      unlistenQueue?.();
    };
  }, [pushToast, refresh, embedded]);

  const acknowledge = useCallback(
    async (id: number) => {
      setBusyId(id);
      try {
        await acknowledgeDiscovery(id);
        setRows((prev) => prev.filter((r) => r.id !== id));
        seenStrongIds.current.delete(id);
      } catch (err) {
        pushToast({ tone: 'warn', title: 'Acknowledge failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      } finally {
        setBusyId(null);
      }
    },
    [pushToast],
  );

  const openReview = useCallback((publicationId: string) => {
    seedDiscoveryPresetForLegacyKey('discovery-review');
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
    window.dispatchEvent(
      new CustomEvent('vox://navigate-surface', {
        detail: { view: 'scientia', publicationId },
      }),
    );
  }, []);

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <h2 className="font-display text-lg tracking-wider text-text-primary uppercase">{useLabel('discovery-inbox')}</h2>
          <p className="font-sans text-xs text-text-muted">
            Unacknowledged surfaced research candidates. Updates automatically via live Scientia queue events.
          </p>
        </div>
      </div>

      <div className="flex flex-col overflow-hidden rounded-xl border border-border-subtle bg-bg-surface">
        <div className="flex items-center justify-between border-b border-border-subtle px-4 py-3">
          <span className="font-display text-xs uppercase tracking-[0.18em] text-text-muted">
            Unacknowledged ({rows.length})
          </span>
        </div>

        {rows.length === 0 && !loading && (
          <div className="px-4 py-10 text-center text-xs text-text-muted">
            No unacknowledged research candidates.
          </div>
        )}

        <div className="flex flex-col" role="list" aria-live="polite">
          {rows.map((r) => {
            const strong = r.intake_tier === STRONG_TIER;
            const researchOriginated = r.origin === 'research';
            return (
              <div
                key={r.id}
                role="listitem"
                className="flex flex-wrap items-start justify-between gap-4 border-b border-border-subtle px-4 py-3 last:border-b-0"
              >
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`rounded border px-2 py-0.5 font-sans text-[11px] font-semibold uppercase tracking-wider ${
                        strong
                          ? 'border-brass/45 bg-bg-elevated text-brass'
                          : 'border-border-subtle bg-bg-elevated text-text-muted'
                      }`}
                    >
                      {tierLabel(r.intake_tier)}
                    </span>
                    {researchOriginated && (
                      <span className="rounded border border-(--color-status-info)/40 bg-bg-elevated px-2 py-0.5 font-sans text-[11px] font-medium uppercase tracking-wider text-(--color-status-info)">
                        research
                      </span>
                    )}
                    <span className="truncate font-serif text-xs font-semibold text-text-primary">
                      {formatDiscoveryCandidateTitle(r)}
                    </span>
                    <span className="truncate font-mono text-[11px] font-medium text-text-muted">{r.publication_id}</span>
                    <span className="ml-auto shrink-0 font-mono text-[11px] text-text-muted">
                      {relativeTime(r.surfaced_at_ms)}
                    </span>
                  </div>
                  {r.signal_codes.length > 0 && (
                    <div className="mt-1.5 flex flex-wrap gap-1.5">
                      {r.signal_codes.map((c) => (
                        <span
                          key={c}
                          className="rounded border border-brass/30 bg-bg-elevated px-2 py-0.5 font-mono text-[11px] text-brass"
                        >
                          {c}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
                <div className="flex shrink-0 flex-wrap items-center gap-2">
                  <button
                    type="button"
                    data-slot="button"
                    data-testid={`promote-candidate-${r.id}`}
                    disabled={busyId === r.id}
                    onClick={async () => {
                      setBusyId(r.id);
                      try {
                        const promoted = await promoteCandidate(r.publication_id);
                        try {
                          window.localStorage.setItem(
                            'vox_discovery_review_seed',
                            promoted.publication_id,
                          );
                        } catch {
                          /* localStorage unavailable */
                        }
                        pushToast({
                          tone: 'ok',
                          title: 'Promoted to publication',
                          body: `${promoted.publication_id} (${promoted.claims_seeded} claims seeded)`,
                          cause: 'backend-ok',
                        });
                      } catch (err) {
                        pushToast({
                          tone: 'warn',
                          title: 'Promotion failed',
                          body: sanitizeErrorForToast(err),
                          cause: 'backend-error',
                        });
                      } finally {
                        setBusyId(null);
                      }
                    }}
                    className="inline-flex h-8 items-center rounded-lg border border-brass/45 bg-bg-elevated px-3 text-xs font-medium text-brass hover:bg-brass/15 disabled:opacity-40 transition-colors"
                  >
                    Promote to Publication
                  </button>
                  <button
                    type="button"
                    data-slot="button"
                    onClick={() => openReview(r.publication_id)}
                    className="inline-flex h-8 items-center rounded-lg border border-border-subtle bg-bg-elevated px-3 text-xs font-medium text-text-secondary hover:text-text-primary hover:border-brass/40 transition-colors"
                  >
                    Open review
                  </button>
                  <button
                    type="button"
                    data-slot="button"
                    disabled={busyId === r.id}
                    onClick={() => acknowledge(r.id)}
                    className="inline-flex h-8 items-center rounded-lg border border-(--color-status-pass)/40 bg-bg-elevated px-3 text-xs font-medium text-(--color-status-pass) hover:border-(--color-status-pass) disabled:opacity-40 transition-colors"
                  >
                    {busyId === r.id ? 'Acknowledging…' : 'Acknowledge'}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
