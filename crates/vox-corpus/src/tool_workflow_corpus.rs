//! Schemas for tool, workflow, and A2A traces distilled into supervised JSONL.

use serde::{Deserialize, Serialize};

/// One tool invocation suitable for SFT (prompt → tool → args → result).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTraceRecord {
    /// User or system text that triggered the tool call.
    pub task_prompt: String,
    /// Registered tool name (MCP or internal).
    pub tool_name: String,
    /// JSON-serialized arguments object.
    pub arguments_json: String,
    /// JSON-serialized tool result payload.
    pub result_json: String,
    /// Whether the invocation completed without error.
    pub success: bool,
    /// Optional assistant follow-up after the tool result.
    pub followup_text: Option<String>,
    /// Correlates multi-turn sessions when present.
    pub session_id: Option<String>,
}

/// Workflow step trace for training plan → skeleton expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTraceRecord {
    /// Natural-language workflow goal.
    pub intent: String,
    /// Resolved durable workflow name, if known.
    pub workflow_name: Option<String>,
    /// Truncated execution log for supervision.
    pub execution_log_excerpt: String,
    /// Model-produced Vox snippet, if captured.
    pub synthesized_vox: Option<String>,
    /// NNT small-world routing efficiency score (0.0-1.0).
    pub routing_efficiency: Option<f64>,
}

/// A2A message pair for coordination training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATraceRecord {
    /// Sending agent id or role label.
    pub from_agent: String,
    /// Receiving agent id or role label.
    pub to_agent: String,
    /// Discriminator for routing / schema (e.g. `task`, `handoff`).
    pub message_type: String,
    /// JSON body of the A2A message.
    pub payload_json: String,
    /// Suggested reply JSON for SFT targets.
    pub recommended_reply_json: Option<String>,
}

/// Serialize `row` as a single JSONL line (no trailing newline).
pub fn jsonl_line<T: Serialize>(row: &T) -> anyhow::Result<String> {
    Ok(serde_json::to_string(row)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegativeLexParseTypeckRecord {
    pub source: String,
    pub errors_json: String,
    pub origin: String,
    pub reward_signal: f64,
}

#[cfg(feature = "database")]
pub async fn auto_ingest_negative_vox(
    source: &str,
    errors_json: &str,
    db: &vox_db::VoxDb,
) -> anyhow::Result<()> {
    db.upsert_corpus_pair(source, errors_json, "agent", 0.0, "negative")
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}
