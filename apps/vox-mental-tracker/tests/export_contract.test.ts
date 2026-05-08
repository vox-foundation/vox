import { describe, expect, it } from "vitest";
import {
  buildHealthCsv,
  buildJsonBundle,
  isBackdated,
  sortEventsStable,
  type HealthEventRow,
} from "../src/ts/export_contract";

function row(partial: Partial<HealthEventRow> & Pick<HealthEventRow, "event_id" | "event_kind">): HealthEventRow {
  return {
    schema_version: 1,
    payload_json: "{}",
    event_at: "100",
    recorded_at: "200",
    recorded_at_monotonic: 200,
    tz_iana: "UTC",
    tz_offset_minutes: 0,
    source: "test",
    raw_transcript_id: "",
    correction_of: "",
    ...partial,
  };
}

describe("export_contract", () => {
  it("detects backdating beyond 5 minutes", () => {
    expect(isBackdated("0", "299999")).toBe(false);
    expect(isBackdated("0", "300001")).toBe(true);
  });

  it("sorts deterministically by event_at then event_id", () => {
    const r = sortEventsStable([
      row({ event_id: "b", event_at: "2" }),
      row({ event_id: "a", event_at: "10" }),
      row({ event_id: "c", event_at: "2" }),
    ]);
    expect(r.map((x) => x.event_id).join(",")).toBe("b,c,a");
  });

  it("builds CSV with stable header from fixtures", () => {
    const csv = buildHealthCsv([
      row({
        event_id: "e1",
        event_kind: "mood_recorded",
        payload_json: "{\"mood_score\":3}",
        event_at: "1000",
        recorded_at: "400000",
      }),
    ]);
    expect(csv.split("\n")[0]).toContain("is_backdated");
    expect(csv).toContain("mood_recorded");
    expect(csv).toContain("\"true\"");
  });

  it("buildJsonBundle carries metadata", () => {
    const j = buildJsonBundle([row({ event_id: "x", event_kind: "note_recorded" })], 99);
    expect(j.row_count).toBe(1);
    expect(j.generated_ms).toBe(99);
  });
});
