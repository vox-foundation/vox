---
phase: 89-telemetry-surface-audit-remediation-daemon-gui-sinks-public
plan: 03
subsystem: telemetry
tags: [telemetry, typing, vox-db, model-router, orchestrator]
requires: ["89-01", "89-02"]
provides:
  - "18 strongly-typed telemetry taxonomy enums + SaltedHashHex newtype in vox-telemetry"
  - "TypedResearchMetricPayload, TypedResearchMetricRow, DbShapeAuditReport, and VoxDb::audit_research_metric_shapes in vox-db"
  - "Multi-surface model router and 26-variant local DB shape audit tests across CLI, daemon, MCP, and GUI"
affects: [vox-telemetry, vox-db, vox-orchestrator, vox-orchestrator-d]
tech-stack:
  added: []
  patterns: ["Strongly-typed serde enum taxonomy parity + VoxDb::audit_research_metric_shapes"]
key-files:
  created: []
  modified:
    - crates/vox-telemetry/src/types.rs
    - crates/vox-telemetry/src/lib.rs
    - crates/vox-db/src/telemetry_sink.rs
    - crates/vox-orchestrator/src/models/tests.rs
    - crates/vox-orchestrator-d/src/daemon_steps/mod.rs
key-decisions:
  - "Enforced serde rename parity between Rust telemetry enums and contracts/telemetry/collection-taxonomy.v1.json allowed strings."
  - "Verified both positive (all 26 TelemetryEvent variants) and negative (malformed metadata JSON) detection in VoxDb::audit_research_metric_shapes."
patterns-established:
  - "Multi-surface router verification testing CLI/Registry selection, MCP flywheel telemetry persistence, and daemon/GUI user_preferences round-trips in a single deterministic harness."
requirements-completed: [TYPING-01, TYPING-02, ROUTER-01, ROUTER-02, ROUTER-03, ROUTER-04, DBAUDIT-01, DBAUDIT-02]
duration: 15min
completed: 2026-10-19
---

# Phase 89 Plan 03: End-to-End Telemetry Typing, Multi-Surface Model Router & DB Shape Audit Summary

**Added 18 strongly-typed taxonomy enums + `SaltedHashHex` in `vox-telemetry`, implemented `VoxDb::audit_research_metric_shapes` for all 26 `TelemetryEvent` variants, and verified multi-surface model routing across CLI, daemon, MCP, and GUI.**

## Accomplishments
- Defined `DurationBucket`, `TokenBucket`, `CountBucket`, `SizeBucket`, `CommandExitClass`, `EditOpType`, `EditFileKind`, `HarnessToolCallKind`, `HarnessMode`, `SkillTriggerSource`, `SkillSurface`, `ErrorSubsystem`, `TelemetryErrorClass`, `GpuVendor`, `VramBucket`, `TokPerSecBucket`, `DomainCategory`, `DomainDefectClass`, and `SaltedHashHex` in `crates/vox-telemetry/src/types.rs`.
- Implemented `TypedResearchMetricPayload`, `TypedResearchMetricRow`, `DbShapeAuditReport`, and `VoxDb::audit_research_metric_shapes` in `crates/vox-db/src/telemetry_sink.rs`.
- Added `multi_surface_model_router_and_db_shape_audit` in `crates/vox-orchestrator/src/models/tests.rs` and `daemon_boot_steps_apply_persisted_routing_preferences_and_model_registry` in `crates/vox-orchestrator-d/src/daemon_steps/mod.rs`.

## Verification
- `cargo test -p vox-db --lib telemetry_sink` — passed.
- `cargo test -p vox-orchestrator --lib multi_surface_model_router_and_db_shape_audit` — passed.
- `cargo test -p vox-orchestrator-d` — passed.
