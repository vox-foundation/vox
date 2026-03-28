use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningMode {
    Auto,
    Direct,
    ForcePlan,
    WorkflowOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    ImmediateAct,
    ContinuousOoda,
    SequentialDag,
    HierarchicalHtn,
    WorkflowHandoff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterEvaluation {
    pub strategy: PlanningStrategy,
    pub complexity: u8,
    pub confidence: f32,
    pub workflow_match: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplanTrigger {
    CompilerErrorUnresolved,
    TestFailureNewRegression,
    ScopeDenied,
    LockConflictPersistent,
    MissingCapability,
    ExternalDependencyUnreachable,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    #[serde(default)]
    pub allowed_skills: Vec<String>,
    #[serde(default)]
    pub allowed_action_labels: Vec<String>,
    /// When non-empty, used as the orchestrator file manifest for this plan node instead of the
    /// `Cargo.toml` placeholder in [`crate::planning::executor_bridge`].
    #[serde(default)]
    pub file_manifest: Vec<crate::types::FileAffinity>,
    /// Hints merged at enqueue (e.g. from MCP `submit_goal`); persisted with the plan node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enqueue_hints: Option<crate::types::TaskEnqueueHints>,
    /// Audited override for plan quality gate findings.
    #[serde(default)]
    pub force_risky: bool,
    /// Required when `force_risky` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_risky_reason: Option<String>,
    #[serde(default)]
    pub replan_triggers: Vec<ReplanTrigger>,
    /// Omitted in minimal JSON blobs (e.g. tests, hand-authored policy); defaults to `1` like `ExecutionPolicy::default()`.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    pub timeout_ms: Option<u64>,
}

fn default_max_retries() -> u32 {
    1
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            allowed_skills: vec![],
            allowed_action_labels: vec![
                "read".to_string(),
                "write".to_string(),
                "execute".to_string(),
            ],
            file_manifest: vec![],
            enqueue_hints: None,
            force_risky: false,
            force_risky_reason: None,
            replan_triggers: vec![],
            max_retries: 1,
            timeout_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub node_id: String,
    pub description: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub status: PlanStatus,
    pub execution_policy: ExecutionPolicy,
    pub workflow_invocation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Pending,
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSessionRecord {
    pub plan_session_id: String,
    pub origin_session_id: Option<String>,
    pub goal_text: String,
    pub strategy: PlanningStrategy,
    pub current_version: i64,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersionRecord {
    pub plan_session_id: String,
    pub version: i64,
    pub parent_version: Option<i64>,
    pub trigger_event: Option<String>,
    pub trigger_payload_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningTaskMeta {
    pub plan_session_id: String,
    pub plan_node_id: String,
    pub plan_version: u32,
    pub execution_policy_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_tier: Option<crate::reconstruction::ReconstructionBenchmarkTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_role: Option<crate::reconstruction::AgentExecutionRole>,
}
