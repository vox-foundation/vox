# Phase 89: Telemetry Surface Audit Remediation, Daemon/GUI Sinks & Public Repo Cleanup - Context

**Gathered:** 2026-10-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Close all 5 client-side telemetry gaps discovered during the Milestone `v1.7` audit:
1. Initialize the global `vox_telemetry` recorder in `vox-orchestrator-d` and `vox-gui` (`TELEM-01`).
2. Make `crates/vox-db/src/telemetry_sink.rs` (`ResearchMetricsSink`) exhaustive across all 21 existing `TelemetryEvent` variants (removing `_ => {}`) and add the 3 new privacy-safe community intelligence events (`ModelBenchCase`, `TrainingThroughput`, `DomainMisguidance`) to `vox-telemetry`, `vox-telemetry-otlp`, and `contracts/telemetry/collection-taxonomy.v1.json` (`TELEM-02`).
3. Implement real batched OTLP/HTTP upload (`<= 200` records or `256 KiB` per `POST /v1/logs`) in `crates/vox-telemetry-otlp/src/upload.rs` (`upload_pending_otlp`) and unify `crates/vox-cli/src/telemetry_spool.rs` (`upload_pending`) with `Clavis` secret resolution (`TELEM-03`).
4. Add `vox telemetry preview [--limit <N>] [--format json|table]` to inspect the exact sanitized OTLP JSON payload offline without network I/O, and verify kill-switch precedence in `vox telemetry doctor` (`TELEM-04`).
5. Copy `server/telemetry/` and `contracts/telemetry/collection-taxonomy.v1.json` into `~/dev/vox-ops` before removing `server/telemetry/` from public `vox` and updating `telemetry-trust-ssot.md` + `ADR-023` (`TELEM-05`).

</domain>

<decisions>
## Implementation Decisions

### 1. Daemon & GUI Recorder Wiring (`TELEM-01`)
- **D-01:** Wire `vox_telemetry::set_global_recorder(...)` in `vox-orchestrator-d` and `vox-gui` with **zero new crate dependencies or crate-edge changes** via `vox_db::telemetry_sink::init_process_telemetry(db)`:
  - `vox-orchestrator-d` (`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` / `daemon_steps.rs`) owns the live `VoxDb` handle and registers `ResearchMetricsSink::new(db)` + `StderrDebugSink` (when `VOX_TELEMETRY=debug`) inside `CompositeRecorder` whenever `TelemetryConfig::from_env()` enables sinks (`is_master_enabled() && cfg.enabled`).
  - `vox-gui` (`crates/vox-gui/src/main.rs`) registers a lightweight recorder that respects `CORE-04` (never opening a competing local `VoxDb` connection from the GUI process) by forwarding telemetry events over the existing `PersistentDaemon` RPC / `StderrDebugSink`.

### 2. Exhaustive `ResearchMetricsSink` & New Community Intelligence Telemetry Variants (`TELEM-02`)
- **D-02:** Replace the catch-all `_ => {}` arm in `crates/vox-db/src/telemetry_sink.rs` (`ResearchMetricsSink::record`) with an **exhaustive `match event` over every `TelemetryEvent` variant** so the Rust compiler rejects any future unhandled variant at compile time.
- **D-03:** Add the 3 new privacy-safe `TelemetryEvent` variants required by the Separation-of-Concerns SSoT (`docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md` §4.2–4.3) in Phase 89 so Phase 90's PostgreSQL views have complete schema coverage from day one:
  - `TelemetryEvent::ModelBenchCase(ModelBenchCaseEvent)` (`vox.model_bench_case`: `provider`, `model_id_hash`, `resolved_model_id_hash`, `suite`, `task_category`, `passed`, `score_bp`, `ttft_ms_bucket`, `latency_ms_bucket`, `tokens_per_sec_bucket`, `cost_usd_micros`, `error_class`).
  - `TelemetryEvent::TrainingThroughput(TrainingThroughputEvent)` (`vox.training_throughput`: `gpu_name`, `seq_len_bucket`, `batch_size`, `ms_per_step_bucket`, `sample_count`).
  - `TelemetryEvent::DomainMisguidance(DomainMisguidanceEvent)` (`vox.domain_misguidance`: `culprit_domain`, `defect_class` — strictly excluding full URLs, queries, or code snippets).
  - Register all 3 in `contracts/telemetry/collection-taxonomy.v1.json`, `crates/vox-telemetry-otlp/src/project.rs`, `crates/vox-telemetry-otlp/src/redact.rs`, and `crates/vox-db/src/telemetry_sink.rs`.

### 3. Batched OTLP Uploader & `vox telemetry preview` (`TELEM-03`, `TELEM-04`)
- **D-04:** Implement `crates/vox-telemetry-otlp/src/upload.rs` (`upload_pending_otlp`) and update `crates/vox-cli/src/telemetry_spool.rs` (`upload_pending`) so pending OTLP records are uploaded in batches of up to **200 records or 256 KiB** per `POST /v1/logs` request (`ExportLogsServiceRequest` JSON envelope `resourceLogs[0].scopeLogs[0].logRecords[]`), rate-limited via `governor`, using `vox_secrets::resolve_secret(SecretId::VoxTelemetryUploadToken)` and `SecretId::VoxTelemetryUploadUrl`, acknowledging (`ack`) files only on HTTP `200`/`202`.
- **D-05:** Add `vox telemetry preview [--spool <PATH>] [--limit <N>] [--format json|table]` to `crates/vox-cli/src/commands/telemetry.rs` (`TelemetryCmd::Preview`) to render the exact sanitized, batched OTLP JSON payload (or a summary table of `event_name`, `category`, and attribute keys/values) to stdout with **zero network I/O** and **zero file deletion**.

