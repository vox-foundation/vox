---
phase: 89-telemetry-surface-audit-remediation-daemon-gui-sinks-public
plan: 01
subsystem: telemetry
tags: [telemetry, vox-db, daemon, gui, otlp, taxonomy]
requires: []
provides:
  - "All 26 TelemetryEvent variants mapped in vox-db::telemetry_sink::ResearchMetricsSink"
  - "Process-wide telemetry recorder wiring for vox-orchestrator-d (VoxDb) and vox-gui (StderrDebugSink)"
  - "Three flywheel telemetry categories (model_bench_case, training_throughput, domain_misguidance) in vox-telemetry, vox-telemetry-otlp, and collection-taxonomy.v1.json"
affects: [vox-telemetry, vox-telemetry-otlp, vox-db, vox-orchestrator-d, vox-gui]
tech-stack:
  added: []
  patterns: ["Process-global TelemetryRecorder initialization via init_process_telemetry"]
key-files:
  created: []
  modified:
    - contracts/telemetry/collection-taxonomy.v1.json
    - crates/vox-telemetry/src/types.rs
    - crates/vox-telemetry/src/lib.rs
    - crates/vox-telemetry/tests/taxonomy_ssot_parity.rs
    - crates/vox-telemetry-otlp/src/project.rs
    - crates/vox-telemetry-otlp/tests/projection_coverage.rs
    - crates/vox-db/src/telemetry_sink.rs
    - crates/vox-orchestrator-d/src/daemon_steps/run.rs
    - crates/vox-gui/src/main.rs
key-decisions:
  - "vox-gui initializes StderrDebugSink via init_process_telemetry(None) rather than opening a competing local VoxDb handle alongside vox-orchestrator-d."
  - "DomainMisguidanceEvent projects culprit_domain into a salted BLAKE3 hash (culprit_domain_hash) before OTLP spooling so raw domain strings never cross the remote telemetry boundary."
patterns-established:
  - "Exhaustive match without wildcard arm in map_telemetry_event_to_row guarantees compile-time failure whenever a new TelemetryEvent variant is added."
requirements-completed: [SINK-01, SINK-02, SINK-03, FLYWHEEL-01, FLYWHEEL-02, FLYWHEEL-03]
duration: 15min
completed: 2026-10-19
---

# Phase 89 Plan 01: Telemetry Surface Audit Remediation & Flywheel Categories Summary

**Wired all 26 `TelemetryEvent` variants into `ResearchMetricsSink`, connected process-global telemetry recorders in `vox-orchestrator-d` and `vox-gui`, and added the 3 flywheel telemetry categories (`model_bench_case`, `training_throughput`, `domain_misguidance`).**

## Accomplishments
- Added `METRIC_TYPE_*` constants and strongly-typed event structs for `ModelBenchCaseEvent`, `TrainingThroughputEvent`, and `DomainMisguidanceEvent` in `crates/vox-telemetry/src/types.rs`.
- Extended `contracts/telemetry/collection-taxonomy.v1.json` with `model_bench_case`, `training_throughput`, and `domain_misguidance` (`enum|int|bool|hash` scalar fields only) and verified parity via `taxonomy_ssot_parity`.
- Added OTLP pre-spool projection arms in `crates/vox-telemetry-otlp/src/project.rs` with salted BLAKE3 hashing for `fixture_id` and `culprit_domain`, verified by canary leak tests in `projection_coverage.rs`.
- Replaced the 8-variant partial match in `crates/vox-db/src/telemetry_sink.rs` with an exhaustive 26-variant `map_telemetry_event_to_row` implementation and wired `init_process_telemetry` into `vox-orchestrator-d` (`step_db`) and `vox-gui` (`main`).

## Verification
- `cargo test -p vox-telemetry -- --test-threads=1` — passed (`110` unit tests + `taxonomy_ssot_parity`).
- `cargo test -p vox-telemetry-otlp --test projection_coverage` — passed (`16` passed, `0` failed).
- `cargo test -p vox-db --lib telemetry_sink` — passed (`all_26_telemetry_event_variants_persist_and_pass_db_shape_audit`).
