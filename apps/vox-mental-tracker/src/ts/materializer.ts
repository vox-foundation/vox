/**
 * Replay-able materialization over the append-only HealthEventLog.
 *
 * Inputs are the raw rows from `db.HealthEventLog.all()`. Outputs are pure functions
 * of those rows — same input set produces the same output regardless of insertion order.
 *
 * Correction semantics: a row whose `correction_of` is non-empty supersedes the row
 * it points to. A correction may itself be corrected, forming a chain. The latest row
 * in the chain is the effective row; `effective_event_id` is the chain root (the
 * earliest event_id) so consumers can group corrections back to their original event.
 */

import { isBackdated, type HealthEventRow } from "./export_contract";

export type MaterializedEvent = HealthEventRow & {
  /** event_id at the root of the correction chain (== event_id when uncorrected). */
  effective_event_id: string;
  /** Computed from event_at vs recorded_at via export_contract.isBackdated. */
  is_backdated: boolean;
};

function chainRoot(rowsById: Map<string, HealthEventRow>, startId: string): string {
  let current = startId;
  const seen = new Set<string>();
  while (true) {
    if (seen.has(current)) return current;
    seen.add(current);
    const r = rowsById.get(current);
    if (!r || !r.correction_of || !rowsById.has(r.correction_of)) return current;
    current = r.correction_of;
  }
}

export function resolveCorrections(rows: HealthEventRow[]): MaterializedEvent[] {
  const byId = new Map<string, HealthEventRow>();
  for (const r of rows) byId.set(r.event_id, r);

  const supersededBy = new Map<string, HealthEventRow>();
  for (const r of rows) {
    if (r.correction_of && byId.has(r.correction_of)) {
      const prev = supersededBy.get(r.correction_of);
      if (!prev || r.recorded_at_monotonic > prev.recorded_at_monotonic) {
        supersededBy.set(r.correction_of, r);
      }
    }
  }

  const isSuperseded = new Set<string>();
  for (const target of supersededBy.keys()) isSuperseded.add(target);

  const effective: MaterializedEvent[] = [];
  for (const r of rows) {
    if (isSuperseded.has(r.event_id)) continue;
    const root = chainRoot(byId, r.event_id);
    effective.push({
      ...r,
      effective_event_id: root,
      is_backdated: isBackdated(r.event_at, r.recorded_at),
    });
  }

  effective.sort((a, b) => {
    const ea = Number(a.event_at);
    const eb = Number(b.event_at);
    if (ea !== eb) return ea - eb;
    return a.event_id.localeCompare(b.event_id);
  });

  return effective;
}

function utcDateString(ms: number): string {
  const d = new Date(ms);
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, "0");
  const day = String(d.getUTCDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export type DayBucket = { date: string; events: MaterializedEvent[] };

export function groupByDay(events: MaterializedEvent[]): DayBucket[] {
  const buckets = new Map<string, MaterializedEvent[]>();
  for (const e of events) {
    const ms = Number(e.event_at);
    if (!Number.isFinite(ms)) continue;
    const date = utcDateString(ms);
    const list = buckets.get(date) ?? [];
    list.push(e);
    buckets.set(date, list);
  }
  return [...buckets.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([date, events]) => ({
      date,
      events: [...events].sort((a, b) => {
        const ea = Number(a.event_at);
        const eb = Number(b.event_at);
        if (ea !== eb) return ea - eb;
        return a.event_id.localeCompare(b.event_id);
      }),
    }));
}

export type WeeklyAggregate = {
  window_days: number;
  window_start_ms: number;
  window_end_ms: number;
  total_events: number;
  per_kind: Record<string, number>;
};

export function weeklyAggregate(
  events: MaterializedEvent[],
  nowMs: number,
  windowDays = 7,
): WeeklyAggregate {
  const windowStart = nowMs - windowDays * 86_400_000;
  const perKind: Record<string, number> = {};
  let total = 0;
  for (const e of events) {
    const at = Number(e.event_at);
    if (!Number.isFinite(at)) continue;
    if (at < windowStart || at > nowMs) continue;
    perKind[e.event_kind] = (perKind[e.event_kind] ?? 0) + 1;
    total += 1;
  }
  return {
    window_days: windowDays,
    window_start_ms: windowStart,
    window_end_ms: nowMs,
    total_events: total,
    per_kind: perKind,
  };
}
