//! # vox-orchestrator-core — boundary marker crate
//!
//! **Status: boundary marker** (same pattern as `vox-cli-ci` from PR3).
//!
//! ## Intent
//!
//! This crate marks the intended extraction boundary for the core
//! router/dispatcher logic that currently lives in
//! `crates/vox-orchestrator/src/orchestrator/` (~11,532 LoC, 49 files).
//!
//! That subdir includes:
//! - `orchestrator/core/` — `Orchestrator::new`, init, telemetry, usage
//! - `orchestrator/agent/` — spawn, registration, lifecycle_ops, handoff, doubt
//! - `orchestrator/task_dispatch/` — submit (goal, batch, dei_plan_materialize),
//!   complete (success, fail, harness)
//! - `orchestrator/persistence/` — lifecycle, replay
//! - `orchestrator/scaling.rs`, `campaigns.rs`, `vcs_ops.rs`, etc.
//!
//! ## Why extraction is blocked
//!
//! The `orchestrator/` subdir uses `crate::` imports into 30+ sibling modules
//! of `vox-orchestrator`:
//!
//! ```text
//! planning, services, budget, scope, affinity, bulletin, config, locks,
//! groups, types, topology, snapshot, oplog, context, events, a2a, monitor,
//! qa, catalog, models, heartbeat, tool_receipt, privacy_router, judge_model,
//! reconstruction, attention, summary, workspace, conflicts
//! ```
//!
//! Moving the code to a separate crate would require either:
//! 1. Pulling all 30+ sibling modules along (defeating the purpose), or
//! 2. Creating `vox-orchestrator` → `vox-orchestrator-core` → `vox-orchestrator`
//!    circular deps (not allowed by Cargo).
//!
//! ## Path to full extraction
//!
//! The cross-cuts need to be addressed first:
//! - Pure-data types (`types/`, `topology/`) → `vox-orchestrator-types` (L0)
//! - Queue primitives (`oplog`, `locks`, `affinity`) → already in `vox-orchestrator-queue`
//! - Service interfaces (`services/`, `planning/`) → could move to a new L2
//!   `vox-orchestrator-policy` or `vox-orchestrator-services` crate
//! - Once sibling deps are L0/L1/L2, `orchestrator/` can move here cleanly
//!
//! ## Spec reference
//!
//! `docs/src/architecture/2026-05-08-crate-org-followup-design.md` §C5 (PR5).
