const BANNER_STYLE: Record<'contested' | 'high' | 'mixed' | 'refuted', string> = {
  high: 'bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30',
  mixed: 'bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30',
  contested: 'bg-red-500/15 text-red-300 ring-1 ring-red-500/30',
  refuted: 'bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30',
};

/**
 * One-line headline verdict summarizing a research run's overall trust
 * posture: high-confidence (no contested claims and >= 2 corroborating sources),
 * mixed (awaiting corroboration or some contested), refuted (contradicted claims),
 * or contested (>30% of claims contested). Sits above the report body so
 * the reader gets a trust signal before diving into prose.
 */
export function HeadlineVerdictBanner({
  confidenceTier,
  corroboratingSources,
  contestedClaims,
  contradictedClaims = 0,
  totalClaims,
}: {
  confidenceTier: 'Direct' | 'Light' | 'DeepResearch';
  corroboratingSources: number;
  contestedClaims: number;
  contradictedClaims?: number;
  totalClaims: number;
}) {
  let tone: 'contested' | 'high' | 'mixed' | 'refuted';
  let message: string;

  const contestRate = totalClaims > 0 ? contestedClaims / totalClaims : 0;
  if (contradictedClaims > 0) {
    tone = 'refuted';
    message = `Refuted by Evidence — ${contradictedClaims} of ${totalClaims} claims contradicted by sources`;
  } else if (contestRate > 0.3) {
    tone = 'contested';
    message = `Contested Evidence — ${contestedClaims} of ${totalClaims} claims contested (>30%)`;
  } else if (contestedClaims === 0 && corroboratingSources >= 2) {
    tone = 'high';
    message = `High confidence — ${corroboratingSources} corroborating sources, no contested claims`;
  } else if (corroboratingSources === 0) {
    tone = 'mixed';
    message = 'Preliminary — Awaiting evidence corroboration';
  } else {
    tone = 'mixed';
    message =
      contestedClaims > 0
        ? `Mixed evidence — ${contestedClaims} of ${totalClaims} claims contested, treat with care`
        : `Preliminary — 1 corroborating source, awaiting additional evidence`;
  }

  return (
    <div
      className={`headline-verdict-banner mb-2 flex items-center justify-between rounded-xl border border-border-subtle px-3 py-2 font-mono text-[11px] uppercase tracking-wide ${BANNER_STYLE[tone]}`}
    >
      <span>{message}</span>
      {confidenceTier && (
        <span className="opacity-75 text-[10px] lowercase tracking-normal">
          tier: {confidenceTier}
        </span>
      )}
    </div>
  );
}
