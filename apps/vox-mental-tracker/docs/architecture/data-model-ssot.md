# Data model — append-only SSOT

## Tables

- **`HealthEventLog`** — authoritative append-only facts (`event_id`, `event_kind`, `payload_json`, clocks, provenance).
- **`RawTranscript`** — optional capture rows for voice (`transcript_id`, `transcript_text`, `parser_decisions_json`, `confidence`).

## Writes

All inserts go through **`record_health_event`** (`src/main.vox`). Corrections/tombstones are additional rows referencing **`correction_of`** (expand in next milestone).

## Clocks

- **`event_at`** — intended instant (defaults to client-supplied wall representation in this scaffold).
- **`recorded_at`** / **`recorded_at_monotonic`** — captured server-side via **`std.time.now_ms()`** for ordering.

## Materialization (derived state)

The append-only log is the only authoritative store. **Derived state — corrections collapsed, daily timeline buckets, weekly per-kind aggregates — lives in `src/ts/materializer.ts`** as pure functions over the row set. Same input rows produce the same output regardless of insertion order; covered by `tests/materializer.test.ts`.

- **Correction chains:** a row with non-empty `correction_of` supersedes the row it points to. Chains (`A → A' → A''`) collapse to the latest row, with `effective_event_id` carrying the chain root so consumers can group corrections back to their original event.
- **`is_backdated`:** computed via `export_contract.isBackdated` (recorded_at − event_at > 5 min) and propagated onto materialized events.
- **Window aggregation:** `weeklyAggregate(events, nowMs, windowDays = 7)` returns total + per-kind counts.

The Vox compiler **does** support row iteration and typed field access (probed against `db.HealthEventLog.all()`), so the Vox endpoints can do partial materialization themselves:
- `weekly_summary_json` returns total + per-kind counts (filters out rows with non-empty `correction_of`; full chain collapse is a TS-side concern).
- `timeline_events_json` returns a JSON array shaped for `materializer.resolveCorrections` — consumers (export pipelines, future React surfaces) feed it into the TS materializer for chain collapse, day grouping, and is_backdated derivation.

The TS materializer remains the SSOT for any consumer needing full correction-chain semantics or daily/weekly bucketing with deterministic ordering.

## Views / exports

CSV/JSON/HTML exports are specified under **`contracts/export/`** (incremental). The full clinician export pipeline lives in **`src/ts/export_pipeline.ts`** and composes:

1. raw rows (from `db.HealthEventLog.all()` → `timeline_events_json`)
2. → `resolveCorrections` (collapses correction chains)
3. → `buildHealthCsv` (deterministic CSV row projection)
4. → `sha256Hex(csv)` (WebCrypto content hash)
5. → JSON bundle + clinical HTML (with weekly aggregate table + daily timeline)

`buildExportBundle(rows, generatedMs)` returns `{csv, json, html, content_sha256, row_count_raw, row_count_effective}` — fully deterministic for the same inputs. Vox endpoint wiring (so `ExportPage` can drive the pipeline) is tracked in the Phase 4 plan; until that lands, consumers (tests, future React integration, CLI export commands) call the pipeline directly.
