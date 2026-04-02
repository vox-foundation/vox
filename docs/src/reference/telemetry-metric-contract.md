---
title: "Telemetry and research_metrics contract"
description: "research_metrics row shape, write validation, session_id prefix conventions, key metric_type semantics, Mens telemetry.jsonl envelope and KPI tiers, deprecation notes, and related CI guards."
category: "reference"
---

# Telemetry & `research_metrics` contract

## Related SSOT

- [Telemetry trust boundary and SSOT map](../architecture/telemetry-trust-ssot.md)
- [Telemetry taxonomy and contracts SSOT](../architecture/telemetry-taxonomy-contracts-ssot.md) (roadmap)
- [Telemetry retention and sensitivity SSOT](../architecture/telemetry-retention-sensitivity-ssot.md) (roadmap)
- [Telemetry client disclosure SSOT](../architecture/telemetry-client-disclosure-ssot.md)
- [Telemetry implementation blueprint 2026](../architecture/telemetry-implementation-blueprint-2026.md) and [backlog](../architecture/telemetry-implementation-backlog-2026.md)
- Optional **explicit** remote upload (local JSON spool, not `research_metrics`): [ADR 023](../adr/023-optional-telemetry-remote-upload.md), [Telemetry remote sink specification](../architecture/telemetry-remote-sink-spec.md), CLI **`vox telemetry`**

Code enforcement for row validation: [`validate_research_metric_row`](../../../crates/vox-db/src/research_metrics_contract.rs) (called from `append_research_metric`). Repository-scoped producers should use [`TelemetryWriteOptions`](../../../crates/vox-db/src/research_metrics_contract.rs) plus the `METRIC_TYPE_*` / `SESSION_PREFIX_*` / `SESSION_ID_*` constants in [`vox_db::research_metrics_contract`](../../../crates/vox-db/src/research_metrics_contract.rs).

## Row shape

Table `research_metrics` columns: `session_id`, `metric_type`, `metric_value` (nullable `REAL`), `metadata_json`.

- **`metric_value`**: optional scalar. **SQL `NULL` means “no scalar”** — APIs must not coerce NULL to `0.0` (aggregations skip nulls; see `list_research_metrics_by_type`).
- **`metadata_json`**: structured payload; may include units and names that disambiguate mixed benchmarks.

## Validation limits (writes)

| Field | Rule |
|-------|------|
| `session_id` | Non-empty; max **512** UTF-8 characters. |
| `metric_type` | Non-empty; max **128** characters; characters must be ASCII alphanumeric or `_`, `.`, `-`, **`:`** (colon allows MCP-linked namespaces such as `foo:bar`). |
| `metadata_json` | Optional; if present, max **256 KiB** serialized length. |

## Session id namespaces (convention)

Producers should prefix `session_id` so rollups and dashboards can group without colliding:

| Prefix | Example | Typical producer |
|--------|---------|------------------|
| `bench:` | `bench:<repository_id>` | CLI / build timings |
| `syntaxk:` | `syntaxk:<repository_id>` | Syntax-K eval fixtures |
| `mcp:` | `mcp:<repository_id>` | MCP Socrates / surface telemetry |
| `mens:` | `mens:<repository_id>` | Populi control-plane audit (`populi_control_event`) |
| `workflow:` | `workflow:<repository_id>` | Interpreted workflow journal (`workflow_journal_entry`, versioned event payloads from the workflow durability contract) |

**Fixed session (no repository in id):** hybrid memory fusion uses session `socrates:retrieval` and metric type `memory_hybrid_fusion` (see `SESSION_ID_MEMORY_HYBRID_FUSION` in the Rust module).

**Questioning / linked metrics:** MCP may use opaque `session_key` strings for `questioning_event` and `vox_db_research_metric_linked` (not forced through [`TelemetryWriteOptions`](../../../crates/vox-db/src/research_metrics_contract.rs)); those rows still must satisfy validation caps above.

