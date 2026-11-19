export type TrustSignal =
  | { kind: 'formal'; venueType: string; retracted: boolean }
  | { kind: 'corroborated'; sourceCount: number }
  | { kind: 'uncorroborated' };

const TRUST_STYLE: Record<string, string> = {
  formal: 'bg-bg-elevated text-(--color-status-pass) border border-(--color-status-pass)/35',
  retracted: 'bg-bg-elevated text-(--color-status-fail) border border-(--color-status-fail)/35',
  corroborated: 'bg-bg-elevated text-brass border border-brass/35',
  uncorroborated: 'bg-bg-surface text-text-muted border border-border-subtle',
};

function chip(cls: string, label: string) {
  return <span className={`inline-flex items-center rounded px-2 py-0.5 font-sans text-[11px] font-medium tracking-wide ${cls}`}>{label}</span>;
}

/**
 * 3-tier citation trust chip: formal (venue type, retraction check),
 * corroborated (N independent sources), or uncorroborated (single source).
 */
export function TrustChip({ signal }: { signal: TrustSignal }) {
  if (signal.kind === 'formal') {
    if (signal.retracted) {
      return chip(TRUST_STYLE.retracted, 'RETRACTED');
    }
    return chip(TRUST_STYLE.formal, `${signal.venueType} · not retracted`);
  }
  if (signal.kind === 'corroborated') {
    return chip(
      TRUST_STYLE.corroborated,
      `Confirmed by ${signal.sourceCount} independent source${signal.sourceCount === 1 ? '' : 's'}`
    );
  }
  return chip(TRUST_STYLE.uncorroborated, 'Single source — not independently corroborated');
}
