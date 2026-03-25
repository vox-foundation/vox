/// A report on the reliability of a given skill.
#[derive(Debug, Clone)]
pub struct SkillReliabilityReport {
    pub skill_id: String,
    pub total: u64,
    pub success_rate: f64,
    pub median_duration_ms: i64,
    pub p99_duration_ms: i64,
    pub last_error_kind: Option<String>,
}

/// One row from `endpoint_reliability`.
#[derive(Debug, Clone)]
pub struct EndpointReliabilityEntry {
    pub endpoint_url: String,
    pub model_id: String,
    pub total_requests: u64,
    pub hallucination_proxy_ewma: f64,
    pub contradiction_ratio_ewma: f64,
    pub infra_failure_ewma: f64,
    pub rate_limit_hits: u64,
    pub timeout_hits: u64,
    pub updated_at_ms: i64,
}

/// A single training data row exported from the database for Mens.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorpusRow {
    pub category: String,
    pub language: String,
    pub source_id: String,
    pub instruction: String,
    pub output: String,
    pub quality_score: f64,
    pub tags: Vec<String>,
}

impl CorpusRow {
    /// Serialize the row to a plain JSON line for streaming.
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}
/// A single row from `skill_executions` — DB DTO (never used in business logic).
#[derive(Debug, Clone)]
pub struct SkillExecutionRow {
    /// Primary key of this execution row.
    pub id: i64,
    /// Skill identifier (foreign key to `skill_manifests`).
    pub skill_id: String,
    /// Version string of the skill at execution time.
    pub version: String,
    /// MCP/agent session that triggered the execution, if any.
    pub session_id: Option<String>,
    /// Workflow execution identifier, if any.
    pub workflow_id: Option<String>,
    /// Agent that ran the skill.
    pub agent_id: Option<String>,
    /// `"ok"` | `"error"` | `"timeout"`.
    pub status: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// SHA3 hash of the input payload (for deduplication).
    pub input_hash: Option<String>,
    /// Byte size of the output payload.
    pub output_size: Option<i64>,
    /// Short error category when status != `"ok"`.
    pub error_kind: Option<String>,
    /// Socrates reflection score 0..=100, if evaluated.
    pub reflection_score: Option<f64>,
    /// ISO-8601 creation timestamp (UTC).
    pub created_at: String,
}

/// A single row from `workflow_executions` — DB DTO.
#[derive(Debug, Clone)]
pub struct WorkflowExecutionRow {
    /// Stable workflow identifier.
    pub workflow_id: String,
    /// `"running"` | `"ok"` | `"error"`.
    pub status: String,
    /// Number of planned steps in this execution.
    pub step_count: i64,
    /// Number of steps that produced errors.
    pub error_count: i64,
    /// ISO-8601 start timestamp.
    pub started_at: String,
    /// ISO-8601 finish timestamp; `None` when still running.
    pub finished_at: Option<String>,
}

/// A single row from `agent_questions`.
#[derive(Debug, Clone)]
pub struct QuestionRow {
    pub id: i64,
    pub asker_id: String,
    pub responder_id: String,
    pub question_text: String,
    pub answer_text: Option<String>,
    pub status: String,
    pub created_at_ms: i64,
}

/// One unacknowledged row from `a2a_messages` as returned by [`crate::VoxDb::poll_a2a_inbox`].
#[derive(Debug, Clone)]
pub struct A2AMessageRow {
    pub id: i64,
    pub message_uuid: String,
    pub sender_agent: String,
    pub receiver_agent: String,
    pub msg_type: String,
    pub payload: String,
    pub priority: i64,
    pub thread_id: Option<String>,
    pub acknowledged: bool,
    pub created_at: String,
    pub repository_id: String,
}

/// A single row from `agent_events` (telemetry / gamify audit).
#[derive(Debug, Clone)]
pub struct AgentEventRow {
    pub id: i64,
    pub agent_id: String,
    pub event_type: String,
    pub payload_json: Option<String>,
    pub cli_version: Option<String>,
    pub timestamp: String,
}

/// A single row from `benchmark_events`.
#[derive(Debug, Clone)]
pub struct BenchmarkEventRow {
    pub id: i64,
    pub benchmark_id: String,
    pub model_id: String,
    pub score: f64,
    pub latency_ms: i64,
    pub metadata_json: String,
    pub created_at_ms: i64,
}

