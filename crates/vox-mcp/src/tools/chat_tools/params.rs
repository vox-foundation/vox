//! Deserialize/Serialize shapes for chat, inline edit, planning, ghost text, and ambient tools.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Editor/LLM anti-stub rider appended to system prompts that must emit full code.
pub const ANTI_LAZINESS_RIDER: &str = "\nCRITICAL DIRECTIVE: You must output the COMPLETE, fully-implemented replacement code. DO NOT under any circumstances use placeholders, stubs, 'TODOs', or elide implementation details. Writing partial code is a catastrophic failure.";

/// One persisted chat turn in the session transcript (also returned in history APIs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTranscriptEntry {
    /// Opaque message id (UUID/ulid string).
    pub id: String,
    /// `"user"`, `"assistant"`, or `"system"`.
    pub role: String, // "user" | "assistant" | "system"
    /// Message body after expansion (mentions resolved server-side).
    pub content: String,
    /// Epoch seconds when stored.
    pub timestamp: u64,
    /// Extra files pulled in via @mentions or explicit attachments.
    pub context_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Model id recorded for assistant turns.
    pub model_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Approximate token usage when available.
    pub tokens: Option<u64>,
}

/// Arguments for `vox_chat_message` (prompt + rich editor context).
#[derive(Debug, Deserialize)]
pub struct ChatMessageParams {
    /// User message text (`message` is accepted for registry / legacy clients).
    #[serde(alias = "message")]
    pub prompt: String,
    #[serde(default)]
    /// Explicit @mention or attachment paths from the client.
    pub context_files: Vec<String>,
    /// Open file paths provided by the editor for implicit context injection
    #[serde(default)]
    pub open_files: Vec<String>,
    /// Active editor file path (workspace-relative)
    #[serde(default)]
    pub active_file: Option<String>,
    /// Active editor cursor line (1-indexed)
    #[serde(default)]
    pub active_line: Option<u32>,
    /// Selected text in the active editor
    #[serde(default)]
    pub selected_text: Option<String>,
    /// Active LSP diagnostics to inject as context
    #[serde(default)]
    pub diagnostics: Vec<Value>,
    /// Optional logical grouping identifier for this chat thread.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optionally selects a specific LLM routing profile (e.g. "reasoning", "fast", "creative").
    #[serde(default)]
    pub cognitive_profile: Option<String>,
    /// If true, enforces strict JSON output from the LLM.
    #[serde(default)]
    pub json_mode: bool,
}

/// Retrieve history for a specific session ID.
#[derive(Debug, Deserialize)]
pub struct ChatHistoryParams {
    /// Logical grouping identifier to fetch history for.
    pub session_id: String,
}

