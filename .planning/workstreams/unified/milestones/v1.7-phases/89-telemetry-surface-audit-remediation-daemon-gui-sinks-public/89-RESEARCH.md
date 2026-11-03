# Phase 89: Telemetry Surface Audit Remediation, Daemon/GUI Sinks & Public Repo Cleanup - Research

**Researched:** 2026-10-19
**Domain:** `vox-telemetry`, `vox-telemetry-otlp`, `vox-db::telemetry_sink`, `vox-orchestrator-d`, `vox-gui`, `vox-cli`
**Confidence:** HIGH

## Summary

Phase 89 closes all five telemetry audit findings (`TELEM-01` through `TELEM-05`) documented in [`docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md`](../../../../../docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md):
1. **Daemon & GUI Process Sink Wiring (`TELEM-01`, D-01):** Only `vox-cli` (`crates/vox-cli/src/lib.rs:748-810`) calls `vox_telemetry::set_global_recorder(...)`. Both `vox-orchestrator-d` and `vox-gui` leave the global recorder uninitialized (`OnceLock` = `None`), causing every `record_event!` in daemon/GUI processes to no-op. By adding `pub fn init_process_telemetry(db: Option<VoxDb>)` to `crates/vox-db/src/telemetry_sink.rs`, both `vox-orchestrator-d` (`crates/vox-orchestrator-d/src/daemon_steps/run.rs` in `step_db`) and `vox-gui` (`crates/vox-gui/src/main.rs`) can initialize the process recorder with zero new crate dependencies (`Cargo.toml` / `Cargo.lock` / `layers.toml` untouched).
2. **Exhaustive `TelemetryEvent` Coverage & 3 New Flywheel Variants (`TELEM-02`, D-02, D-03):** `crates/vox-db/src/telemetry_sink.rs:292` ends with `_ => {}` and silently drops 7 Track E/F variants (`CommandUsage`, `SkillActivation`, `EditPattern`, `HarnessUsage`, `ErrorSurface`, `DefaultDecision`, `ModelPrompt`). Removing `#[non_exhaustive]` from `TelemetryEvent` in `crates/vox-telemetry/src/types.rs` and replacing `_ => {}` / `_ => None` in `crates/vox-db/src/telemetry_sink.rs` and `crates/vox-telemetry-otlp/src/project.rs` makes variant handling compiler-enforced. Adding `ModelBenchCase(ModelBenchCaseEvent)`, `TrainingThroughput(TrainingThroughputEvent)`, and `DomainMisguidance(DomainMisguidanceEvent)` to `vox-telemetry`, `contracts/telemetry/collection-taxonomy.v1.json`, `vox-telemetry-otlp`, and `vox-db` completes the client-side schema for Phase 90–92 community intelligence tables.
3. **Batched OTLP Upload (`TELEM-03`, D-04):** `crates/vox-telemetry-otlp/src/upload.rs` is a `TODO(track-B5)` stub returning `Ok(0)`, while `crates/vox-cli/src/telemetry_spool.rs::upload_pending` posts one HTTP request per file with raw payload bodies. Implementing batched OTLP/HTTP JSON framing (`<= 200` records or `<= 256 KiB` per `POST /v1/logs` request in `ExportLogsServiceRequest` format `resourceLogs[0].scopeLogs[0].logRecords[]`) in `crates/vox-telemetry-otlp/src/otlp_json.rs` + `upload.rs` and wiring `telemetry_spool::upload_pending` to delegate to it eliminates per-file HTTP overhead and fixes wire-format compatibility.
4. **Transparent Spool Inspection (`TELEM-04`, D-05):** Adding `TelemetryCmd::Preview` (`vox telemetry preview [--spool <PATH>] [--limit <N>] [--format json|table]`) in `crates/vox-cli/src/commands/telemetry.rs` gives users full read-only visibility into the exact redacted OTLP payload batch queued on disk before any remote upload occurs.
5. **Public Repo Cleanup (`TELEM-05`, D-06):** Staging `server/telemetry/` and `contracts/telemetry/collection-taxonomy.v1.json` into `~/dev/vox-ops`, removing `server/telemetry/` and its `[workspace].exclude` entry from public `vox`, and updating `docs/src/architecture/telemetry-trust-ssot.md` + `docs/src/adr/023-optional-telemetry-remote-upload.md` cleanly separates private server infrastructure from the public compiler/runtime workspace.

## Architectural Responsibility Map

| Capability | Primary Tier | Owning Crate / File | Responsibility |
|------------|-------------|---------------------|----------------|
| Event definitions & metric constants | L1 Foundation | `crates/vox-telemetry/src/{types,lib}.rs` | `TelemetryEvent` enum (exhaustive across workspace), `METRIC_TYPE_*` constants, validation helpers |
| Taxonomy contract & privacy gate | Contract / Test | `contracts/telemetry/collection-taxonomy.v1.json`, `crates/vox-telemetry/tests/taxonomy_ssot_parity.rs` | Single source of truth for allowed categories and `enum\|int\|bool\|hash` scalar fields |
| Projection, redaction & OTLP batching | Egress Core | `crates/vox-telemetry-otlp/src/{project,redact,otlp_json,upload}.rs` | Map `TelemetryEvent` to taxonomy category, strip non-allowlisted fields, batch into OTLP `ExportLogsServiceRequest`, POST with `governor` rate-limit |
| Local `VoxDb` persistence & process init | Persistence | `crates/vox-db/src/telemetry_sink.rs` | `ResearchMetricsSink` (exhaustive `match event`) + `init_process_telemetry(db: Option<VoxDb>)` |
| Daemon & GUI startup wiring | Binaries | `crates/vox-orchestrator-d/src/daemon_steps/run.rs`, `crates/vox-gui/src/main.rs` | Register `TelemetryRecorder` on process startup without competing SQLite locks |
| CLI spool management & preview | CLI Surface | `crates/vox-cli/src/{telemetry_spool.rs,commands/telemetry.rs}` | `vox telemetry preview`, batched `vox telemetry upload` with Clavis secrets |

## Validation Architecture

- `timeout 900s cargo test -p vox-telemetry` — verifies round-trip serialization, `validate_research_metric_row`, and `taxonomy_ssot_parity` (`enum|int|bool|hash` invariant across all 14 categories including `model_bench_case`, `training_throughput`, and `domain_misguidance`).
- `timeout 900s cargo test -p vox-telemetry-otlp --all-features` — verifies `projection_coverage` (canary leak tests across all projected variants), `upload_gating`, OTLP batch envelope construction (`<= 200` records / `256 KiB`), and mock HTTP upload/ack lifecycle.
- `timeout 900s cargo test -p vox-db --lib telemetry_sink` — verifies `ResearchMetricsSink::record` persists every `TelemetryEvent` variant into `research_metrics` and `init_process_telemetry` is idempotent.
- `timeout 900s cargo test -p vox-orchestrator-d` — verifies daemon boot plan (`step_db`) initializes telemetry and passes all boot-step ordering tests.
- `timeout 900s cargo test -p vox-cli --lib telemetry` — verifies `vox telemetry preview` (`json` and `table` formats) and batched `upload_pending`.
- `./target/debug/vox-doc-pipeline --lint-only --paths architecture/telemetry-trust-ssot.md adr/023-optional-telemetry-remote-upload.md` — verifies documentation frontmatter and link integrity.