/// Row shape for `agent_sessions` (MCP/orchestrator session lifecycle in Codex).
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub agent_id: String,
    pub agent_name: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub task_snapshot: Option<String>,
    pub context_summary: Option<String>,
}

/// A single row from `agent_session_events`.
#[derive(Debug, Clone)]
pub struct SessionEventRow {
    pub id: i64,
    pub session_id: String,
    pub event_type: String,
    pub payload_json: String,
    pub created_at_ms: i64,
}

/// A single row from `build_runs`.
#[derive(Debug, Clone)]
pub struct BuildRunRow {
    pub id: i64,
    pub build_id: String,
    pub repository_id: String,
    pub status: String,
    pub duration_ms: i64,
    pub metadata_json: String,
    pub created_at_ms: i64,
}

/// A single row from `build_crate_samples`.
#[derive(Debug, Clone)]
pub struct CrateSampleRow {
    pub id: i64,
    pub build_id: String,
    pub crate_name: String,
    pub duration_ms: i64,
    pub status: String,
    pub created_at_ms: i64,
}

/// A single row from `cloud_dispatch_log`.
#[derive(Debug, Clone)]
pub struct CloudDispatchRow {
    pub id: i64,
    pub provider: String,
    pub endpoint_type: String,
    pub status: String,
    pub duration_ms: i64,
    pub metadata_json: String,
    pub created_at_ms: i64,
}

/// A single row from `model_throughput_profiles`.
#[derive(Debug, Clone)]
pub struct ThroughputProfileRow {
    pub id: i64,
    pub model_id: String,
    pub hardware_id: String,
    pub tokens_per_sec: f64,
    pub latency_p50_ms: i64,
    pub updated_at_ms: i64,
}

/// One row from `publication_manifests`.
#[derive(Debug, Clone)]
pub struct PublicationManifestRow {
    pub publication_id: String,
    pub content_type: String,
    pub source_ref: Option<String>,
    pub title: String,
    pub author: String,
    pub abstract_text: Option<String>,
    pub body_markdown: String,
    pub citations_json: Option<String>,
    pub metadata_json: Option<String>,
    pub content_sha3_256: String,
    pub version: i64,
    pub state: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// One row from `scholarly_submissions`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScholarlySubmissionRow {
    pub id: i64,
    pub publication_id: String,
    pub content_sha3_256: String,
    pub adapter: String,
    pub external_submission_id: String,
    pub status: String,
    pub submitted_at_ms: i64,
    pub updated_at_ms: i64,
    pub response_fingerprint: Option<String>,
    pub metadata_json: Option<String>,
}

/// A single row from `local_train_log`.
#[derive(Debug, Clone)]
pub struct LocalTrainRow {
    pub id: i64,
    pub run_id: String,
    pub model_id: String,
    pub dataset_id: String,
    pub status: String,
    pub loss: Option<f64>,
    pub step: i64,
    pub created_at_ms: i64,
}

/// A single row from `arca_warnings`.
#[derive(Debug, Clone)]
pub struct WarningRow {
    pub id: i64,
    pub warning_type: String,
    pub message: String,
    pub metadata_json: String,
    pub created_at_ms: i64,
}

/// One row from `plan_sessions`.
#[derive(Debug, Clone)]
pub struct PlanSessionRow {
    pub plan_session_id: String,
    pub origin_session_id: Option<String>,
    pub goal_text: String,
    pub strategy: String,
    pub current_version: i64,
    pub status: String,
}

/// One row from `plan_versions`.
#[derive(Debug, Clone)]
pub struct PlanVersionRow {
    pub plan_session_id: String,
    pub version: i64,
    pub parent_version: Option<i64>,
    pub trigger_event: Option<String>,
    pub trigger_payload_json: Option<String>,
}

/// One row from `plan_nodes`.
#[derive(Debug, Clone)]
pub struct PlanNodeRow {
    pub plan_session_id: String,
    pub version: i64,
    pub node_id: String,
    pub description: String,
    pub dependencies_json: String,
    pub execution_policy_json: String,
    pub status: String,
    pub workflow_invocation: Option<String>,
}
