use serde::{Deserialize, Serialize};

/// Selects which grammar constraint backend to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrammarMode {
    /// No grammar constraint — pass all logits through.
    None,
    /// Vox language grammar (EBNF-derived, Earley backend).
    Vox,
    /// Vox language grammar (PDA backend — lower latency, context-independent caching).
    VoxPda,
    /// JSON schema constraint (existing FSM).
    Json,
}

impl Default for GrammarMode {
    fn default() -> Self {
        Self::None
    }
}
