//! Grammar-constrained inference engine for Vox.
//!
//! Provides a [`ConstrainedSampler`] trait with two backends:
//! - [`earley::EarleySampler`] — Earley parser consuming the EBNF grammar export
//! - [`pda::PdaSampler`] — Pushdown automaton with context-independent token caching
//!
//! Both are wrapped by:
//! - [`deadlock::DeadlockWatchdog`] — configurable timeout; produces `VoxValidationError` on deadlock
//! - [`revision::RevisionSampler`] — injects a backtrack token for mid-generation self-correction
//!
//! **Research rationale:** Grammar Constraints §4.4 proves deadlocks are systemic, not edge-case.
//! XGrammar-2/llguidance PDA backends achieve <40µs/token with native recursion support.

pub mod deadlock;
pub mod earley;
pub mod pda;
pub mod revision;

mod constrained_sampler;
mod error;
mod factory;
mod grammar_mode;
mod sampler_state;

pub use constrained_sampler::ConstrainedSampler;
pub use error::{ConstrainedGenError, Result};
pub use factory::build_sampler;
pub use grammar_mode::GrammarMode;
pub use sampler_state::SamplerState;
