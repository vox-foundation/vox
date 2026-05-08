//! Shared **Socrates** confidence policy for orchestrator, MCP, and TOESTUB review.
//!
//! Single source of truth for numeric thresholds so prompts, filters, and gates stay aligned.
//! See `docs/src/architecture/socrates-protocol-ssot.md`.

mod complexity;
mod confidence_override;
mod confidence_policy;
mod entropy;
mod policy_types;

pub use complexity::SocratesComplexityJudge;
pub use confidence_override::ConfidencePolicyOverride;
pub use entropy::{expected_information_gain_bits, shannon_entropy_bits};
pub use policy_types::{
    CLARIFICATION_INTERRUPT_COST_MS, ClarificationStopReason, ComplexityBand, ConfidencePolicy,
    QuestionCandidate, QuestionKind, QuestionSelection, QuestioningPolicy, RiskBand, RiskDecision,
    SocratesResearchDecision,
};
