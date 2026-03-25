//! RPC parameter and response types for all Vox MCP tools.
//!
//! All `#[derive(Deserialize)]` structs accepted by tool handlers live here.
//! All `#[derive(Serialize)]` response types live here.
//! The generic [`ToolResult<T>`] envelope lives here.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Generic tool result envelope
// ---------------------------------------------------------------------------

/// A standard envelope for all tool responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult<T> {
    /// Whether the tool invocation succeeded.
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Success payload when `success` is true.
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Error message when `success` is false.
    pub error: Option<String>,
}

impl<T: Serialize> ToolResult<T> {
    /// Wraps a successful value in a [`ToolResult`].
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Builds a failed [`ToolResult`] with the given message.
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }

    /// Serializes this result to pretty-printed JSON for MCP text content.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| {
            format!("{{\"success\":false,\"error\":\"serialization failed: {e}\"}}")
        })
    }
}

// ---------------------------------------------------------------------------
// File / task request types
// ---------------------------------------------------------------------------

/// File path and access mode for task affinity.
#[derive(Debug, Deserialize)]
pub struct FileSpec {
    /// Workspace-relative or absolute file path.
    pub path: String,
    /// Access mode, e.g. `"read"` or `"write"`.
    pub access: String,
}

/// Arguments for submitting a new orchestrator task.
#[derive(Debug, Deserialize)]
pub struct SubmitTaskParams {
    /// Natural-language task description.
    pub description: String,
    /// Files this task may touch (drives locking and routing).
    pub files: Vec<FileSpec>,
    /// Optional queue priority hint (`"urgent"`, `"normal"`, `"background"`).
    pub priority: Option<String>,
    /// Optional named agent; enforces scope when configured.
    pub agent_name: Option<String>,
    /// Optional GPU / hardware routing hints.
    #[serde(default)]
    pub capabilities: Option<vox_orchestrator::TaskCapabilityHints>,
    /// Optional session identifier for Mens telemetry grouping.
    pub session_id: Option<String>,
    /// Optional planning mode (`auto`, `direct`, `force_plan`, `workflow_only`).
    pub planning_mode: Option<String>,
    /// Optional semantic goal type hint for planning.
    pub goal_type: Option<String>,
    /// Optional scope hint for planning.
    pub goal_scope: Option<String>,
    /// Optional cap on planner depth.
    pub max_plan_depth: Option<u32>,
    /// Optional retrieval envelope to seed Socrates task context.
    #[serde(default)]
    pub retrieval: Option<crate::memory::RetrievalEvidenceEnvelope>,
}

/// Identifier payload returned after a successful [`SubmitTaskParams`] submission.
#[derive(Debug, Serialize)]
pub struct SubmitTaskResponse {
    /// Newly assigned task id.
    pub task_id: u64,
    /// Agent id the task was routed to.
    pub agent_id: u64,
    /// Whether the task description was canonicalized (order-invariant, normalized).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_canonicalized: Option<bool>,
    /// Conflict warnings from the prompt canonicalization pipeline (for transparency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_warnings: Option<Vec<String>>,
    /// Hash of the original prompt for debug/traceability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_prompt_hash: Option<String>,
    /// Present when [`OrchestratorConfig::orchestration_migration`] has v2 enabled (MCP hint).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orchestration_contract: Option<String>,
}

/// Query the status of a single task by id.
#[derive(Debug, Deserialize)]
pub struct TaskStatusParams {
    /// Task id to look up.
    pub task_id: u64,
}

/// Mark a task completed.
#[derive(Debug, Deserialize)]
pub struct CompleteTaskParams {
    /// Task id to complete.
    pub task_id: u64,
}

/// Mark a task failed with a reason string.
#[derive(Debug, Deserialize)]
pub struct FailTaskParams {
    /// Task id to fail.
    pub task_id: u64,
    /// Human-readable failure reason.
    pub reason: String,
}

/// Cancel a queued or in-progress task.
#[derive(Debug, Deserialize)]
pub struct CancelTaskParams {
    /// Task id to cancel.
    pub task_id: u64,
}

