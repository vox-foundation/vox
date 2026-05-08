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

Wiring the Vox UI to call the materializer is **blocked on compiler row-projection support** (the `weekly_summary_json` endpoint counts total rows only until that lands). Until then, the materializer is the authoritative reference for any export pipeline running outside Vox.

## Views / exports

CSV/PDF exports are specified under **`contracts/export/`** (incremental). The CSV row projection in `src/ts/export_contract.ts` consumes materialized events when writing rows.
