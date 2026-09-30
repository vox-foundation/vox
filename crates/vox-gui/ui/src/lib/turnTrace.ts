import type { TurnEventDto } from '../types/dashboard';
import type { ChatVerbosity } from '../hooks/useChatVerbosity';
import { isKnownTurnEvent, routingModelLabel } from './turnEvents';

export type TraceItemStatus = 'ok' | 'failed' | 'info';

/** One trace row; `count` > 1 when identical consecutive events were coalesced. */
export interface TraceItem {
  event: TurnEventDto;
  count: number;
  status: TraceItemStatus;
}

export interface TurnTraceSummary {
  /** `routingModelLabel` of the turn's routing decision, else the message's plain `modelId`. */
  modelLabel: string | null;
  tools: number;
  receiptsVerified: number;
  receiptsUnverified: number;
  delegations: number;
  researchWaves: number;
  durationMs: number | null;
}

export interface TurnTrace {
  summary: TurnTraceSummary;
  /** Rows inside the collapsible trace, in arrival order. */
  steps: TraceItem[];
  /** Things a human must act on; shown inline even while the trace is collapsed. */
  interrupts: TraceItem[];
  /** Actionable, non-urgent chips kept inline (skill activation and its "not this one"). */
  inline: TurnEventDto[];
  defaultExpanded: boolean;
}

const OK_KINDS = new Set(['delegation_spawned', 'research_milestone', 'receipt_claims']);

/** Interrupts are only what a human must act on (critique anti-spam policy, rule 1). */
export function isInterrupt(e: TurnEventDto): boolean {
  return e.kind === 'receipt_claims' && ((e.fabricated as number) > 0 || (e.unverified as number) > 0);
}

function statusOf(e: TurnEventDto): TraceItemStatus {
  if (e.kind === 'tool_receipt') return e.verified === true ? 'ok' : 'failed';
  if (OK_KINDS.has(e.kind)) return 'ok';
  return 'info';
}

/** Append `e`, or bump the count when it is identical to the previous item. */
function pushCoalesced(items: TraceItem[], e: TurnEventDto): void {
  const last = items[items.length - 1];
  if (last && JSON.stringify(last.event) === JSON.stringify(e)) {
    last.count += 1;
    return;
  }
  items.push({ event: e, count: 1, status: statusOf(e) });
}

/** Decide what one assistant turn's trace says and whether it opens by default. */
export function buildTurnTrace(
  events: TurnEventDto[] | undefined,
  verbosity: ChatVerbosity,
  meta: { latencyMs?: number; modelId?: string } = {},
): TurnTrace {
  const summary: TurnTraceSummary = {
    modelLabel: null,
    tools: 0,
    receiptsVerified: 0,
    receiptsUnverified: 0,
    delegations: 0,
    researchWaves: 0,
    durationMs:
      typeof meta.latencyMs === 'number' && Number.isFinite(meta.latencyMs) ? meta.latencyMs : null,
  };
  const steps: TraceItem[] = [];
  const interrupts: TraceItem[] = [];
  const inline: TurnEventDto[] = [];
  for (const e of events ?? []) {
    if (!isKnownTurnEvent(e)) continue;
    if (e.kind === 'skill_activated') {
      inline.push(e);
      continue;
    }
    if (isInterrupt(e)) {
      pushCoalesced(interrupts, e);
      continue;
    }
    if (e.kind === 'routing_decision' && summary.modelLabel === null) {
      summary.modelLabel = routingModelLabel(e);
    }
    if (e.kind === 'tool_receipt') {
      summary.tools += 1;
      if (e.verified === true) summary.receiptsVerified += 1;
      else summary.receiptsUnverified += 1;
    }
    if (e.kind === 'delegation_spawned') summary.delegations += 1;
    if (e.kind === 'research_milestone') summary.researchWaves += e.waves_executed as number;
    pushCoalesced(steps, e);
  }
  // No routing decision (history hydrate, attachments, unmapped providers, background tasks):
  // keep the reply's own model id, plainly, with no version or recency claim.
  if (summary.modelLabel === null && typeof meta.modelId === 'string' && meta.modelId.length > 0) {
    summary.modelLabel = meta.modelId;
  }
  const needsALook = steps.some((s) => s.status === 'failed') || interrupts.length > 0;
  const defaultExpanded = verbosity === 'verbose' || (verbosity === 'normal' && needsALook);
  return { summary, steps, interrupts, inline, defaultExpanded };
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** The one-line summary shown on the trace row. */
export function summaryText(s: TurnTraceSummary): string {
  const parts: string[] = [];
  if (s.modelLabel) parts.push(s.modelLabel);
  if (s.tools > 0) parts.push(plural(s.tools, 'tool', 'tools'));
  const receipts = s.receiptsVerified + s.receiptsUnverified;
  if (receipts > 0) {
    parts.push(
      s.receiptsUnverified > 0
        ? `${s.receiptsVerified}/${receipts} receipts verified`
        : `${plural(receipts, 'receipt', 'receipts')} ✓`,
    );
  }
  if (s.delegations > 0) parts.push(`${s.delegations} delegated`);
  if (s.researchWaves > 0) parts.push(plural(s.researchWaves, 'research wave', 'research waves'));
  if (s.durationMs != null) parts.push(`${(s.durationMs / 1000).toFixed(1)}s`);
  return parts.join(' · ');
}
