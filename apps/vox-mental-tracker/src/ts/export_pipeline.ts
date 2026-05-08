/**
 * Export pipeline — turns raw HealthEventLog rows into a clinician-shareable
 * bundle (deterministic CSV, JSON bundle with content hash, HTML summary).
 *
 * Composes:
 *   resolveCorrections (materializer) → buildHealthCsv (export_contract)
 *   → sha256Hex (export_contract) → buildJsonBundle, renderClinicalHtml.
 *
 * All inputs are the raw event rows (e.g., from the timeline_events_json
 * Vox endpoint). Outputs are pure functions of those rows + a generation
 * timestamp; same inputs always produce the same artifacts.
 */

import {
  buildHealthCsv,
  buildJsonBundle,
  sha256Hex,
  type HealthEventRow,
} from "./export_contract";
import {
  groupByDay,
  resolveCorrections,
  weeklyAggregate,
  type DayBucket,
  type MaterializedEvent,
  type WeeklyAggregate,
} from "./materializer";

export type ExportBundle = {
  generated_ms: number;
  content_sha256: string;
  row_count_raw: number;
  row_count_effective: number;
  csv: string;
  json: ReturnType<typeof buildJsonBundle> & { content_sha256: string };
  html: string;
};

export async function buildExportBundle(
  rows: HealthEventRow[],
  generatedMs: number,
): Promise<ExportBundle> {
  const materialized = resolveCorrections(rows);
  const csv = buildHealthCsv(materialized);
  const contentSha256 = await sha256Hex(csv);
  const weekly = weeklyAggregate(materialized, generatedMs, 7);
  const daily = groupByDay(materialized);
  const json = {
    ...buildJsonBundle(materialized, generatedMs),
    content_sha256: contentSha256,
  };
  const html = renderClinicalHtml({
    materialized,
    weekly,
    daily,
    generatedMs,
    contentSha256,
    rowCountRaw: rows.length,
  });
  return {
    generated_ms: generatedMs,
    content_sha256: contentSha256,
    row_count_raw: rows.length,
    row_count_effective: materialized.length,
    csv,
    json,
    html,
  };
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderWeeklyTable(weekly: WeeklyAggregate): string {
  const rows = Object.entries(weekly.per_kind)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(
      ([kind, count]) =>
        `<tr><th scope="row">${escapeHtml(kind)}</th><td>${count}</td></tr>`,
    )
    .join("");
  return `<table class="mh-weekly"><thead><tr><th>kind</th><th>count</th></tr></thead><tbody>${rows}<tr><th scope="row">total</th><td>${weekly.total_events}</td></tr></tbody></table>`;
}

function renderDailyTimeline(daily: DayBucket[]): string {
  if (daily.length === 0) return `<p class="mh-empty">No events recorded.</p>`;
  return daily
    .map((bucket) => {
      const items = bucket.events
        .map((e) => {
          const ts = new Date(Number(e.event_at)).toISOString();
          const flag = e.is_backdated ? ` <span class="mh-backdated">(backdated)</span>` : "";
          return `<li><time datetime="${ts}">${ts}</time> · <code>${escapeHtml(e.event_kind)}</code> · ${escapeHtml(e.payload_json)}${flag}</li>`;
        })
        .join("");
      return `<section class="mh-day"><h3>${bucket.date}</h3><ul>${items}</ul></section>`;
    })
    .join("");
}

export function renderClinicalHtml(args: {
  materialized: MaterializedEvent[];
  weekly: WeeklyAggregate;
  daily: DayBucket[];
  generatedMs: number;
  contentSha256: string;
  rowCountRaw: number;
}): string {
  const generated = new Date(args.generatedMs).toISOString();
  const windowStart = new Date(args.weekly.window_start_ms).toISOString();
  const windowEnd = new Date(args.weekly.window_end_ms).toISOString();
  return [
    `<!doctype html>`,
    `<html lang="en"><head><meta charset="utf-8"/>`,
    `<title>Mental Health Tracker — clinical export</title>`,
    `<style>body{font-family:system-ui;margin:24px;max-width:780px}code{font-family:ui-monospace,monospace}.mh-meta{color:#555;font-size:90%}.mh-backdated{color:#a55;font-size:85%}table{border-collapse:collapse;margin:8px 0}th,td{border:1px solid #ccc;padding:4px 10px;text-align:left}.mh-day h3{margin-top:24px;border-bottom:1px solid #eee;padding-bottom:4px}</style>`,
    `</head><body>`,
    `<h1>Clinical export</h1>`,
    `<p class="mh-meta">Generated ${escapeHtml(generated)} · raw rows ${args.rowCountRaw} · effective rows ${args.materialized.length}</p>`,
    `<p class="mh-meta">Content SHA-256: <code>${args.contentSha256}</code></p>`,
    `<h2>Weekly summary (${escapeHtml(windowStart)} – ${escapeHtml(windowEnd)})</h2>`,
    renderWeeklyTable(args.weekly),
    `<h2>Methodology</h2>`,
    `<p>Append-only event log. Rows whose <code>correction_of</code> field references a prior row supersede that row; chains collapse to the latest entry. <code>is_backdated</code> is set when <code>recorded_at − event_at &gt; 5 min</code>. Daily and weekly views are deterministic functions of the raw row set.</p>`,
    `<h2>Daily timeline</h2>`,
    renderDailyTimeline(args.daily),
    `</body></html>`,
  ].join("");
}