/// Change priority of a queued task.
#[derive(Debug, Deserialize)]
pub struct ReorderTaskParams {
    /// Task id to reorder.
    pub task_id: u64,
    /// New priority (`"urgent"`, `"normal"`, or `"background"`).
    pub priority: String,
}

/// Remove all queued work from an agent without retiring it.
#[derive(Debug, Deserialize)]
pub struct DrainAgentParams {
    /// Target agent id.
    pub agent_id: u64,
}

/// Bind an external session id to an orchestrator agent.
#[derive(Debug, Deserialize)]
pub struct MapAgentSessionParams {
    /// Agent id to update.
    pub agent_id: u64,
    /// Opaque session identifier from the client.
    pub session_id: String,
}

/// Validate a `.vox` source file through the compiler pipeline.
#[derive(Debug, Deserialize)]
pub struct ValidateFileParams {
    /// Path to the `.vox` file.
    pub path: String,
}

/// Run `cargo test` for one crate with an optional name filter.
#[derive(Debug, Deserialize)]
pub struct RunTestsParams {
    /// Cargo package name (`-p` target).
    pub crate_name: String,
    /// Optional substring filter passed after `--`.
    pub test_filter: Option<String>,
}

/// Publish a bulletin-board style message (orchestrator-internal).
#[derive(Debug, Deserialize)]
pub struct PublishMessageParams {
    /// Message body to publish.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Per-agent summary in a [`StatusResponse`].
#[derive(Debug, Serialize)]
pub struct AgentInfo {
    /// Agent id.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Tasks waiting in queue.
    pub queued: usize,
    /// Tasks finished historically.
    pub completed: usize,
    /// Whether the agent is paused.
    pub paused: bool,
}

/// High-level orchestrator snapshot for MCP status tools.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Number of registered agents.
    pub agent_count: usize,
    /// Tasks currently executing.
    pub in_progress: usize,
    /// Tasks finished across all agents.
    pub completed: usize,
    /// Per-agent rows.
    pub agents: Vec<AgentInfo>,
    /// Current scaling profile (conservative / balanced / aggressive) for transparency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_profile: Option<String>,
    /// Effective scale-up threshold (scaling_threshold * profile multiplier) for explainability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_scale_up_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Gamify companion state when Codex is attached.
    pub companion: Option<vox_ludus::companion::Companion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Pre-rendered Markdown summary for clients.
    pub markdown_summary: Option<String>,
    // -- VCS health stats --
    /// Total snapshots stored in the snapshot ring-buffer.
    pub snapshot_count: usize,
    /// Total operations in the operation log.
    pub oplog_count: usize,
    /// Number of active (unresolved) file conflicts.
    pub active_conflicts: usize,
    /// Number of active agent workspaces.
    pub active_workspaces: usize,
    /// Number of tracked logical changes.
    pub active_changes: usize,
    /// Read-only mens HTTP snapshot when [`vox_orchestrator::OrchestratorConfig::populi_control_url`] is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh_snapshot: Option<serde_json::Value>,
    /// Background-polled mens federation cache (same URL); does not replace `mesh_snapshot` live fetch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub populi_federation_cache: Option<serde_json::Value>,
    /// Optional planning summary from persisted plan sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planning: Option<serde_json::Value>,
}

/// Single LSP-style diagnostic for validate-file responses.
#[derive(Debug, Serialize)]
pub struct DiagnosticInfo {
    /// `"error"` or `"warning"`.
    pub severity: String,
    /// Human-readable diagnostic text.
    pub message: String,
    /// Tool or compiler source label.
    pub source: String,
    /// Start line (0-based).
    pub start_line: u32,
    /// Start column (0-based).
    pub start_col: u32,
    /// End line (0-based).
    pub end_line: u32,
    /// End column (0-based).
    pub end_col: u32,
}

/// Result of validating a Vox source file.
#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    /// Number of diagnostics reported.
    pub count: usize,
    /// Diagnostic list.
    pub diagnostics: Vec<DiagnosticInfo>,
}
