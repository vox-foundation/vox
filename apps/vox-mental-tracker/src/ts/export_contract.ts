/**
 * Deterministic CSV / JSON export aligned with contracts/export/csv-columns.v1.yaml
 * (full row materialization lives client-side until typed DB row projection is exposed in Vox check).
 */

export type HealthEventRow = {
  event_id: string;
  event_kind: string;
  schema_version: number;
  payload_json: string;
  event_at: string;
  recorded_at: string;
  recorded_at_monotonic: number;
  tz_iana: string;
  tz_offset_minutes: number;
  source: string;
  raw_transcript_id: string;
  correction_of: string;
};

const FIVE_MIN_MS = 300_000;

export function isBackdated(eventAtMs: string, recordedAtMs: string): boolean {
  const ea = Number(eventAtMs);
  const ra = Number(recordedAtMs);
  if (!Number.isFinite(ea) || !Number.isFinite(ra)) return false;
  return ra - ea > FIVE_MIN_MS;
}

function csvQuote(value: string): string {
  const inner = value.replace(/"/g, '""');
  return `"${inner}"`;
}

/** Stable sort: event_at asc, then event_id lexicographic */
export function sortEventsStable(rows: HealthEventRow[]): HealthEventRow[] {
  return [...rows].sort((a, b) => {
    const ea = Number(a.event_at);
    const eb = Number(b.event_at);
    if (ea !== eb) return ea - eb;
    return a.event_id.localeCompare(b.event_id);
  });
}

export function buildHealthCsv(rows: HealthEventRow[]): string {
  const header =
    "event_id,event_kind,schema_version,payload_json,event_at,recorded_at,recorded_at_monotonic,tz_iana,tz_offset_minutes,source,raw_transcript_id,correction_of,is_backdated";
  const sorted = sortEventsStable(rows);
  const lines = sorted.map((e) => {
    const back = isBackdated(e.event_at, e.recorded_at);
    return [
      csvQuote(e.event_id),
      csvQuote(e.event_kind),
      String(e.schema_version),
      csvQuote(e.payload_json),
      csvQuote(e.event_at),
      csvQuote(e.recorded_at),
      String(e.recorded_at_monotonic),
      csvQuote(e.tz_iana),
      String(e.tz_offset_minutes),
      csvQuote(e.source),
      csvQuote(e.raw_transcript_id),
      csvQuote(e.correction_of),
      csvQuote(String(back)),
    ].join(",");
  });
  return [header, ...lines].join("\n");
}

export async function sha256Hex(text: string): Promise<string> {
  const enc = new TextEncoder().encode(text);
  // eslint-disable-next-line no-undef
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) {
    throw new Error("WebCrypto subtle unavailable — run in browser or Node 20+ with global crypto");
  }
  const buf = await subtle.digest("SHA-256", enc);
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function buildJsonBundle(rows: HealthEventRow[], generatedMs: number): Record<string, unknown> {
  const csv = buildHealthCsv(rows);
  return {
    schema: "vox.mental_tracker.export_bundle",
    version: 1,
    generated_ms: generatedMs,
    row_count: rows.length,
    note: "content_sha256 should be computed from buildHealthCsv output (async sha256Hex)",
    rows,
  };
}
