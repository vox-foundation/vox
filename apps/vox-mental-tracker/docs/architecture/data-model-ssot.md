# Data model — append-only SSOT

## Tables

- **`HealthEventLog`** — authoritative append-only facts (`event_id`, `event_kind`, `payload_json`, clocks, provenance).
- **`RawTranscript`** — optional capture rows for voice (`transcript_id`, `transcript_text`, `parser_decisions_json`, `confidence`).

## Writes

All inserts go through **`record_health_event`** (`src/main.vox`). Corrections/tombstones are additional rows referencing **`correction_of`** (expand in next milestone).

## Clocks

- **`event_at`** — intended instant (defaults to client-supplied wall representation in this scaffold).
- **`recorded_at`** / **`recorded_at_monotonic`** — captured server-side via **`std.time.now_ms()`** for ordering.

## Views / exports

Materialized views and CSV/PDF exports are specified under **`contracts/export/`** (incremental).
