---
phase: 89
phase_name: "telemetry-surface-audit-remediation-daemon-gui-sinks-public"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 17
  lessons: 3
  patterns: 6
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 89 Learnings: telemetry-surface-audit-remediation-daemon-gui-sinks-public

## Decisions

### vox-gui initializes StderrDebugSink via init_process_telemetry(None) rat...
vox-gui initializes StderrDebugSink via init_process_telemetry(None) rather than opening a competing local VoxDb handle alongside vox-orchestrator-d.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-01-SUMMARY.md

---

### DomainMisguidanceEvent projects culprit_domain into a salted BLAKE3 hash...
DomainMisguidanceEvent projects culprit_domain into a salted BLAKE3 hash (culprit_domain_hash) before OTLP spooling so raw domain strings never cross the remote telemetry boundary.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-01-SUMMARY.md

---

### Added METRIC_TYPE_* constants and strongly-typed event structs for Model...
Added `METRIC_TYPE_*` constants and strongly-typed event structs for `ModelBenchCaseEvent`, `TrainingThroughputEvent`, and `DomainMisguidanceEvent` in `crates/vox-telemetry/src/types.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### Extended contracts/telemetry/collection-taxonomy.v1.json with model_benc...
Extended `contracts/telemetry/collection-taxonomy.v1.json` with `model_bench_case`, `training_throughput`, and `domain_misguidance` (`enum|int|bool|hash` scalar fields only) and verified parity via `taxonomy_ssot_parity`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### Added OTLP pre-spool projection arms in crates/vox-telemetry-otlp/src/pr...
Added OTLP pre-spool projection arms in `crates/vox-telemetry-otlp/src/project.rs` with salted BLAKE3 hashing for `fixture_id` and `culprit_domain`, verified by canary leak tests in `projection_coverage.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### Replaced the 8-variant partial match in crates/vox-db/src/telemetry_sink...
Replaced the 8-variant partial match in `crates/vox-db/src/telemetry_sink.rs` with an exhaustive 26-variant `map_telemetry_event_to_row` implementation and wired `init_process_telemetry` into `vox-orchestrator-d` (`step_db`) and `vox-gui` (`main`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-01-SUMMARY.md

---

### Merged pending spool files into standard OTLP/HTTP ExportLogsServiceRequ...
Merged pending spool files into standard OTLP/HTTP ExportLogsServiceRequest batches (up to 200 logRecords or 256 KiB per POST) and delete queued files only after receiving HTTP 2xx.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-02-SUMMARY.md

---

### Staged server/telemetry/ into /Users/brbrainerd/dev/vox-ops/legacy-click...
Staged server/telemetry/ into /Users/brbrainerd/dev/vox-ops/legacy-clickhouse-reference/ and removed it from the public vox workspace.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-02-SUMMARY.md

---

### Added OTLP_BATCH_MAX_RECORDS (200), OTLP_BATCH_MAX_BYTES (256 * 1024), a...
Added `OTLP_BATCH_MAX_RECORDS` (`200`), `OTLP_BATCH_MAX_BYTES` (`256 * 1024`), and `merge_otlp_log_batch` in `crates/vox-telemetry-otlp/src/otlp_json.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Updated upload_pending_otlp_with_auth in crates/vox-telemetry-otlp/src/u...
Updated `upload_pending_otlp_with_auth` in `crates/vox-telemetry-otlp/src/upload.rs` and `upload_pending` in `crates/vox-cli/src/telemetry_spool.rs` to chunk pending spool files and delete files only after `HTTP 2xx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Unified vox telemetry preview [--spool <PATH>] [--limit <N>] [--format j...
Unified `vox telemetry preview [--spool <PATH>] [--limit <N>] [--format json|table]` in `crates/vox-cli/src/commands/telemetry.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Staged server/telemetry/ and contracts/telemetry/collection-taxonomy.v1....
Staged `server/telemetry/` and `contracts/telemetry/collection-taxonomy.v1.json` into `/Users/brbrainerd/dev/vox-ops`, removed `server/telemetry/` from public `vox` and `Cargo.toml`, and updated `telemetry-trust-ssot.md` and `023-optional-telemetry-remote-upload.md`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-02-SUMMARY.md

---

### Enforced serde rename parity between Rust telemetry enums and contracts/...
Enforced serde rename parity between Rust telemetry enums and contracts/telemetry/collection-taxonomy.v1.json allowed strings.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-03-SUMMARY.md

---

### Verified both positive (all 26 TelemetryEvent variants) and negative (ma...
Verified both positive (all 26 TelemetryEvent variants) and negative (malformed metadata JSON) detection in VoxDb::audit_research_metric_shapes.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 89-03-SUMMARY.md

---

### Defined DurationBucket, TokenBucket, CountBucket, SizeBucket, CommandExi...
Defined `DurationBucket`, `TokenBucket`, `CountBucket`, `SizeBucket`, `CommandExitClass`, `EditOpType`, `EditFileKind`, `HarnessToolCallKind`, `HarnessMode`, `SkillTriggerSource`, `SkillSurface`, `ErrorSubsystem`, `TelemetryErrorClass`, `GpuVendor`, `VramBucket`, `TokPerSecBucket`, `DomainCategory`, `DomainDefectClass`, and `SaltedHashHex` in `crates/vox-telemetry/src/types.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-03-SUMMARY.md

---

### Implemented TypedResearchMetricPayload, TypedResearchMetricRow, DbShapeA...
Implemented `TypedResearchMetricPayload`, `TypedResearchMetricRow`, `DbShapeAuditReport`, and `VoxDb::audit_research_metric_shapes` in `crates/vox-db/src/telemetry_sink.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-03-SUMMARY.md

---

### Added multi_surface_model_router_and_db_shape_audit in crates/vox-orches...
Added `multi_surface_model_router_and_db_shape_audit` in `crates/vox-orchestrator/src/models/tests.rs` and `daemon_boot_steps_apply_persisted_routing_preferences_and_model_registry` in `crates/vox-orchestrator-d/src/daemon_steps/mod.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 89-03-SUMMARY.md

## Lessons

### Location: [crates/vox-telemetry-otlp/src/upload.rs](file:///Users/brbrai...
**Location:** [`crates/vox-telemetry-otlp/src/upload.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-telemetry-otlp/src/upload.rs)

**Context:** Surfaced in structured phase code review.
**Source:** 89-REVIEW.md

---

### Issue: upload_pending_otlp_with_auth batched up to MAX_OTLP_BATCH_RECORD...
**Issue:** `upload_pending_otlp_with_auth` batched up to `MAX_OTLP_BATCH_RECORDS = 100` files, but did not guard the serialized JSON batch against the `256 KiB` ceiling enforced by `vox-ops-ingest` (`POST /v1/logs`).

**Context:** Surfaced in structured phase code review.
**Source:** 89-REVIEW.md

---

### Fix Applied: Added pub const MAX_OTLP_BATCH_BYTES: usize = 256 * 1024; a...
**Fix Applied:** Added `pub const MAX_OTLP_BATCH_BYTES: usize = 256 * 1024;` and explicit pre-flight validation before sending the HTTP request.

**Context:** Surfaced in structured phase code review.
**Source:** 89-REVIEW.md

## Patterns

### Exhaustive match without wildcard arm in map_telemetry_event_to_row guar...
Exhaustive match without wildcard arm in map_telemetry_event_to_row guarantees compile-time failure whenever a new TelemetryEvent variant is added.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-01-SUMMARY.md

---

### Process-global TelemetryRecorder initialization via init_process_telemet...
Process-global TelemetryRecorder initialization via init_process_telemetry

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-01-SUMMARY.md

---

### Offline-first inspection via vox telemetry preview --format json|table u...
Offline-first inspection via `vox telemetry preview --format json|table` using the exact `merge_otlp_log_batch` function used by `upload_pending`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-02-SUMMARY.md

---

### Batched OTLP ExportLogsServiceRequest HTTP upload with post-2xx file ack...
Batched OTLP ExportLogsServiceRequest HTTP upload with post-2xx file acknowledgement

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-02-SUMMARY.md

---

### Multi-surface router verification testing CLI/Registry selection, MCP fl...
Multi-surface router verification testing CLI/Registry selection, MCP flywheel telemetry persistence, and daemon/GUI user_preferences round-trips in a single deterministic harness.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-03-SUMMARY.md

---

### Strongly-typed serde enum taxonomy parity + VoxDb::audit_research_metric...
Strongly-typed serde enum taxonomy parity + VoxDb::audit_research_metric_shapes

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 89-03-SUMMARY.md

## Surprises

