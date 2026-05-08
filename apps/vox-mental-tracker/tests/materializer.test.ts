import { describe, expect, it } from "vitest";
import {
  resolveCorrections,
  groupByDay,
  weeklyAggregate,
  type MaterializedEvent,
} from "../src/ts/materializer";
import type { HealthEventRow } from "../src/ts/export_contract";

function row(partial: Partial<HealthEventRow> & Pick<HealthEventRow, "event_id" | "event_kind">): HealthEventRow {
  return {
    schema_version: 1,
    payload_json: "{}",
    event_at: "1000",
    recorded_at: "1000",
    recorded_at_monotonic: 1000,
    tz_iana: "UTC",
    tz_offset_minutes: 0,
    source: "test",
    raw_transcript_id: "",
    correction_of: "",
    ...partial,
  };
}

describe("resolveCorrections", () => {
  it("returns originals untouched when no corrections present", () => {
    const rows = [
      row({ event_id: "a", event_kind: "mood_recorded" }),
      row({ event_id: "b", event_kind: "meal_recorded" }),
    ];
    const out = resolveCorrections(rows);
    expect(out.map((e) => e.event_id).sort()).toEqual(["a", "b"]);
  });

  it("collapses a single correction (A → A')", () => {
    const rows = [
      row({ event_id: "a", event_kind: "mood_recorded", payload_json: "{\"mood_score\":2}", recorded_at_monotonic: 100 }),
      row({ event_id: "a-prime", event_kind: "mood_recorded", payload_json: "{\"mood_score\":4}", correction_of: "a", recorded_at_monotonic: 200 }),
    ];
    const out = resolveCorrections(rows);
    expect(out).toHaveLength(1);
    expect(out[0].event_id).toBe("a-prime");
    expect(out[0].effective_event_id).toBe("a");
    expect(out[0].payload_json).toBe("{\"mood_score\":4}");
  });

  it("collapses a chain A → A' → A''", () => {
    const rows = [
      row({ event_id: "a", event_kind: "mood_recorded", recorded_at_monotonic: 100 }),
      row({ event_id: "a2", event_kind: "mood_recorded", correction_of: "a", recorded_at_monotonic: 200 }),
      row({ event_id: "a3", event_kind: "mood_recorded", correction_of: "a2", recorded_at_monotonic: 300 }),
    ];
    const out = resolveCorrections(rows);
    expect(out).toHaveLength(1);
    expect(out[0].event_id).toBe("a3");
    expect(out[0].effective_event_id).toBe("a");
  });

  it("is order-independent (deterministic regardless of input ordering)", () => {
    const a = row({ event_id: "a", event_kind: "mood_recorded", recorded_at_monotonic: 100 });
    const a2 = row({ event_id: "a2", event_kind: "mood_recorded", correction_of: "a", recorded_at_monotonic: 200 });
    const b = row({ event_id: "b", event_kind: "meal_recorded", recorded_at_monotonic: 150 });
    const orderings = [
      [a, a2, b],
      [a2, a, b],
      [b, a2, a],
      [a2, b, a],
    ];
    const serialized = orderings.map((rows) =>
      JSON.stringify(resolveCorrections(rows).map((e) => [e.effective_event_id, e.event_id])),
    );
    expect(new Set(serialized).size).toBe(1);
  });

  it("ignores corrections that reference an unknown prior event", () => {
    const rows = [
      row({ event_id: "orphan", event_kind: "mood_recorded", correction_of: "ghost" }),
    ];
    const out = resolveCorrections(rows);
    expect(out).toHaveLength(1);
    expect(out[0].event_id).toBe("orphan");
    expect(out[0].effective_event_id).toBe("orphan");
  });

  it("flags is_backdated on the effective row using its own clocks", () => {
    const rows = [
      row({ event_id: "a", event_kind: "mood_recorded", event_at: "0", recorded_at: "10000000" }),
    ];
    const out = resolveCorrections(rows);
    expect(out[0].is_backdated).toBe(true);
  });
});

describe("groupByDay", () => {
  it("groups events into UTC date buckets in sorted order", () => {
    const rows: MaterializedEvent[] = resolveCorrections([
      row({ event_id: "a", event_kind: "mood_recorded", event_at: String(Date.UTC(2026, 4, 1, 10)) }),
      row({ event_id: "b", event_kind: "meal_recorded", event_at: String(Date.UTC(2026, 4, 1, 15)) }),
      row({ event_id: "c", event_kind: "sleep_started", event_at: String(Date.UTC(2026, 4, 2, 3)) }),
    ]);
    const grouped = groupByDay(rows);
    expect(grouped.map((g) => g.date)).toEqual(["2026-05-01", "2026-05-02"]);
    expect(grouped[0].events.map((e) => e.event_id)).toEqual(["a", "b"]);
    expect(grouped[1].events.map((e) => e.event_id)).toEqual(["c"]);
  });

  it("is deterministic regardless of input row order", () => {
    const r1 = row({ event_id: "a", event_kind: "mood_recorded", event_at: String(Date.UTC(2026, 4, 1, 10)) });
    const r2 = row({ event_id: "b", event_kind: "meal_recorded", event_at: String(Date.UTC(2026, 4, 1, 15)) });
    const m1 = resolveCorrections([r1, r2]);
    const m2 = resolveCorrections([r2, r1]);
    expect(JSON.stringify(groupByDay(m1))).toBe(JSON.stringify(groupByDay(m2)));
  });
});

describe("weeklyAggregate", () => {
  const dayMs = 86_400_000;
  const now = Date.UTC(2026, 4, 8, 12);

  it("counts per kind within the 7-day window", () => {
    const rows: MaterializedEvent[] = resolveCorrections([
      row({ event_id: "a", event_kind: "mood_recorded", event_at: String(now - 1 * dayMs) }),
      row({ event_id: "b", event_kind: "mood_recorded", event_at: String(now - 2 * dayMs) }),
      row({ event_id: "c", event_kind: "meal_recorded", event_at: String(now - 3 * dayMs) }),
      row({ event_id: "d", event_kind: "exercise_recorded", event_at: String(now - 8 * dayMs) }),
    ]);
    const agg = weeklyAggregate(rows, now);
    expect(agg.window_days).toBe(7);
    expect(agg.total_events).toBe(3);
    expect(agg.per_kind.mood_recorded).toBe(2);
    expect(agg.per_kind.meal_recorded).toBe(1);
    expect(agg.per_kind.exercise_recorded).toBeUndefined();
  });

  it("excludes corrected-away events (uses effective set)", () => {
    const rows = resolveCorrections([
      row({ event_id: "a", event_kind: "mood_recorded", event_at: String(now - 1 * dayMs), recorded_at_monotonic: 100 }),
      row({ event_id: "a2", event_kind: "sleep_started", event_at: String(now - 1 * dayMs), correction_of: "a", recorded_at_monotonic: 200 }),
    ]);
    const agg = weeklyAggregate(rows, now);
    expect(agg.total_events).toBe(1);
    expect(agg.per_kind.sleep_started).toBe(1);
    expect(agg.per_kind.mood_recorded).toBeUndefined();
  });

  it("respects custom window_days", () => {
    const rows = resolveCorrections([
      row({ event_id: "a", event_kind: "mood_recorded", event_at: String(now - 5 * dayMs) }),
      row({ event_id: "b", event_kind: "mood_recorded", event_at: String(now - 10 * dayMs) }),
    ]);
    expect(weeklyAggregate(rows, now, 30).total_events).toBe(2);
    expect(weeklyAggregate(rows, now, 7).total_events).toBe(1);
  });
});
