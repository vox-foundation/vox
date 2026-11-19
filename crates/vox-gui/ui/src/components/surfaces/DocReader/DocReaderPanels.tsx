import React from 'react';
import { Button } from '../../ui/Button';
import { VoxSelect } from '../../ui/Select';
import { OpenWikiHealStudio } from './OpenWikiHealStudio';
import {
  DOMAIN_OPTIONS,
  TOPIC_OPTIONS,
  TRUST_OPTIONS,
  type KbFindingItem,
  type KbPromotionScope,
  type KbSearchResultItem,
  type KbStatusDto,
} from './docReaderHelpers';

export interface DocReaderLibraryBarProps {
  libraryItems: KbSearchResultItem[];
  filteredResults: KbSearchResultItem[];
  effectivePath: string;
  searchQuery: string;
  topicFilter: string;
  domainFilter: string;
  trustFilter: string;
  onSearchChange: (q: string) => void;
  onTopicChange: (v: string) => void;
  onDomainChange: (v: string) => void;
  onTrustChange: (v: string) => void;
  onSelectDoc: (docPath: string) => void;
}

export function DocReaderLibraryBar({
  libraryItems,
  filteredResults,
  effectivePath,
  searchQuery,
  topicFilter,
  domainFilter,
  trustFilter,
  onSearchChange,
  onTopicChange,
  onDomainChange,
  onTrustChange,
  onSelectDoc,
}: DocReaderLibraryBarProps) {
  const facetGroups = [
    { label: 'Filter by topic', testId: 'kb-filter-topic', prefix: 'Topic', value: topicFilter, options: TOPIC_OPTIONS, onChange: onTopicChange },
    { label: 'Filter by domain', testId: 'kb-filter-domain', prefix: 'Domain', value: domainFilter, options: DOMAIN_OPTIONS, onChange: onDomainChange },
    { label: 'Filter by trust tier', testId: 'kb-filter-trust', prefix: 'Trust', value: trustFilter, options: TRUST_OPTIONS, onChange: onTrustChange },
  ] as const;

  const hasActiveFilter =
    searchQuery.trim().length > 0 || topicFilter !== 'all' || domainFilter !== 'all' || trustFilter !== 'all';

  return (
    <div data-testid="kb-facet-filter-bar" className="vox-marble-tablet mb-4 space-y-2.5 rounded-lg border border-(--color-gold-500)/50 bg-bg-surface p-3.5 text-xs">
      <div className="flex flex-wrap items-center gap-1.5" data-testid="kb-pinned-guides-bar">
        <span className="mr-1 font-display text-[11px] font-semibold uppercase tracking-wider text-text-primary">Live Index:</span>
        {libraryItems.slice(0, 5).map((pin) => (
          <button
            key={pin.path}
            type="button"
            data-slot="button"
            onClick={() => onSelectDoc(pin.path)}
            className={`inline-flex h-8 items-center rounded-md border px-2.5 font-sans text-xs transition-colors ${
              effectivePath === pin.path
                ? 'border-(--color-gold-400) bg-bg-elevated text-text-primary font-semibold'
                : 'border-border-strong bg-bg-elevated text-text-primary hover:border-(--color-gold-400)/60 font-medium'
            }`}
          >
            {pin.title.split(' (')[0]}
          </button>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <input
          type="search"
          data-testid="kb-search-input"
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="Hybrid search KB (BM25 + HNSW RRF)…"
          aria-label="Hybrid search KB"
          className="h-8 min-w-[180px] flex-1 rounded border border-border-strong bg-bg-elevated px-3 font-sans text-xs text-text-primary placeholder:text-text-secondary focus:border-(--color-gold-400) focus:outline-hidden"
        />
      </div>

      <details className="pt-0.5">
        <summary className="cursor-pointer font-mono text-[11px] text-text-muted select-none">
          Filter Facets ({hasActiveFilter ? 'Filtered' : 'All Topics, Domains & Trust'})
        </summary>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          {facetGroups.map((grp) => (
            <div key={grp.testId} role="group" aria-label={grp.label} data-testid={grp.testId} className="flex flex-wrap items-center gap-1">
              {grp.options.map((opt) => (
                <button
                  key={opt}
                  type="button"
                  data-slot="chip"
                  onClick={() => grp.onChange(opt)}
                  className={`inline-flex h-7 items-center rounded border px-2.5 font-sans text-[11px] transition-colors ${
                    grp.value === opt
                      ? 'border-(--color-gold-400) bg-bg-elevated text-text-primary font-semibold'
                      : 'border-border-strong bg-bg-elevated text-text-secondary hover:text-text-primary font-medium'
                  }`}
                >
                  {opt === 'all' ? `${grp.prefix}: All` : opt}
                </button>
              ))}
            </div>
          ))}
        </div>
      </details>

      {hasActiveFilter && (
        <div className="w-full pt-1.5" data-testid="kb-search-results">
          {filteredResults.length === 0 ? (
            <div className="py-2 text-xs text-text-muted">No live KB documents matched this filter.</div>
          ) : (
            filteredResults.map((res) => (
              <div key={res.path} className="flex items-center justify-between rounded px-2 py-1 hover:bg-bg-elevated">
                <button
                  type="button"
                  data-slot="button"
                  onClick={() => onSelectDoc(res.path)}
                  className="text-left font-mono text-xs text-brass hover:underline"
                >
                  {res.title} ({res.path})
                </button>
                <span className="font-mono text-[11px] text-text-muted">
                  {res.trustTier} · RRF {res.rrfScore.toFixed(2)}
                </span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

export function DocReaderInboxPanel({
  inboxItems,
  promoteScope,
  onScopeChange,
  onReview,
}: {
  inboxItems: KbFindingItem[];
  promoteScope: KbPromotionScope;
  onScopeChange: (s: KbPromotionScope) => void;
  onReview: (id: string, decision: 'promote' | 'reject') => void;
}) {
  return (
    <section data-testid="kb-inbox-panel" className="mb-4 space-y-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="font-display text-sm text-text-primary">Candidate KB Findings Inbox (VoxDb)</h2>
        <div className="flex items-center gap-2">
          <span className="font-sans text-[11px] font-semibold uppercase text-text-muted">Promote Scope:</span>
          <VoxSelect
            size="xs"
            aria-label="Promotion target scope"
            data-testid="kb-promote-scope-select"
            value={promoteScope}
            onValueChange={(v) => onScopeChange(v as KbPromotionScope)}
            options={[
              { value: 'repo_local', label: 'Repo-Local (docs/wiki/)' },
              { value: 'global_library', label: 'Global Library (~/.vox/library/)' },
            ]}
            className="h-8 w-56 border-border-subtle bg-bg-elevated font-mono text-[11px] text-text-primary"
          />
        </div>
      </div>
      {inboxItems.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border-subtle bg-bg-surface p-4 text-center text-xs text-text-muted">
          No candidate findings queued in VoxDb inbox.
        </div>
      ) : (
        inboxItems.map((item) => (
          <div
            key={item.id}
            data-testid={`kb-finding-row-${item.id}`}
            className="flex flex-col justify-between gap-2 rounded-lg border border-border-subtle bg-bg-surface p-3 sm:flex-row sm:items-center"
          >
            <div>
              <p className="text-xs font-medium text-text-primary leading-relaxed">{item.claim}</p>
              <p className="mt-1 font-mono text-[11px] text-text-muted">
                Anchor: [{item.citationAnchor}] · Tier: {item.trustTier} · Conf: {(item.confidence * 100).toFixed(0)}%
              </p>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <Button variant="outline" size="sm" data-testid={`kb-promote-${item.id}`} className="h-8 px-3 text-xs" onClick={() => onReview(item.id, 'promote')}>
                Promote to Verified
              </Button>
              <Button variant="ghost" size="sm" data-testid={`kb-reject-${item.id}`} className="h-8 px-3 text-xs" onClick={() => onReview(item.id, 'reject')}>
                Reject
              </Button>
            </div>
          </div>
        ))
      )}
    </section>
  );
}

export function DocReaderHealthPanel({
  kbStatus,
  staleVerifiedCount,
  onVerifyStale,
  onSelectDoc,
}: {
  kbStatus: KbStatusDto | null;
  staleVerifiedCount: number;
  onVerifyStale: () => void;
  onSelectDoc: (docPath: string) => void;
}) {
  return (
    <section data-testid="kb-health-panel" className="mb-4 space-y-3 rounded-lg border border-border-subtle bg-bg-surface p-3.5">
      <h2 className="font-display text-sm text-text-primary">KB Corpus Freshness &amp; Retrieval Quality</h2>
      <div className="grid grid-cols-2 gap-2 text-xs sm:grid-cols-4">
        <div className="rounded border border-border-subtle bg-bg-elevated p-2.5">
          <div className="text-text-muted">Indexed Corpus Pages</div>
          <div className="mt-0.5 font-mono text-sm font-semibold text-(--color-status-pass)">
            {kbStatus?.doc_count != null ? `${kbStatus.doc_count} docs · ${kbStatus.chunk_count ?? 0} chunks` : '—'}
          </div>
        </div>
        <div className="rounded border border-border-subtle bg-bg-elevated p-2.5">
          <div className="text-text-muted">Drifting / Deficits</div>
          <div className="mt-0.5 font-mono text-sm font-semibold text-(--color-status-pass)">
            {kbStatus?.drifting_count != null ? `${kbStatus.drifting_count} / ${kbStatus.deficit_count ?? 0}` : '—'}
          </div>
        </div>
        <div className="rounded border border-border-subtle bg-bg-elevated p-2.5">
          <div className="text-text-muted">Hybrid Retrieval</div>
          <div className="mt-0.5 font-mono text-sm font-semibold text-brass">
            {kbStatus?.mrr_at_5 != null
              ? `MRR@5 ${kbStatus.mrr_at_5.toFixed(2)} · R@5 ${(kbStatus.recall_at_5 ?? 0).toFixed(2)}`
              : 'Live BM25 + HNSW'}
          </div>
        </div>
        <div className="rounded border border-border-subtle bg-bg-elevated p-2.5">
          <div className="text-text-muted">Staleness Verify Runs</div>
          <div className="mt-0.5 font-mono text-sm font-semibold text-text-primary" data-testid="kb-stale-verify-count">
            {staleVerifiedCount}
          </div>
        </div>
      </div>
      <Button variant="outline" size="sm" data-testid="kb-verify-stale-btn" className="h-8 px-3 text-xs" onClick={onVerifyStale}>
        Run Mechanical Staleness Check (vox kb verify)
      </Button>
      <OpenWikiHealStudio compact onSelectDocPath={onSelectDoc} />
    </section>
  );
}
