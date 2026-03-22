//! Retrieval tier and context bundle assembly for CLI / MCP tooling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Named retrieval depth presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalTier {
    /// Minimal context.
    Shallow,
    /// Default depth.
    #[default]
    Standard,
    /// Deeper recall.
    Deep,
}

impl RetrievalTier {
    /// Parse CLI tier string (`shallow`, `standard`, `deep`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "shallow" => Some(Self::Shallow),
            "standard" => Some(Self::Standard),
            "deep" => Some(Self::Deep),
            _ => None,
        }
    }
}

/// Policy controlling how many items are pulled into a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicy {
    /// Active tier (informational; filtering may use [`Self::max_items`]).
    pub tier: RetrievalTier,
    /// Maximum JSON values to retain.
    pub max_items: u32,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            tier: RetrievalTier::Standard,
            max_items: 10,
        }
    }
}

/// Assembled context for prompting or inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArsContextBundle {
    /// Bundle label (e.g. CLI command name).
    pub label: String,
    /// Tier used when assembling.
    pub tier: RetrievalTier,
    /// Retrieved items (typically memory rows as JSON).
    pub items: Vec<Value>,
}

/// Merge `sources` into a bundle honoring `policy.max_items`.
pub fn assemble_bundle(
    label: impl Into<String>,
    policy: &ContextPolicy,
    sources: Vec<Value>,
) -> ArsContextBundle {
    let mut items: Vec<Value> = sources
        .into_iter()
        .take(policy.max_items as usize)
        .collect();
    items.shrink_to_fit();
    ArsContextBundle {
        label: label.into(),
        tier: policy.tier,
        items,
    }
}
