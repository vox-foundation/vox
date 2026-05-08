import { describe, expect, it } from "vitest";
import { buildExportBundle } from "../src/ts/export_pipeline";
import type { HealthEventRow } from "../src/ts/export_contract";

function row(p: Partial<HealthEventRow> & Pick<HealthEventRow, "event_id" | "event_kind">): HealthEventRow {
  return {
    schema_version: 1,
    payload_json: "{}",
    event_at: "1746576000000",
    recorded_at: "1746576000000",
    recorded_at_monotonic: 1746576000000,
    tz_iana: "UTC",
    tz_offset_minutes: 0,
    source: "test",
    raw_transcript_id: "",
    correction_of: "",
    ...p,
  };
}

const generatedMs = 1746662400000;

describe("export_pipeline.buildExportBundle", () => {
  it("produces a deterministic bundle for the same inputs", async () => {
    const rows: HealthEventRow[] = [
      row({ event_id: "a", event_kind: "mood_recorded", payload_json: '{"mood_score":3}' }),
      row({ event_id: "b", event_kind: "meal_recorded", payload_json: '{"description":"toast"}', event_at: "1746576100000" }),
    ];
    const a = await buildExportBundle(rows, generatedMs);
    const b = await buildExportBundle(rows, generatedMs);
    expect(a.csv).toBe(b.csv);
    expect(a.content_sha256).toBe(b.content_sha256);
    expect(a.html).toBe(b.html);
  });

  it("is order-independent on the input row list (after materialization)", async () => {
    const r1 = row({ event_id: "a", event_kind: "mood_recorded", payload_json: '{"mood_score":3}' });
    const r2 = row({ event_id: "b", event_kind: "meal_recorded", event_at: "1746576100000" });
    const a = await buildExportBundle([r1, r2], generatedMs);
    const b = await buildExportBundle([r2, r1], generatedMs);
    expect(a.csv).toBe(b.csv);
    expect(a.content_sha256).toBe(b.content_sha256);
  });

  it("changes the hash when the row set changes", async () => {
    const base = await buildExportBundle(
      [row({ event_id: "a", event_kind: "mood_recorded" })],
      generatedMs,
    );
    const more = await buildExportBundle(
      [
        row({ event_id: "a", event_kind: "mood_recorded" }),
        row({ event_id: "b", event_kind: "meal_recorded" }),
      ],
      generatedMs,
    );
    expect(base.content_sha256).not.toBe(more.content_sha256);
  });

  it("collapses correction chains in the materialized count", async () => {
    const bundle = await buildExportBundle(
      [
        row({ event_id: "a", event_kind: "mood_recorded", recorded_at_monotonic: 100 }),
        row({ event_id: "a2", event_kind: "mood_recorded", correction_of: "a", recorded_at_monotonic: 200 }),
      ],
      generatedMs,
    );
    expect(bundle.row_count_raw).toBe(2);
    expect(bundle.row_count_effective).toBe(1);
  });

  it("emits HTML with title, hash, weekly section, and daily section", async () => {
    const bundle = await buildExportBundle(
      [row({ event_id: "a", event_kind: "mood_recorded" })],
      generatedMs,
    );
    expect(bundle.html).toContain("<title>Mental Health Tracker — clinical export</title>");
    expect(bundle.html).toContain(bundle.content_sha256);
    expect(bundle.html).toContain("Weekly summary");
    expect(bundle.html).toContain("Daily timeline");
  });

  it("includes content_sha256 in the JSON bundle", async () => {
    const bundle = await buildExportBundle(
      [row({ event_id: "a", event_kind: "mood_recorded" })],
      generatedMs,
    );
    expect(bundle.json.content_sha256).toBe(bundle.content_sha256);
    expect(bundle.json.row_count).toBe(1);
  });

  it("renders an empty-state HTML when no rows present", async () => {
    const bundle = await buildExportBundle([], generatedMs);
    expect(bundle.row_count_raw).toBe(0);
    expect(bundle.html).toContain("No events recorded.");
  });
});