### 4. Safe Extraction & Removal of `server/telemetry/` (`TELEM-05`)
- **D-06:** Initialize `~/dev/vox-ops` locally first and copy `server/telemetry/` + `contracts/telemetry/collection-taxonomy.v1.json` into `~/dev/vox-ops/legacy-clickhouse-reference/` and `~/dev/vox-ops/contracts/telemetry/` **before** removing `server/telemetry/` from public `vox` and updating `docs/src/architecture/telemetry-trust-ssot.md` and `docs/src/adr/023-optional-telemetry-remote-upload.md`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Privacy SSoTs
- `docs/src/architecture/centralized-postgres-telemetry-vox-ops-ssot-2026.md` — Canonical Milestone `v1.7` architecture, telemetry audit table, and Separation-of-Concerns matrix.
- `docs/src/architecture/telemetry-trust-ssot.md` — Telemetry trust boundaries, S0–S3 sensitivity classes, and kill-switch hierarchy.
- `docs/src/adr/023-optional-telemetry-remote-upload.md` — ADR-023 optional remote telemetry upload and local spool contract.
- `contracts/telemetry/collection-taxonomy.v1.json` — Machine-readable allowlist of telemetry event categories, event names, and attribute types.

### Client Telemetry & Sink Implementation Files
- `crates/vox-telemetry/src/lib.rs` — `TelemetryEvent` enum, `TelemetryRecorder` trait, `CompositeRecorder`, `StderrDebugSink`, and `set_global_recorder`.
- `crates/vox-telemetry/src/config.rs` — `TelemetryConfig::from_env()`, `ConsentState`, `is_master_enabled()`, and `is_remote_allowed()`.
- `crates/vox-telemetry-otlp/src/project.rs` — `project_event(&TelemetryEvent) -> Option<(String, BTreeMap<String, Value>)>`.
- `crates/vox-telemetry-otlp/src/redact.rs` — Taxonomy allowlist guard (`redact_event`).
- `crates/vox-telemetry-otlp/src/otlp_json.rs` — `to_otlp_log` OTLP/HTTP JSON formatter.
- `crates/vox-telemetry-otlp/src/upload.rs` — `upload_pending_otlp` async uploader.
- `crates/vox-db/src/telemetry_sink.rs` — `ResearchMetricsSink` writing `TelemetryEvent` variants into `VoxDb::append_research_metric`.
- `crates/vox-cli/src/lib.rs` — `init_telemetry_sinks_for` (lines 766–810).
- `crates/vox-cli/src/telemetry_sink.rs` — `SpoolSink`.
- `crates/vox-cli/src/telemetry_spool.rs` — Bounded pending JSON queue (`enqueue`, `prune`, `list_pending`, `upload_pending`).
- `crates/vox-cli/src/commands/telemetry.rs` — `vox telemetry {doctor,consent,status,export,enqueue,upload}` CLI surface.
- `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` & `crates/vox-orchestrator-d/src/daemon_steps.rs` — Daemon startup steps where `VoxDb` is connected.
- `crates/vox-gui/src/main.rs` — Tauri GUI startup entrypoint.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox_telemetry::CompositeRecorder` and `vox_telemetry::StderrDebugSink` in `crates/vox-telemetry/src/lib.rs` can be reused directly in `vox_db::telemetry_sink::init_process_telemetry` so `vox-orchestrator-d` and `vox-gui` need zero new crate dependencies.
- `vox_secrets::SecretId::VoxTelemetryUploadUrl` and `vox_secrets::SecretId::VoxTelemetryUploadToken` already exist in `crates/vox-secrets` and are already used in `crates/vox-cli/src/commands/telemetry.rs` (lines 163–164, 216–224).
- `crates/vox-cli/src/telemetry_spool.rs` already implements `DEFAULT_MAX_SPOOL_BYTES` (`10 MiB`), `DEFAULT_MAX_SPOOL_AGE` (`7 days`), and age/size self-pruning on `enqueue`.

### Established Patterns
- Zero new crate dependencies or crate-edge changes (`contracts/ci/crate-edges.allow.v1.json`) when existing dependency edges (`vox-orchestrator-d -> vox-db -> vox-telemetry`) already provide all required types.
- `CORE-04` (from Phase 79): `vox-gui` never opens a competing local `VoxDb` SQLite/Turso handle when `vox-orchestrator-d` owns `VoxDb`.

### Integration Points
- `crates/vox-orchestrator-d/src/daemon_steps.rs`: right after `VoxDb` is initialized in `DaemonBootCtx`, call `vox_db::telemetry_sink::init_process_telemetry`.
- `crates/vox-cli/src/commands/telemetry.rs`: add `TelemetryCmd::Preview` and update command catalog if required by `vox ci command-sync`.

</code_context>

<specifics>
## Specific Ideas

- `vox telemetry preview` must allow a developer to inspect both the exact OTLP JSON batch (`--format json`) and a concise human-readable table (`--format table`, default when stdout is a TTY) showing `event_name`, `category`, and redacted attributes so privacy transparency is tangible and immediate.

</specifics>

<deferred>
## Deferred Ideas

- `vox telemetry intelligence {status,sync,inspect}` CLI subcommands — scheduled for **Phase 92 (`INTEL-04`)** alongside `CommunityIntelligenceBundleV1` client hydration.

</deferred>

---

*Phase: 89-Telemetry Surface Audit Remediation, Daemon/GUI Sinks & Public Repo Cleanup*
*Context gathered: 2026-10-19*
