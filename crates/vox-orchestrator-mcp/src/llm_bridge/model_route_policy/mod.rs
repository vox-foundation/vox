//! Pure model resolution for MCP chat: registry lookup, free-tier enforcement, context signals.

pub mod budget_guard;
mod free_tier_adapter;
mod policy;
pub(crate) mod resolve;
pub mod turn_decision_pack;
mod types;

#[cfg(test)]
#[allow(unsafe_code)] // tests use `set_var` / `remove_var` under a global lock (Rust 2024)
mod tests;

#[cfg(test)]
pub(crate) use resolve::build_selection_request;
pub use resolve::{
    build_selection_request_with_decision_pack, mcp_global_llm_context_fill_ratio,
    mcp_provider_telemetry_labels, resolve_mcp_chat_model, resolve_mcp_chat_model_sync,
    resolve_mcp_chat_model_with_rationale,
};
pub use turn_decision_pack::{
    ALLOW_JEV_DISTILLATION_INTO_MENS_SFT, CalibrationSample, CallerExplicitRouteFlags,
    SelectivePredictionMetrics, TURN_DECISION_PACK_SCHEMA_V1, TurnDecisionPack,
    TurnDecisionPatchOutcome, apply_turn_decision_pack_upstream, evaluate_selective_prediction,
    hosted_jev_consultation_allowed, validate_jev_training_isolation,
};
pub use types::McpChatModelResolution;
