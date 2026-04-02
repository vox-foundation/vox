//! **DeI staging crate** (workspace member; not a dependency of `vox-cli` — see `vox ci no-vox-dei-import`).
//!
//! The historical `research/` and `selection/` trees on disk are not wired into this library yet.
//! This crate exists so Socrates-aligned constants are type-checked with **`cargo check -p vox-dei`**
//! and can share [`vox_socrates_policy`] without pulling a full DeI graph into consumers.
//! Runtime authority today is the MCP/orchestrator path (`vox-mcp` + `vox-orchestrator`);
//! retrieval trigger behavior and Socrates telemetry integration are implemented there.
//!
//! ## Public modules
//! - [`research_policy`] — persisted confidence floors shared with Socrates policy.
//! - [`route_telemetry`] — structured `tracing` events for model routing (target `vox_dei::model_route`).

pub mod route_telemetry;

/// Numeric floors shared with [`vox_socrates_policy::ConfidencePolicy`] for research persistence
/// and Mens training-pair export (see `research/orchestrator.rs` when that module is reattached).
pub mod research_policy {
    pub use vox_socrates_policy::ConfidencePolicy;

    /// Same numeric floor as [`ConfidencePolicy::min_persist_confidence`] on [`ConfidencePolicy::workspace_default`].
    #[must_use]
    pub const fn persist_min_confidence() -> f64 {
        ConfidencePolicy::DEFAULT_MIN_PERSIST_CONFIDENCE
    }

    /// Same numeric floor as [`ConfidencePolicy::min_training_pair_confidence`] on [`ConfidencePolicy::workspace_default`].
    #[must_use]
    pub const fn training_pair_min_confidence() -> f64 {
        ConfidencePolicy::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE
    }
}
