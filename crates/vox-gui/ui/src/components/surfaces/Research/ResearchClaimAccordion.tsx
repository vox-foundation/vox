import { useState, useEffect } from 'react';
import { TrustChip, type TrustSignal } from './TrustChip';
import { VerdictBadge } from '../Scientia/ClaimsView';
import { SafeExternalLink } from './SafeExternalLink';

export interface ResearchClaimRow {
  claimId: string;
  text: string;
  verdict: string;
  confidence: number;
  resampleStability: number;
  citations: Array<{ url: string; trust: TrustSignal }>;
}

/**
 * Collapsed-by-default claim accordion for ResearchView: a one-line summary
 * (claims verified / contested / sources) that expands into per-claim rows
 * with verdict badge, confidence, resample-stability note, and per-citation
 * trust chips. Reuses ClaimsView's VerdictBadge and Research's TrustChip so
 * the visual language matches the rest of the trust UI.
 */
export function ResearchClaimAccordion({
  claims,
  sourceCount,
  highlightedClaimId,
  isExpanded,
  onExpandedChange,
}: {
  claims: ResearchClaimRow[];
  sourceCount: number;
  highlightedClaimId?: string;
  isExpanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
}) {
  const [internalExpanded, setInternalExpanded] = useState(false);
  const expanded = isExpanded !== undefined ? isExpanded : internalExpanded;

  const toggleExpanded = () => {
    const next = !expanded;
    setInternalExpanded(next);
    onExpandedChange?.(next);
  };

  // Automatically expand when a claim is highlighted
  useEffect(() => {
    if (highlightedClaimId && isExpanded === undefined) {
      setInternalExpanded(true);
    }
  }, [highlightedClaimId, isExpanded]);

  const contestedCount = claims.filter((c) => c.verdict?.toLowerCase() === 'contested').length;
  const contradictedCount = claims.filter(
    (c) => c.verdict?.toLowerCase() === 'contradicted' || c.verdict?.toLowerCase() === 'refuted',
  ).length;
  const supportedCount = claims.filter((c) => c.verdict?.toLowerCase() === 'supported').length;

  return (
    <div className="research-claim-accordion rounded-xl border border-border-subtle bg-overlay-subtle">
      <button
        type="button"
        aria-expanded={expanded}
        onClick={toggleExpanded}
        className="flex w-full items-center justify-between gap-2 px-3 py-2 font-mono text-[11px] uppercase tracking-wider text-text-secondary hover:bg-overlay-subtle"
      >
        <span>
          {claims.length} claim{claims.length === 1 ? '' : 's'} verified · {contestedCount} contested
          {contradictedCount > 0 ? ` · ${contradictedCount} contradicted` : ''} · {sourceCount}{' '}
          source{sourceCount === 1 ? '' : 's'}
        </span>
        <span className="font-mono text-xs text-text-muted" aria-hidden="true">
          {expanded ? '▲' : '▼'}
        </span>
      </button>
      {expanded && (
        <ul className="space-y-2 border-t border-border-subtle p-3" role="list">
          {claims.map((claim) => (
            <li
              key={claim.claimId}
              id={`claim-${claim.claimId}`}
              role="listitem"
              className={`rounded-lg border border-border-subtle bg-overlay-subtle p-3 transition-all ${
                claim.claimId === highlightedClaimId ? 'ring-2 ring-brass' : ''
              }`}
            >
              <div className="mb-1 flex items-center gap-2">
                <VerdictBadge verdict={claim.verdict} />
                <span className="font-mono text-[10px] text-text-muted">
                  confidence {Math.round(claim.confidence * 100)}%
                </span>
              </div>
              <div className="text-sm text-text-secondary">{claim.text}</div>
              <div className="mt-1 font-mono text-[10px] text-text-muted">
                {claim.resampleStability >= 0.5
                  ? `Stable across resamples (${Math.round(claim.resampleStability * 100)}%)`
                  : 'Verdict flipped in resampling — treat with care'}
              </div>
              {claim.citations.length > 0 && (
                <ul className="mt-2 space-y-1" role="list">
                  {claim.citations.map((cite) => (
                    <li key={cite.url} role="listitem" className="flex flex-wrap items-center gap-2">
                      <SafeExternalLink
                        url={cite.url}
                        className="truncate text-[11px] text-brass underline decoration-dotted hover:text-brass/80"
                      />
                      <TrustChip signal={cite.trust} />
                    </li>
                  ))}
                </ul>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
