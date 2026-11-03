//! Orchestrator-group wire DTOs (plan 07 Task 10): task hopper, context budget, live plan DAG
//! edits, GUI-run journal and the activity timeline. Field names are part of the Axis UI
//! contract — do NOT rename.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Token budget snapshot for the chat context meter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudgetPayload {
    /// Maximum tokens the model's context can hold (from `CompactionConfig`).
    pub max_context_tokens: usize,
    /// Tokens reserved for the model's response (subtracted from usable budget).
    pub reserved_tokens: usize,
    /// Token count at which compaction triggers (`max * compaction_threshold`).
    pub threshold_tokens: usize,
    /// Usable token budget (`max - reserved`).
    pub usable_tokens: usize,
    /// Human-readable compaction strategy name: "aggressive", "balanced", or "conservative".
    pub strategy: String,
    /// Input + output of the session's most recent `llm_interactions` row; zero when no session
    /// is given, none is recorded, or the server has no DB.
    pub used_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HopperTaskDto {
    pub item_id: String,
    pub intent: String,
    pub priority: u8,
    pub state: String,
    pub task_id: u64,
    /// Chat/CLI session the item was submitted from (persisted column).
    pub session_id: Option<String>,
    /// Agent bound to the item while `state == "assigned"` (from state JSON).
    pub agent_id: Option<String>,
    /// Origin daemon for mesh-replicated items (from source JSON).
    pub remote_node: Option<String>,
}

/// The most recently updated `plan_sessions` row linked to a chat session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestPlanSessionForChat {
    pub plan_session_id: String,
    pub plan_version: i64,
}

/// One live plan-DAG node (`plan_nodes` row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanNodeDto {
    pub plan_session_id: String,
    pub version: i64,
    pub node_id: String,
    pub description: String,
    pub dependencies_json: String,
    pub execution_policy_json: String,
    pub status: String,
    pub workflow_invocation: Option<String>,
}

impl From<vox_db::PlanNodeRow> for PlanNodeDto {
    fn from(row: vox_db::PlanNodeRow) -> Self {
        Self {
            plan_session_id: row.plan_session_id.into_string(),
            version: row.version,
            node_id: row.node_id,
            description: row.description,
            dependencies_json: row.dependencies_json,
            execution_policy_json: row.execution_policy_json,
            status: row.status,
            workflow_invocation: row.workflow_invocation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdatePlanNodeInput {
    pub plan_session_id: String,
    pub plan_version: i64,
    pub node_id: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InsertPlanNodeInput {
    pub plan_session_id: String,
    pub plan_version: i64,
    pub node_id: String,
    pub description: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiRunRecord {
    pub run_id: String,
    pub workflow_name: String,
    pub status: String,
    pub planned_steps: i64,
    pub completed_steps: i64,
    pub started_at_ms: i64,
    pub updated_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub last_error: Option<String>,
    pub command: Option<String>,
    pub model: Option<String>,
    pub cost_usd: Option<f64>,
}

impl From<vox_db::AgentRunRow> for GuiRunRecord {
    fn from(row: vox_db::AgentRunRow) -> Self {
        Self {
            run_id: row.run_id,
            workflow_name: row.workflow_name,
            status: row.status,
            planned_steps: row.planned_steps,
            completed_steps: row.completed_steps,
            started_at_ms: row.started_at_ms,
            updated_at_ms: row.updated_at_ms,
            completed_at_ms: row.completed_at_ms,
            last_error: row.last_error,
            command: row.command,
            model: row.model,
            cost_usd: Some(row.cost_usd),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StartGuiRunInput {
    pub run_id: String,
    pub workflow_name: String,
    pub planned_steps: Option<i64>,
    pub command: Option<String>,
    pub repo: Option<String>,
    pub worktree: Option<String>,
    pub model: Option<String>,
}

/// `finish_gui_run`'s positional arguments as one struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FinishGuiRunInput {
    pub run_id: String,
    pub success: bool,
    pub completed_steps: Option<i64>,
    pub error: Option<String>,
    pub cost_usd: Option<f64>,
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    /// The orchestrator's numeric task id (stringified) when the run wraps a submitted task;
    /// the join key for its approval and telemetry totals.
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityRowDto {
    pub id: i64,
    pub ts_ms: i64,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub kind: String,
    pub summary: String,
    pub detail_json: String,
}

impl From<vox_db::ActivityLogEntry> for ActivityRowDto {
    fn from(e: vox_db::ActivityLogEntry) -> Self {
        Self {
            id: e.id,
            ts_ms: e.ts_ms,
            agent_id: e.agent_id,
            session_id: e.session_id,
            kind: e.kind,
            summary: e.summary,
            detail_json: e.detail_json,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_input_round_trips_with_optional_fields_absent() {
        let input: FinishGuiRunInput =
            serde_json::from_value(serde_json::json!({"run_id": "r1", "success": true}))
                .expect("deserialize");
        assert_eq!(input.task_id, None);
        let back: FinishGuiRunInput =
            serde_json::from_value(serde_json::to_value(&input).unwrap()).unwrap();
        assert_eq!(back, input);
    }

    #[test]
    fn activity_dto_copies_every_column() {
        let dto = ActivityRowDto::from(vox_db::ActivityLogEntry {
            id: 7,
            ts_ms: 9,
            agent_id: Some("A1".into()),
            session_id: None,
            kind: "k".into(),
            summary: "s".into(),
            detail_json: "{}".into(),
        });
        assert_eq!(
            (dto.id, dto.ts_ms, dto.agent_id.as_deref()),
            (7, 9, Some("A1"))
        );
        assert_eq!(dto.detail_json, "{}");
    }

    #[test]
    fn plan_node_dto_unwraps_the_session_id_newtype() {
        let dto = PlanNodeDto::from(vox_db::PlanNodeRow {
            plan_session_id: "ps1".into(),
            version: 1,
            node_id: "n1".into(),
            description: "d".into(),
            dependencies_json: "[]".into(),
            execution_policy_json: "{}".into(),
            status: "pending".into(),
            workflow_invocation: None,
        });
        assert_eq!(
            serde_json::to_value(&dto).unwrap()["plan_session_id"],
            "ps1"
        );
    }
}
