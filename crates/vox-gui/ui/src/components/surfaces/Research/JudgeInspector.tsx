import React from 'react';

export interface JudgeClaim {
  claim_id?: string;
  claimId?: string;
  text?: string;
  verdict?: string;
  confidence?: number;
  resample_stability?: number;
  resampleStability?: number;
  citation_urls?: string[];
  citations?: unknown[];
  corroboration_count?: number;
  [key: string]: unknown;
}

export interface JudgeInspectorProps {
  confidenceTier?: string;
  sourceCount?: number;
  citationPrecision?: number;
  claims?: JudgeClaim[] | unknown[];
  rationale?: string;
}

export function JudgeInspector({
  confidenceTier = 'Direct',
  sourceCount = 0,
  citationPrecision = 1.0,
  claims = [],
  rationale,
}: JudgeInspectorProps) {
  const claimList = (claims ?? []) as JudgeClaim[];

  const supportedCount = claimList.filter((c) => {
    const v = (c?.verdict ?? '').toLowerCase();
    return v === 'supported' || v === 'verified';
  }).length;

  const contestedCount = claimList.filter((c) => {
    const v = (c?.verdict ?? '').toLowerCase();
    return v === 'contested';
  }).length;

  const refutedCount = claimList.filter((c) => {
    const v = (c?.verdict ?? '').toLowerCase();
    return v === 'refuted' || v === 'contradicted';
  }).length;

  const precisionPct =
    citationPrecision != null ? `${Math.round(citationPrecision * 100)}%` : '100%';

  const computedRationale =
    rationale ??
    (claimList.length === 0
      ? `Epistemic Judge evaluated 0 claim(s) across ${sourceCount} independent source domain(s). No discrete claims were extracted for epistemic evaluation.`
      : `Epistemic Judge evaluated ${claimList.length} claim(s) across ${sourceCount} independent source domain(s) with ${precisionPct} citation precision. ${
          refutedCount > 0
            ? `Identified ${refutedCount} refuted claim(s) contradicted by ground evidence.`
            : contestedCount > 0
            ? `Flagged ${contestedCount} contested claim(s) requiring multi-perspective verification.`
            : 'All verified claims are well-grounded in primary source evidence.'
        } Routing confidence classified as ${confidenceTier}.`);

  return (
    <div
      data-testid="judge-inspector"
      className="rounded-lg border border-border-subtle bg-bg-surface p-3 space-y-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="font-display text-xs uppercase tracking-wide text-text-primary">
          Epistemic Judge Inspector
        </h3>
        <span className="text-xs text-text-muted">
          Verification metrics, claim verdict distribution, and grounding details
        </span>
      </div>

      {/* Metric Badges */}
      <div className="grid grid-cols-3 gap-2">
        <div className="flex flex-col rounded-lg border border-border-subtle bg-bg-elevated p-2.5">
          <span className="text-[11px] font-medium uppercase tracking-wider text-text-muted">
            Confidence Tier
          </span>
          <span
            data-testid="metric-confidence-tier"
            className="font-mono text-sm font-semibold text-brass"
          >
            {confidenceTier}
          </span>
        </div>

        <div className="flex flex-col rounded-lg border border-border-subtle bg-bg-elevated p-2.5">
          <span className="text-[11px] font-medium uppercase tracking-wider text-text-muted">
            Source Count
          </span>
          <span
            data-testid="metric-source-count"
            className="font-mono text-sm font-semibold text-text-primary"
          >
            {sourceCount}
          </span>
        </div>

        <div className="flex flex-col rounded-lg border border-border-subtle bg-bg-elevated p-2.5">
          <span className="text-[11px] font-medium uppercase tracking-wider text-text-muted">
            Citation Precision
          </span>
          <span
            data-testid="metric-citation-precision"
            className="font-mono text-sm font-semibold text-text-primary"
          >
            {precisionPct}
          </span>
        </div>
      </div>

      {/* Claims Verdict Distribution */}
      <div className="rounded-lg border border-border-subtle bg-bg-panel p-3 space-y-2">
        <div className="text-[11px] font-medium uppercase tracking-wider text-text-muted">
          Claims Verdict Distribution
        </div>
        <div className="grid grid-cols-3 gap-2">
          <div className="flex items-center justify-between rounded border border-(--color-status-pass)/35 bg-bg-elevated px-2.5 py-1.5 text-xs">
            <span className="font-medium text-(--color-status-pass)">Supported</span>
            <span data-testid="count-supported" className="font-mono font-bold text-(--color-status-pass)">
              {supportedCount}
            </span>
          </div>
          <div className="flex items-center justify-between rounded border border-(--color-status-warn)/35 bg-bg-elevated px-2.5 py-1.5 text-xs">
            <span className="font-medium text-(--color-status-warn)">Contested</span>
            <span data-testid="count-contested" className="font-mono font-bold text-(--color-status-warn)">
              {contestedCount}
            </span>
          </div>
          <div className="flex items-center justify-between rounded border border-(--color-status-fail)/35 bg-bg-elevated px-2.5 py-1.5 text-xs">
            <span className="font-medium text-(--color-status-fail)">Refuted</span>
            <span data-testid="count-refuted" className="font-mono font-bold text-(--color-status-fail)">
              {refutedCount}
            </span>
          </div>
        </div>
      </div>

      {/* Chain-of-Thought Rationale & Grounding Details */}
      <div className="rounded-lg border border-border-subtle bg-bg-panel p-3 space-y-2">
        <div className="text-[11px] font-medium uppercase tracking-wider text-text-muted">
          Chain-of-Thought Rationale &amp; Grounding Details
        </div>
        <div
          data-testid="judge-rationale"
          className="text-xs leading-relaxed text-text-secondary bg-bg-elevated border border-border-subtle rounded p-2.5 font-sans"
        >
          {computedRationale}
        </div>

        {claimList.length > 0 && (
          <div className="space-y-1.5 pt-1">
            <div className="text-[11px] font-medium uppercase tracking-wider text-text-muted">
              Grounding Claims ({claimList.length})
            </div>
            <div className="space-y-1 max-h-[220px] overflow-auto">
              {claimList.map((claim, idx) => {
                const id = claim?.claim_id ?? claim?.claimId ?? `c${idx + 1}`;
                const verdict = claim?.verdict ?? 'Unverified';
                const conf =
                  claim?.confidence != null ? `${Math.round(claim.confidence * 100)}%` : null;
                const stab =
                  (claim?.resample_stability ?? claim?.resampleStability) != null
                    ? `${Math.round((claim.resample_stability ?? claim.resampleStability) * 100)}%`
                    : null;
                return (
                  <div
                    key={id}
                    className="rounded border border-border-subtle bg-bg-elevated p-2.5 text-xs"
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="font-mono text-text-primary font-semibold">Claim {id}</span>
                      <span className="font-mono text-text-muted text-[11px]">
                        {verdict}
                        {conf ? ` · conf: ${conf}` : ''}
                        {stab ? ` · stability: ${stab}` : ''}
                      </span>
                    </div>
                    <p className="text-text-secondary text-xs leading-relaxed">{claim?.text}</p>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
