//! Research pipeline subsystem for `vox-orchestrator`.
//!
//! See [`docs/src/architecture/scientia-self-publication-finalization-plan-2026.md`]
//! for the strategic context. This module is currently in **Phase 0a stub**
//! state: types are real, behavior returns empty/default values. Phase 1
//! replaces the stub bodies with the `vox-claim-extractor` crate.
//!
//! All stubs are marked `// PHASE_0a_STUB` for grep-based discovery.

pub mod claims;
pub(super) mod config;
pub mod gate;
pub mod model_select;
pub mod orchestrator;
pub mod persistence;
pub mod planner;
pub mod provider;
pub mod types;
pub mod verifier;

pub use orchestrator::{ResearchConfig, run_research};
pub use types::{
    Citation, CompetenceSignal, ResearchHit, ResearchMetadata, ResearchPlan, ResearchQuery,
    ResearchResult, ResearchScope, RetrievalDiagnostics, RoutingTier, SelfVerificationResult,
};
