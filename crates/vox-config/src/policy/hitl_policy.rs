//! Human-in-the-Loop (HITL) policy and configuration.
//!
//! Governs the "Doubt" loop, resolution tiers, and feedback semantics.

use serde::{Deserialize, Serialize};

/// High-level HITL configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HitlPolicy {
    /// Global toggle for the HITL Doubt loop.
    pub enabled: bool,
    /// Whether to play audio jingles in compatible surfaces (e.g. VS Code).
    pub audio_enabled: bool,
    /// Whether to show redundant visual stamps even when audio is enabled.
    pub visual_redundancy: bool,
    /// Model tier used for resolving user doubts.
    pub resolution_tier: String,
    /// Whether to allow the resolution agent to correctly "fight back" against user error.
    pub allow_obsequiousness_correction: bool,
    /// Factor by which Ludus XP is multiplied during intensive HITL sessions.
    pub ludus_multiplier: f64,
}

impl Default for HitlPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            audio_enabled: true,
            visual_redundancy: true,
            resolution_tier: "precision".to_string(), // Maps to high-tier models via vox-config resolution
            allow_obsequiousness_correction: true,
            ludus_multiplier: 1.5,
        }
    }
}
