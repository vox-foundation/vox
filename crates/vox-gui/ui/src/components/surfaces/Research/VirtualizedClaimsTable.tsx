import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Icon } from '../../ui/Icons';
import { VerdictBadge } from '../Scientia/ClaimsView';
import type { ResearchClaimRow } from './ResearchClaimAccordion';
import { SafeExternalLink } from './SafeExternalLink';
import { TrustChip } from './TrustChip';

export interface VirtualizedClaimsTableProps {
  claims: ResearchClaimRow[];
  sourceCount: number;
  totalClaims?: number;
  highlightedClaimId?: string;
  hasMore?: boolean;
  loadingMore?: boolean;
  onLoadMore?: () => void;
  viewportHeight?: number;
  rowHeight?: number;
  overscan?: number;
}

const DEFAULT_VIEWPORT_HEIGHT = 380;
const DEFAULT_ROW_HEIGHT = 76;
const DEFAULT_OVERSCAN = 6;
const AUTO_EXPAND_THRESHOLD = 20;

export function VirtualizedClaimsTable({
  claims,
  sourceCount,
  totalClaims,
  highlightedClaimId,
  hasMore = false,
  loadingMore = false,
  onLoadMore,
  viewportHeight = DEFAULT_VIEWPORT_HEIGHT,
  rowHeight = DEFAULT_ROW_HEIGHT,
  overscan = DEFAULT_OVERSCAN,
}: VirtualizedClaimsTableProps) {
  const [expanded, setExpanded] = useState(() => claims.length > AUTO_EXPAND_THRESHOLD);
  const [verdictFilter, setVerdictFilter] = useState<'all' | 'supported' | 'contested' | 'contradicted'>('all');
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (highlightedClaimId) {
      setExpanded(true);
    }
  }, [highlightedClaimId]);

  useEffect(() => {
    if (claims.length > AUTO_EXPAND_THRESHOLD) {
      setExpanded(true);
    }
  }, [claims.length]);

  const counts = useMemo(() => {
    let contested = 0;
    let contradicted = 0;
    let supported = 0;
    for (const c of claims) {
      const v = c.verdict?.toLowerCase() ?? '';
      if (v === 'contested') contested += 1;
      else if (v === 'contradicted' || v === 'refuted') contradicted += 1;
      else if (v === 'supported' || v === 'verified') supported += 1;
    }
    return { contested, contradicted, supported };
  }, [claims]);

  const filteredClaims = useMemo(() => {
    if (verdictFilter === 'all') return claims;
    return claims.filter((c) => {
      const v = c.verdict?.toLowerCase() ?? '';
      if (verdictFilter === 'supported') return v === 'supported' || v === 'verified';
      if (verdictFilter === 'contested') return v === 'contested';
      return v === 'contradicted' || v === 'refuted';
    });
  }, [claims, verdictFilter]);

  const virtualizer = useVirtualizer({
    count: expanded ? filteredClaims.length : 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan,
    initialRect: { width: 800, height: viewportHeight },
    observeElementRect: (instance, cb) => {
      const el = instance.scrollElement;
      if (!el) return;
      const emit = () => {
        const rect = el.getBoundingClientRect();
        const width = el.clientWidth || el.offsetWidth || rect.width || 800;
        const height = el.clientHeight || el.offsetHeight || rect.height || viewportHeight;
        cb({ width: Math.round(width), height: Math.round(height) });
      };
      emit();
      if (typeof ResizeObserver === 'undefined') return;
      const ro = new ResizeObserver(() => emit());
      ro.observe(el);
      return () => ro.disconnect();
    },
  });

  useEffect(() => {
    if (!expanded || !highlightedClaimId) return;
    const idx = filteredClaims.findIndex((c) => c.claimId === highlightedClaimId);
    if (idx >= 0) {
      virtualizer.scrollToIndex(idx, { align: 'center' });
    }
  }, [expanded, highlightedClaimId, filteredClaims, virtualizer]);

  const virtualItems = virtualizer.getVirtualItems();
  const displayTotal = totalClaims ?? claims.length;

  return (
    <div
      data-testid="virtualized-claims-table"
      className="research-claim-accordion rounded-xl border border-border-subtle bg-overlay-subtle"
    >
      <button
        type="button"
        data-testid="virtualized-claims-toggle"
        aria-expanded={expanded}
        onClick={() => setExpanded((prev) => !prev)}
        className="flex w-full items-center justify-between gap-2 px-3 py-2 font-mono text-[11px] uppercase tracking-wider text-text-secondary hover:bg-overlay-subtle"
      >
        <span>
          {displayTotal} claim{displayTotal === 1 ? '' : 's'} verified · {counts.contested} contested
          {counts.contradicted > 0 ? ` · ${counts.contradicted} contradicted` : ''} · {sourceCount}{' '}
          source{sourceCount === 1 ? '' : 's'}
        </span>
        <span className="inline-flex items-center font-mono text-xs text-text-muted" aria-hidden="true">
          {expanded ? <Icon.chevronUp className="size-3.5 shrink-0" /> : <Icon.chevronDown className="size-3.5 shrink-0" />}
        </span>
      </button>

      {expanded && (
        <div className="border-t border-border-subtle p-3 space-y-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div role="group" aria-label="Filter claims by verdict" className="flex flex-wrap items-center gap-1">
              {(['all', 'supported', 'contested', 'contradicted'] as const).map((filterKey) => (
                <button
                  key={filterKey}
                  type="button"
                  data-testid={`claims-filter-${filterKey}`}
                  aria-pressed={verdictFilter === filterKey}
                  onClick={() => setVerdictFilter(filterKey)}
                  className={`rounded px-2 py-0.5 font-mono text-[10px] uppercase tracking-wider transition-colors ${
                    verdictFilter === filterKey
                      ? 'border border-brass/40 bg-brass/20 text-brass'
                      : 'border border-border-subtle bg-black/20 text-text-muted hover:text-text-secondary'
                  }`}
                >
                  {filterKey}
                </button>
              ))}
            </div>
            <span className="font-mono text-[10px] text-text-muted">
              Showing {filteredClaims.length} of {displayTotal}
            </span>
          </div>

          {filteredClaims.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border-subtle py-6 text-center text-xs text-text-muted">
              No claims match the active verdict filter.
            </div>
          ) : (
            <div
              ref={scrollRef}
              role="region"
              tabIndex={0}
              aria-label="Virtualized claims list"
              data-testid="virtualized-claims-viewport"
              style={{ height: Math.min(viewportHeight, Math.max(rowHeight, filteredClaims.length * rowHeight)) }}
              className="overflow-y-auto custom-scrollbar pr-1"
            >
              <div
                style={{
                  height: `${virtualizer.getTotalSize()}px`,
                  position: 'relative',
                  width: '100%',
                }}
              >
                {virtualItems.map((vItem) => {
                  const claim = filteredClaims[vItem.index];
                  if (!claim) return null;
                  const isHighlighted = claim.claimId === highlightedClaimId;
                  const primaryCitation = claim.citations[0];

                  return (
                    <div
                      key={claim.claimId}
                      data-testid="virtualized-claim-row"
                      data-index={vItem.index}
                      style={{
                        position: 'absolute',
                        top: 0,
                        left: 0,
                        width: '100%',
                        height: `${vItem.size}px`,
                        transform: `translateY(${vItem.start}px)`,
                      }}
                      className="pb-1.5"
                    >
                      <div
                        id={`claim-${claim.claimId}`}
                        className={`flex h-full flex-col justify-between rounded-lg border border-border-subtle bg-black/25 px-3 py-2 transition-colors ${
                          isHighlighted ? 'ring-2 ring-brass' : ''
                        }`}
                      >
                        <div className="flex items-center justify-between gap-2 min-w-0">
                          <div className="flex items-center gap-2 min-w-0">
                            <VerdictBadge verdict={claim.verdict} />
                            <span className="truncate text-xs text-text-primary" title={claim.text}>
                              {claim.text}
                            </span>
                          </div>
                          <span className="shrink-0 font-mono text-[10px] text-text-muted">
                            {Math.round(claim.confidence * 100)}% conf · {Math.round(claim.resampleStability * 100)}% stab
                          </span>
                        </div>

                        <div className="flex items-center justify-between gap-2 min-w-0 text-[10px]">
                          {primaryCitation ? (
                            <div className="flex items-center gap-2 min-w-0">
                              <SafeExternalLink
                                url={primaryCitation.url}
                                className="truncate text-[11px] text-brass underline decoration-dotted hover:text-brass/80"
                              />
                              <TrustChip signal={primaryCitation.trust} />
                            </div>
                          ) : (
                            <span className="font-mono text-text-muted">No direct citations</span>
                          )}
                          {claim.citations.length > 1 && (
                            <span className="shrink-0 font-mono text-text-muted">
                              +{claim.citations.length - 1} more
                            </span>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {hasMore && onLoadMore && (
            <div className="flex justify-end pt-1">
              <button
                type="button"
                data-testid="load-more-claims-btn"
                disabled={loadingMore}
                onClick={onLoadMore}
                className="rounded border border-brass/40 bg-brass/10 px-3 py-1 font-mono text-[11px] text-brass hover:bg-brass/20 disabled:opacity-50"
              >
                {loadingMore ? 'Loading claims…' : 'Load more claims'}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
