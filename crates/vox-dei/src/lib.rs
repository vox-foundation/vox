//! **Workspace-excluded** DeI sources.
//!
//! The historical `research/orchestrator.rs` tree is not fully wired into this library yet.
//! This crate exists so Socrates-aligned constants can be type-checked via
//! `cargo check --manifest-path crates/vox-dei/Cargo.toml` and so excluded fragments can depend on
//! [`vox_socrates_policy`] without pulling the full DeI graph into the main workspace.
//! Runtime authority today is the MCP/orchestrator path (`vox-mcp` + `vox-orchestrator`);
//! retrieval trigger behavior and Socrates telemetry integration are implemented there.

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