## Metric types (non-exhaustive)

| `metric_type` | Session prefix | Scalar semantics | Notes |
|---------------|----------------|------------------|-------|
| `benchmark_event` | `bench:<repository_id>` | Optional; unit in metadata `metric_value_unit` | CLI build timings use **`seconds`** for wall time. |
| `syntax_k_event` | `syntaxk:<repository_id>` | Optional ratio / timing | Fixture id in metadata; optional `support_metrics` (representability / LLM surface / runtime projection summaries per `contracts/eval/syntax-k-event.schema.json`). |
| `socrates_surface` | `mcp:<repository_id>` | Hallucination-risk proxy | Prefer metadata for interpretability; eval summaries inject explicit denominators (below). |

### `socrates_surface` aggregate metadata (`record_socrates_eval_summary`)

Rollups written to `eval_runs` include JSON with both raw counts and **explicit denominators** so downstream tools do not misread rates when some rows lack a scalar or parseable metadata:

- `rate_denominator`: literal `"parsed_metadata_rows"` — rates (`answer_rate`, `abstain_rate`) use this count.
- `abstain_rate_denominator_n` / `answer_rate_denominator_n`: same as `parsed_metadata_rows`.
- `mean_proxy_denominator_n`: `rows_with_metric_value` — mean hallucination-risk proxy uses only rows where `metric_value` was non-NULL.
- `rows_total_n`: `sample_size` — all `socrates_surface` rows scanned.

**Quality** in `eval_runs` uses the mean proxy **only** when `rows_with_metric_value > 0`; otherwise quality is **0.0** (avoids implying a perfect score with no scalar signal).

### `benchmark_event` metadata (`BenchmarkEventMeta`)

- `name`: logical benchmark id (`cargo_build_metrics`, …).
- `metric_value_unit`: when `metric_value` is set, unit SSOT (`seconds`, `milliseconds`, `ratio`, …).
- `details`: free-form JSON (per-crate timings, pass/fail flags).

For cross-repo querying via MCP, `benchmark_event` may use `name = "cross_repo_query"` with `metric_value_unit = "milliseconds"` and `details` such as:

- `query_kind`
- `trace_id`
- `correlation_id`
- `conversation_id`
- `workspace_repository_id`
- `target_repository_ids`
- `source_plane`
- `query_backend`
- `result_count`
- `skipped_count`

## Training JSONL (`telemetry.jsonl`)

Envelope per line: `{ "ts_ms", "event", "payload" }`. Payload keys are defined in `crates/vox-populi/src/mens/tensor/telemetry_schema.rs` (e.g. `eta_seconds_remaining`, `steps_per_sec_ema`). The CLI viewer `vox mens watch-telemetry` must track this schema (guarded by `vox ci data-ssot-guards`).

### Mens training KPI ownership (decision-driving)

- **Tier 1 (gate-driving)**:
  - `tokens_per_sec` (with `tokens_per_sec_is_proxy` when derived),
  - `valid_tokens`,
  - `theoretical_tokens`,
  - `supervised_ratio_pct`.
- **Tier 2 (diagnostic)**:
  - `steps_per_sec_ema`,
  - `eta_seconds_remaining`,
  - skip counters (`skip_no_supervised_positions`, `skip_short_seq`, ...).

### Deprecation / compatibility window

- Consumers should prefer canonical fields above.
- Legacy aliases are still read with warnings (status / eval-gate paths), then normalized at read time.
- `steps_per_sec_ema` as a throughput surrogate is considered deprecated for gates when `tokens_per_sec` is present.

## CI

- `vox ci data-ssot-guards` — asserts watch-telemetry references schema keys and `research_metrics` list API avoids `COALESCE(metric_value, 0.0)`.
- Web IR structural gate: workflow sets `VOX_WEBIR_VALIDATE=1` and runs `cargo test -p vox-compiler --test web_ir_lower_emit` (see `.github/workflows/ci.yml`).
