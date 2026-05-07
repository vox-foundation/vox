use serde::{Deserialize, Serialize};

/// Selects which grammar constraint backend to use.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrammarMode {
    /// No grammar constraint — pass all logits through.
    #[default]
    None,
    /// Vox language grammar (EBNF-derived, Earley backend).
    Vox,
    /// Vox language grammar (PDA backend — lower latency, context-independent caching).
    VoxPda,
    /// JSON schema constraint (existing FSM).
    Json,
}
