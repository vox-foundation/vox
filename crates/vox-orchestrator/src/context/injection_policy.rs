use serde::{Deserialize, Serialize};
use crate::context_envelope::{ContextPriority, ContextMergeStrategy};

/// Policy for deciding which context envelopes to inject into an agent's session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInjectionPolicy {
    /// Minimum priority required for automatic injection.
    pub min_priority: ContextPriority,
    /// Maximum number of envelopes to inject per turn.
    pub max_envelopes_per_turn: usize,
    /// Default merge strategy for conflicting facts.
    pub default_merge_strategy: ContextMergeStrategy,
}

impl Default for ContextInjectionPolicy {
    fn default() -> Self {
        Self {
            min_priority: ContextPriority::Normal,
            max_envelopes_per_turn: 5,
            default_merge_strategy: ContextMergeStrategy::AppendOnly,
        }
    }
}

/// Evaluates if an envelope should be injected based on the policy.
pub fn should_inject_envelope(
    policy: &ContextInjectionPolicy,
    priority: ContextPriority,
    current_count: usize,
) -> bool {
    if priority < policy.min_priority {
        return false;
    }
    if current_count >= policy.max_envelopes_per_turn {
        return false;
    }
    true
}
