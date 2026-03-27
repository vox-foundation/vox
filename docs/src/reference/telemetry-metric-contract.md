# Telemetry & `research_metrics` contract

## Row shape

Table `research_metrics` columns: `session_id`, `metric_type`, `metric_value` (nullable `REAL`), `metadata_json`.

- **`metric_value`**: optional scalar. **SQL `NULL` means “no scalar”** — APIs must not coerce NULL to `0.0` (aggregations skip nulls; see `list_research_metrics_by_type`).
- **`metadata_json`**: structured payload; may include units and names that disambiguate mixed benchmarks.

## Metric types (non-exhaustive)

| `metric_type` | Session prefix | Scalar semantics | Notes |
|---------------|----------------|------------------|-------|
| `benchmark_event` | `bench:<repository_id>` | Optional; unit in metadata `metric_value_unit` | CLI build timings use **`seconds`** for wall time. |
| `syntax_k_event` | `syntaxk:<repository_id>` | Optional ratio / timing | Fixture id in metadata. |
| `socrates_surface` | `mcp:<repository_id>` | Hallucination-risk proxy | Prefer metadata for interpretability. |

### `benchmark_event` metadata (`BenchmarkEventMeta`)

- `name`: logical benchmark id (`cargo_build_metrics`, …).
- `metric_value_unit`: when `metric_value` is set, unit SSOT (`seconds`, `milliseconds`, `ratio`, …).
- `details`: free-form JSON (per-crate timings, pass/fail flags).

## Training JSONL (`telemetry.jsonl`)

Envelope per line: `{ "ts_ms", "event", "payload" }`. Payload keys are defined in `crates/vox-populi/src/mens/tensor/telemetry_schema.rs` (e.g. `eta_seconds_remaining`, `steps_per_sec_ema`). The CLI viewer `vox mens watch-telemetry` must track this schema (guarded by `vox ci data-ssot-guards`).

## CI

- `vox ci data-ssot-guards` — asserts watch-telemetry references schema keys and `research_metrics` list API avoids `COALESCE(metric_value, 0.0)`.
