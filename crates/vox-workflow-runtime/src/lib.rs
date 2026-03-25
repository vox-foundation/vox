//! Minimal **interpreted** workflow runner: walks a [`vox_compiler::hir::HirModule`] workflow body for
//! activity calls and executes **no-op** steps with optional mens hooks.
//!
//! - Activities whose name starts with `mesh_` are treated as [`PopuliActivity`] steps when the
//!   **`mens`** feature is enabled: they register with [`vox_populi::publish_local_registry_best_effort`]
//!   and call the mens HTTP control plane derived from **`VOX_MESH_CONTROL_ADDR`** / `Vox.toml`
//!   `[mens]` (never a user-supplied URL in workflow source). Use `with { mens: "noop" | "join" |
//!   "snapshot" | "heartbeat" }` to select the operation; see `mesh_noop`, `mesh_join`,
//!   `mesh_snapshot` shorthands.
//! - Other activities are recorded as local no-ops (journal only).
//!
//! **Codex journal:** when **`VOX_WORKFLOW_JOURNAL_CODEX=1`** (and DB config resolves), `vox-cli`
//! persists the interpreted journal after a successful run via `VoxDb::record_workflow_journal_entry`
//! (see `docs/src/architecture/orchestration-unified-ssot.md`). Journal rows include
//! **`ActivityStarted` / `ActivityCompleted`** with **`activity_id`** for idempotency hints.
//!
//! This crate is the MVP engine behind `vox mens workflow run` when `vox-cli` is built with
//! **`workflow-runtime`**.

#![deny(missing_docs)]

pub mod db_tracker;
pub mod workflow;

pub use db_tracker::VoxDbTracker;
#[cfg(feature = "mens")]
pub use workflow::execute_populi_step;
pub use workflow::{
    DefaultTracker, PlannedActivity, PopuliActivity, PopuliHttpOp, WorkflowTracker,
    interpret_workflow, interpret_workflow_durable, plan_workflow_activities,
};