/// Arguments for `vox_inline_edit` (range replacement inside one file).
#[derive(Debug, Deserialize)]
pub struct InlineEditParams {
    /// The edit instruction / prompt from the user (`instruction` is a legacy alias).
    #[serde(alias = "instruction")]
    pub prompt: String,
    /// Workspace-relative file path (`file_path` is a legacy alias).
    #[serde(alias = "file_path")]
    pub file: String,
    /// Start line of target range (1-indexed)
    pub start_line: u32,
    /// End line of target range (1-indexed, inclusive)
    pub end_line: u32,
    /// The current text in the range (sent by editor; `selection` is a legacy alias).
    #[serde(alias = "selection")]
    pub current_text: String,
    /// Language ID of the file
    #[serde(default)]
    pub language: Option<String>,
    /// Surrounding context lines before and after the range (0-40 lines typically)
    #[serde(default)]
    pub context_before: Option<String>,
    #[serde(default)]
    /// Optional lines after the selection for better LLM grounding.
    pub context_after: Option<String>,
    /// If true, enforces strict JSON output from the LLM (rarely used for raw code edits).
    #[serde(default)]
    pub json_mode: bool,
    /// Optional tenant/session partition key for usage attribution.
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Successful inline edit payload returned to the editor host.
#[derive(Debug, Serialize)]
pub struct InlineEditResult {
    /// Replacement text for the range [`start_line`, `end_line`]
    pub replacement: String,
    /// Human-readable explanation of what was changed
    pub explanation: String,
    /// Estimated token usage
    pub tokens: u64,
    /// Model that produced this edit
    pub model_used: String,
}

/// Arguments for `vox_plan` structured planning tool.
#[derive(Debug, Deserialize)]
pub struct PlanParams {
    /// The request / goal to plan for
    pub goal: String,
    /// Optional files to scope the plan to
    #[serde(default)]
    pub scope_files: Vec<String>,
    /// Whether to write the plan to PLAN.md in the workspace root
    #[serde(default)]
    pub write_to_disk: bool,
    /// Maximum number of tasks to generate (default: 30)
    #[serde(default)]
    pub max_tasks: Option<usize>,
    /// Optional tenant/session partition key for usage attribution.
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Arguments for `vox_replan` — forwards to DeI `ai.plan.replan` when `vox-dei-d` is available.
#[derive(Debug, Deserialize)]
pub struct PlanReplanParams {
    /// Session id from a prior `vox_plan` or `ai.plan.new`.
    pub session_id: String,
    /// What changed since the last plan version.
    pub delta_hint: String,
    #[serde(default)]
    pub write_to_disk: bool,
    #[serde(default)]
    pub mode: Option<String>,
}

/// Arguments for `vox_plan_status` — forwards to DeI `ai.plan.status`.
#[derive(Debug, Deserialize)]
pub struct PlanStatusParams {
    /// Plan session id to query.
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// One row inside a generated plan (dependencies + complexity estimate).
pub struct PlanTask {
    /// Monotonic task index inside the plan.
    pub id: usize,
    /// Short imperative description.
    pub description: String,
    /// Related file paths for affinity routing.
    pub files: Vec<String>,
    /// Heuristic difficulty on a 1-10 scale.
    pub estimated_complexity: u8, // 1-10
    /// Task ids that should complete first.
    pub depends_on: Vec<usize>,
}

#[derive(Debug, Serialize)]
/// Full structured plan returned to the IDE / LLM.
pub struct PlanResult {
    /// Echo of the user goal string.
    pub goal: String,
    /// Ordered task breakdown.
    pub tasks: Vec<PlanTask>,
    /// One-line executive summary.
    pub summary: String,
    /// Markdown document (may mirror on-disk `PLAN.md`).
    pub plan_md: String,
    /// Whether `PLAN.md` was written under the workspace root.
    pub written_to_disk: bool,
}

/// Parameters for the `vox_ghost_text` MCP tool.
#[derive(Debug, Deserialize)]
pub struct GhostTextParams {
    /// Source code prefix (up to 20 lines before cursor).
    pub prefix: String,
    /// Source code suffix (up to 5 lines after cursor).
    pub suffix: String,
    /// VS Code language ID (e.g. "vox", "rust", "typescript").
    #[serde(default)]
    pub language: Option<String>,
    /// Workspace-relative file path for context.
    #[serde(default)]
    pub file_path: Option<String>,
    /// Maximum tokens to generate. Defaults to 128 for low latency.
    #[serde(default)]
    pub max_tokens: Option<u64>,
    /// Optional tenant/session partition key for usage attribution.
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Response from `vox_ghost_text`.
#[derive(Debug, Serialize)]
pub struct GhostTextResult {
    /// The generated completion text.
    pub completion: String,
    /// Model that produced this completion.
    pub model_used: String,
    /// Approximate token count.
    pub tokens: u64,
    /// Latency to first token (milliseconds, best-effort).
    pub latency_ms: u64,
}

/// Parameters for `vox_ambient_state`.
#[derive(Debug, Deserialize)]
pub struct AmbientStateParams {
    /// Optional workspace-relative path filter. Returns only decorations for this path prefix.
    #[serde(default)]
    pub path_prefix: Option<String>,
    /// Maximum number of decorations to return. Defaults to 100.
    #[serde(default)]
    pub limit: Option<usize>,
}
