import React, { useCallback, useEffect, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { extractClaims, listPublicationClaims } from '../../../api/scientia';
import { listPublicationManifests } from './archiveApi';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';
import { useLabel } from '../../../hooks/useLanguage';

export interface ClaimRow {
  claim_id: number;
  text: string;
  is_numeric: boolean;
  verifiability_score: number | null;
  verdict: string | null;
  confidence: number | null;
  verifier_model: string | null;
  created_at_ms: number;
  source_url?: string | null;
}

export type VerdictFilter = 'all' | 'Supported' | 'Contradicted' | 'Unverified';

import { SafeExternalLink } from '../Research/SafeExternalLink';
export { SafeExternalLink };

const VERDICT_STYLE: Record<string, string> = {
  supported: 'bg-bg-elevated text-(--color-status-pass) border border-(--color-status-pass)/40',
  contested: 'bg-bg-elevated text-(--color-status-warn) border border-(--color-status-warn)/40',
  contradicted: 'bg-bg-elevated text-(--color-status-fail) border border-(--color-status-fail)/40',
  refuted: 'bg-bg-elevated text-(--color-status-fail) border border-(--color-status-fail)/40',
  abstain: 'bg-bg-surface text-text-secondary border border-border-subtle',
  unverified: 'bg-bg-surface text-text-secondary border border-border-subtle',
};

export function VerdictBadge({ verdict }: { verdict: string | null }) {
  const label = verdict ?? 'pending';
  const key = verdict ? verdict.toLowerCase() : '';
  const cls = key ? VERDICT_STYLE[key] ?? VERDICT_STYLE.abstain : 'bg-bg-surface text-text-muted border border-border-subtle';
  return (
    <span className={`inline-flex items-center rounded px-2 py-0.5 font-sans text-[11px] font-semibold uppercase tracking-wider ${cls}`}>{label}</span>
  );
}

export interface ClaimsViewProps extends Partial<SurfaceDecoratorProps> {
  initialClaims?: ClaimRow[];
}

/**
 * Scientia claims ledger: extract a publication's atomic claims (VeriScore ->
 * atomic -> span -> MiniCheck) and view them with verdicts via typed daemon RPCs.
 */
export function ClaimsView({ pushToast, initialClaims }: ClaimsViewProps) {
  const [publicationId, setPublicationId] = useState('');
  const [claims, setClaims] = useState<ClaimRow[] | null>(initialClaims ?? null);
  const [filter, setFilter] = useState<VerdictFilter>('all');
  const [busy, setBusy] = useState(false);
  const [recentManifests, setRecentManifests] = useState<
    Array<{ publication_id: string; state: string }>
  >([]);

  const toast = pushToast ?? (() => {});

  useEffect(() => {
    if (initialClaims !== undefined) return;
    let cancelled = false;
    listPublicationManifests(6)
      .then((rows) => {
        if (!cancelled && Array.isArray(rows)) setRecentManifests(rows);
      })
      .catch(() => {
        /* ignore when offline or outside backend */
      });
    return () => {
      cancelled = true;
    };
  }, [initialClaims]);

  const loadClaimsFor = useCallback(
    async (targetId: string) => {
      const id = targetId.trim();
      if (!id) return;
      setBusy(true);
      try {
        const out = await listPublicationClaims(id);
        setClaims(out.claims ?? []);
      } catch (err) {
        toast({ tone: 'warn', title: 'Load claims failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
        setClaims([]);
      } finally {
        setBusy(false);
      }
    },
    [toast],
  );

  const loadClaims = async () => loadClaimsFor(publicationId);

  const extract = async () => {
    if (!publicationId.trim()) return;
    setBusy(true);
    try {
      const out = await extractClaims(publicationId.trim());
      toast({
        tone: 'ok',
        title: 'Claim extraction',
        body: `Extracted ${out.total_atomic} claims (${out.supported} supported)`,
        cause: 'backend-ok',
      });
      await loadClaims();
    } catch (err) {
      toast({ tone: 'warn', title: 'Extraction failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const visibleClaims = (claims ?? []).filter((c) => {
    if (filter === 'all') return true;
    const v = (c.verdict ?? 'unverified').toLowerCase();
    if (filter === 'Supported') return v === 'supported';
    if (filter === 'Contradicted') return v === 'contradicted' || v === 'refuted' || v === 'contested';
    if (filter === 'Unverified') return v === 'unverified' || v === 'abstain' || !c.verdict;
    return true;
  });

  return (
    <section className="vox-marble-tablet space-y-4 rounded-xl border border-border-strong bg-bg-surface p-4 shadow-xs">
      <div className="border-b border-border-subtle pb-3">
        <h2 className="font-display text-lg tracking-wider text-text-primary uppercase">{useLabel('sci-claims')}</h2>
        <p className="mt-0.5 font-sans text-xs text-text-secondary">Atomic claim extraction and verification ledger</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <input
          type="text"
          value={publicationId}
          onChange={(e) => setPublicationId(e.target.value)}
          placeholder="publication id"
          className="h-9 min-w-[16rem] flex-1 rounded-lg border border-border-strong bg-bg-elevated px-3 font-mono text-sm text-text-primary focus:border-brass focus:outline-hidden"
        />
        <button
          type="button"
          data-slot="button"
          className="h-9 rounded-lg border border-border-strong bg-bg-elevated px-3.5 text-xs font-semibold uppercase tracking-wider text-text-primary hover:bg-brass/15 disabled:opacity-60 transition-colors"
          disabled={busy || !publicationId.trim()}
          onClick={extract}
        >
          {busy ? 'Working…' : 'Extract claims'}
        </button>
        <button
          type="button"
          data-slot="button"
          className="h-9 rounded-lg border border-border-strong bg-bg-elevated px-3.5 text-xs font-semibold uppercase tracking-wider text-text-primary hover:border-brass disabled:opacity-60 transition-colors"
          disabled={busy || !publicationId.trim()}
          onClick={loadClaims}
        >
          Load
        </button>
      </div>

      {recentManifests.length > 0 && (
        <div
          data-testid="claims-manifest-quick-picker"
          className="flex flex-wrap items-center gap-1.5"
          role="group"
          aria-label="Recent manuscripts quick picker"
        >
          <span className="font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted mr-1">
            Recent Manuscripts
          </span>
          {recentManifests.map((m) => (
            <button
              key={m.publication_id}
              type="button"
              data-slot="button"
              data-roman-plaque="true"
              onClick={() => {
                setPublicationId(m.publication_id);
                void loadClaimsFor(m.publication_id);
              }}
              className={`vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2.5 font-mono text-[11px] transition-colors ${
                publicationId === m.publication_id ? 'ring-2 ring-brass/70' : 'opacity-85'
              }`}
            >
              <span>{m.publication_id}</span>
            </button>
          ))}
        </div>
      )}

      {claims === null && (
        <div className="space-y-3">
          <div className="text-xs font-medium text-text-secondary">
            Enter a publication id, then Extract (runs the pipeline) or Load (reads persisted claims).
          </div>
          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-3">
            <div className="rounded-lg border border-border-strong bg-bg-elevated p-3">
              <div className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                1. VeriScore Extraction
              </div>
              <p className="mt-1 font-sans text-xs leading-relaxed text-text-secondary">
                Splits manuscript sections into atomic, falsifiable empirical propositions.
              </p>
            </div>
            <div className="rounded-lg border border-border-strong bg-bg-elevated p-3">
              <div className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                2. MiniCheck Audit
              </div>
              <p className="mt-1 font-sans text-xs leading-relaxed text-text-secondary">
                Cross-verifies numeric metrics and spans against grounded workspace sources.
              </p>
            </div>
            <div className="rounded-lg border border-border-strong bg-bg-elevated p-3">
              <div className="font-display text-xs font-semibold uppercase tracking-wider text-text-primary">
                3. Archival Ledger
              </div>
              <p className="mt-1 font-sans text-xs leading-relaxed text-text-secondary">
                Filters verdicts by Supported, Contested, or Unverified before DOI issuance.
              </p>
            </div>
          </div>
        </div>
      )}
      {claims !== null && claims.length === 0 && (
        <div className="text-xs text-text-muted">No claims recorded for this publication yet.</div>
      )}
      {claims !== null && claims.length > 0 && (
        <div className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-2">
            <div className="font-mono text-xs uppercase tracking-wider text-text-muted">
              {visibleClaims.length} of {claims.length} claim{claims.length === 1 ? '' : 's'}
            </div>
            <div className="flex items-center gap-1" role="tablist" aria-label="Verdict filters">
              {(['all', 'Supported', 'Contradicted', 'Unverified'] as const).map((tab) => (
                <button
                  key={tab}
                  type="button"
                  role="tab"
                  data-slot="tab"
                  aria-selected={filter === tab}
                  onClick={() => setFilter(tab)}
                  className={`h-8 rounded-md border px-2.5 text-xs font-medium uppercase tracking-wider transition-colors ${
                    filter === tab
                      ? 'bg-brass/20 text-brass border-brass/50 font-semibold'
                      : 'bg-bg-elevated text-text-muted hover:text-text-primary border-border-subtle'
                  }`}
                >
                  {tab}
                </button>
              ))}
            </div>
          </div>
          {visibleClaims.length === 0 ? (
            <div className="text-xs text-text-muted py-2">
              No claims matching verdict filter "{filter}".
            </div>
          ) : (
            <div className="space-y-2" role="list" aria-live="polite">
              {visibleClaims.map((c) => (
                <div key={c.claim_id} role="listitem" className="rounded-xl border border-border-subtle bg-bg-surface p-3.5">
                  <div className="mb-1.5 flex flex-wrap items-center gap-2">
                    <VerdictBadge verdict={c.verdict} />
                    {c.confidence != null && (
                      <span className="font-mono text-[11px] text-text-muted">conf {c.confidence.toFixed(2)}</span>
                    )}
                    {c.verifiability_score != null && (
                      <span className="font-mono text-[11px] text-text-muted">vscore {c.verifiability_score.toFixed(2)}</span>
                    )}
                    {c.is_numeric && (
                      <span className="rounded border border-brass/35 bg-bg-elevated px-1.5 py-0.5 font-mono text-[11px] uppercase tracking-wider text-brass">numeric</span>
                    )}
                    {c.verifier_model && (
                      <span className="ml-auto font-mono text-[11px] text-text-muted">{c.verifier_model}</span>
                    )}
                  </div>
                  <div className="font-serif text-sm text-text-primary leading-relaxed">{c.text}</div>
                  {c.source_url && (
                    <div className="mt-1.5 flex items-center gap-1.5">
                      <span className="text-xs text-text-muted">source:</span>
                      <SafeExternalLink url={c.source_url} className="font-mono text-xs text-brass hover:underline" />
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </section>
  );
}
