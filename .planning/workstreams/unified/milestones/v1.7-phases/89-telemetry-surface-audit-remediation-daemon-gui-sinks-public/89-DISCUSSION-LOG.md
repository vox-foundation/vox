# Phase 89: Telemetry Surface Audit Remediation, Daemon/GUI Sinks & Public Repo Cleanup - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in `89-CONTEXT.md` — this log preserves the alternatives considered.

**Date:** 2026-10-19
**Phase:** 89-Telemetry Surface Audit Remediation, Daemon/GUI Sinks & Public Repo Cleanup
**Areas discussed:** Daemon & GUI Recorder Wiring, Exhaustive `ResearchMetricsSink` & Taxonomy Coverage, Batched OTLP Uploader & `vox telemetry preview`, `server/telemetry/` Extraction & Removal Order

---

## 1. Daemon & GUI Recorder Wiring (`TELEM-01`)

| Option | Description | Selected |
|--------|-------------|----------|
| Zero-new-edge wiring via `vox_db::telemetry_sink::init_process_telemetry(db)` | Register `ResearchMetricsSink(db)` + `StderrDebugSink` in `vox-orchestrator-d` (which owns `VoxDb`), and in `vox-gui` forward GUI events over the existing `PersistentDaemon` RPC / `StderrDebugSink` so `vox-gui` never opens a competing local `VoxDb` connection (`CORE-04`) and zero `Cargo.toml` crate-edges change | ✓ |
| Add `vox-telemetry-otlp` dependency to `vox-orchestrator-d` and `vox-gui` | Requires updating `Cargo.toml`, `layers.toml`, and `crate-edges.allow.v1.json` | |

**User's choice:** Zero-new-edge wiring via `vox_db::telemetry_sink::init_process_telemetry(db)`

---

## 2. Exhaustive `ResearchMetricsSink` & Taxonomy Coverage (`TELEM-02`)

| Option | Description | Selected |
|--------|-------------|----------|
| Exhaustive `match event` over all 21 `TelemetryEvent` variants + 3 new variants (`ModelBenchCase`, `TrainingThroughput`, `DomainMisguidance`) | Remove `_ => {}` so the compiler rejects unhandled future variants, and add `ModelBenchCase`, `TrainingThroughput`, and `DomainMisguidance` (`culprit_domain` + `defect_class` only) to `vox-telemetry` and `collection-taxonomy.v1.json` now | ✓ |
| Map only existing 21 variants and defer the 3 new variants to Phase 92 | Leaves Phase 90's Postgres views without schema definitions for the 3 new intelligence domains | |

**User's choice:** Exhaustive `match event` over all 21 `TelemetryEvent` variants + add `ModelBenchCase`, `TrainingThroughput`, and `DomainMisguidance` now

---

## 3. Batched OTLP Uploader & `vox telemetry preview` (`TELEM-03`, `TELEM-04`)

| Option | Description | Selected |
|--------|-------------|----------|
| Batched `upload_pending_otlp` (`<= 200` records / `256 KiB` per `POST /v1/logs`) + `vox telemetry preview [--limit <N>] [--format json\|table]` | Implements real batched OTLP/HTTP upload with `governor` rate limiting and Clavis auth, and adds offline `vox telemetry preview` | ✓ |
| Keep 1-file-per-POST and alias `preview` to `export` | Does not implement `upload_pending_otlp` batching | |

**User's choice:** Batched `upload_pending_otlp` (`<= 200` records / `256 KiB`) + `vox telemetry preview [--limit <N>] [--format json|table]`

---

## 4. `server/telemetry/` Extraction & Removal Order (`TELEM-05`)

| Option | Description | Selected |
|--------|-------------|----------|
| Copy `server/telemetry/` and `collection-taxonomy.v1.json` into `~/dev/vox-ops` before removing `server/telemetry/` from public `vox` | Preserves reference implementation in `~/dev/vox-ops` before deleting from public `vox` and updating `telemetry-trust-ssot.md` + `ADR-023` | ✓ |
| Delete `server/telemetry/` without copying to `~/dev/vox-ops` first | Relies on git history when Phase 90 starts | |

**User's choice:** Copy `server/telemetry/` and `collection-taxonomy.v1.json` into `~/dev/vox-ops` before removing `server/telemetry/` from public `vox`

---

## the agent's Discretion

- Internal helper function signatures inside `crates/vox-db/src/telemetry_sink.rs` and `crates/vox-telemetry-otlp/src/upload.rs`.

## Deferred Ideas

- `vox telemetry intelligence {status,sync,inspect}` CLI subcommands — scheduled for **Phase 92 (`INTEL-04`)**.
