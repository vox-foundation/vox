//! MENS and GRPO training types for [`crate::VoxDb`].

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A report on an agent interaction observed by the [`Observer`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationReport {
    pub session_id: String,
    pub task_id: String,
    pub observed_at: DateTime<Utc>,
    pub file_path: String,
    pub lsp_error_count: usize,
    pub parse_rate: f32,
    pub construct_coverage: f32,
    pub recommended_action: ObserverAction,
    pub metadata_json: Option<String>,
}

/// Action recommended by the observer based on language invariants.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObserverAction {
    /// Everything looks correct, proceed.
    None,
    /// Minor syntax issues, continue but flag for review.
    Warn,
    /// Severe syntax or LSP errors, trigger immediate replan.
    Replan,
    /// Catastrophic failure, abort current path.
    Abort,
}

/// A decision made by the testing policy for a task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestDecision {
    /// No testing required for this task.
    None,
    /// Create new tests for the modified functionality.
    CreateTests,
    /// Run existing tests to verify the change.
    RunExisting,
    /// Both create and run tests (highest risk).
    FullTest,
}

/// The policy used to decide if and what testing is required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDecisionPolicy {
    pub complexity_threshold: u8,
    pub file_count_threshold: usize,
    pub risk_multiplier: f32,
}

/// A multi-tier victory condition verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VictoryVerdict {
    pub task_id: String,
    pub passed: bool,
    pub tiers_run: Vec<String>,
    pub first_failure: Option<String>,
    pub report: String,
    pub created_at_ms: i64,
}

/// Result of a single victory tier check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierResult {
    pub tier: String,
    pub passed: bool,
    pub diagnostic: String,
}
