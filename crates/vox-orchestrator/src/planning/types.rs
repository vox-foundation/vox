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
    #[serde(default)]
    pub replan_triggers: Vec<ReplanTrigger>,
    pub max_retries: u32,
    pub timeout_ms: Option<u64>,
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
}
