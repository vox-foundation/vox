//! Single-switch accessibility structures for the orchestrator.
//! Provides a flattened representation of actionable UX choices (Switch-Access API)
//! tailored to users with total motor paralysis or ALS utilizing scanning interfaces.
//! See Initiative Epsilon.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SwitchActionType {
    /// Immediate, destructive or final action (Confirm/Delete)
    Terminal,
    /// Progressive action (Next/More Info)
    Progressive,
    /// Corrective action (Undo/Back)
    Corrective,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub action_type: SwitchActionType,
    pub is_focused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchAccessMode {
    pub active_context: String,
    pub actionable_items: Vec<SwitchAction>,
    pub timestamp_ms: u64,
}

impl SwitchAccessMode {
    pub fn new(context: impl Into<String>, items: Vec<SwitchAction>) -> Self {
        Self {
            active_context: context.into(),
            actionable_items: items,
            timestamp_ms: crate::agent_types::ids::now_unix_ms(),
        }
    }
}
