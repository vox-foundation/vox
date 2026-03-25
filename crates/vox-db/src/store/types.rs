//! Request parameters, row shapes, and errors for [`crate::VoxDb`] (Arca on Turso).
//!
//! **SSOT:** Schema versions live under [`crate::schema`]. Public Codex naming and env vars are
//! defined in ADR 004 and the repo-root `AGENTS.md`—do not restate deployment semantics on
//! every struct; link those docs when adding new persisted fields.
//!
//! **Pattern:** Parameter structs that mirror a `VoxDb::…` method include a rustdoc link to that
//! method so call sites stay discoverable.

use thiserror::Error;

/// Parameters for logging an execution (workflow / activity).
pub struct LogExecutionParams<'a> {
    /// Workflow identifier.
    pub workflow_id: &'a str,
    /// Owning agent id.
    pub agent_id: Option<&'a str>,
    /// Skill identifier.
    pub skill_id: Option<&'a str>,
    /// Activity or step name.
    pub activity_name: &'a str,
    /// Outcome label (e.g. `ok`, `error`).
    pub status: &'a str,
    /// Attempt number (1-based).
    pub attempt: u32,
    /// Wall-clock latency in milliseconds.
    pub duration_ms: i64,
    /// Output payload size in bytes.
    pub output_size: i64,
    /// Serialized input payload, if any.
    pub input: Option<&'a [u8]>,
    /// Serialized output payload, if any.
    pub output: Option<&'a [u8]>,
    /// Error message, if failed.
    pub error: Option<&'a str>,
    /// Optional JSON or opaque options string.
    pub options: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::save_memory`].
#[derive(Debug, Clone)]
pub struct SaveMemoryParams<'a> {
    /// Owning agent id.
    pub agent_id: &'a str,
    /// Conversation or session id.
    pub session_id: &'a str,
    /// Memory category / type label.
    pub memory_type: &'a str,
    /// Stored text content.
    pub content: &'a str,
    /// Optional JSON metadata.
    pub metadata: Option<&'a str>,
    /// Relative importance score.
    pub importance: f64,
    /// Optional VCS snapshot correlation id.
    pub vcs_snapshot_id: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::log_interaction`].
#[derive(Debug, Clone)]
pub struct LogInteractionParams<'a> {
    /// Session id for the interaction.
    pub session_id: &'a str,
    /// End-user id when known.
    pub user_id: Option<&'a str>,
    /// User prompt text.
    pub prompt: &'a str,
    /// Model response text.
    pub response: &'a str,
    /// Model identifier or version string.
    pub model_version: &'a str,
    /// Wall-clock latency in milliseconds, if recorded.
    pub latency_ms: Option<i64>,
    /// Token usage, if recorded.
    pub token_count: Option<i64>,
}

/// Parameters for [`crate::VoxDb::save_snippet`].
#[derive(Debug, Clone)]
pub struct SaveSnippetParams<'a> {
    /// Programming language tag.
    pub language: &'a str,
    /// Short title.
    pub title: &'a str,
    /// Source code body.
    pub code: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional comma-separated or JSON tags.
    pub tags: Option<&'a str>,
    /// Author user id, if any.
    pub author_id: Option<&'a str>,
    /// External reference (URL, path, etc.).
    pub source_ref: Option<&'a str>,
    /// Reserved for future embedding linkage.
    pub embedding_ref: Option<&'a str>,
}

/// Parameters for [`crate::VoxDb::publish_artifact`].
#[derive(Debug, Clone)]
pub struct PublishArtifactParams<'a> {
    /// Stable artifact id.
    pub id: &'a str,
    /// Type discriminator (e.g. model, dataset).
    pub artifact_type: &'a str,
    /// Human-readable name.
    pub name: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Publishing user id.
    pub author_id: &'a str,
    /// CAS hash of artifact bytes.
    pub content_hash: &'a str,
    /// Semantic version string.
    pub version: &'a str,
    /// Optional tags string.
    pub tags: Option<&'a str>,
    /// Lifecycle status (e.g. draft, published).
    pub status: &'a str,
}

/// Parameters for [`crate::VoxDb::register_agent`].
#[derive(Debug, Clone)]
pub struct RegisterAgentParams<'a> {
    /// Agent id (primary key).
    pub id: &'a str,
    /// Display name.
    pub name: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// System prompt text, if any.
    pub system_prompt: Option<&'a str>,
    /// JSON tool manifest or config, if any.
    pub tools: Option<&'a str>,
    /// JSON model configuration, if any.
    pub model_config: Option<&'a str>,
    /// Owning user id, if any.
    pub owner_id: Option<&'a str>,
    /// Agent definition version.
    pub version: &'a str,
    /// Whether the agent is visible to other users.
    pub is_public: bool,
}

/// Store operation failure (Turso, not-found, or serialization).
#[derive(Error, Debug)]
pub enum StoreError {
    /// Generic database-layer message.
    #[error("Database error: {0}")]
    Db(String),
    /// Underlying Turso / libSQL error.
    #[error(transparent)]
    Turso(#[from] turso::Error),
    /// Local filesystem I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Requested row or binding was missing.
    #[error("Not found: {0}")]
    NotFound(String),
    /// JSON or other serialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),
    /// Invalid UTF-8 in blob payload.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// Database used the historical multi-row `schema_version` chain; baseline-V1 only supports version 1.
    #[error(
        "legacy Arca schema chain detected (schema_version max={max_version}): export with `vox codex export-legacy`, initialize a fresh Codex database, then `vox codex import-legacy`"
    )]
    LegacySchemaChain {
        /// Highest `schema_version.version` present before baseline migration.
        max_version: i64,
    },
}

/// One row from `execution_log`.
#[derive(Debug, Clone)]
pub struct ExecutionEntry {
    /// Row id.
    pub id: i64,
    /// Activity name.
    pub activity_name: String,
    /// Status label.
    pub status: String,
    /// Attempt number.
    pub attempt: u32,
    /// Error text when status indicates failure.
    pub error: Option<String>,
    /// SQLite `datetime` string.
    pub created_at: String,
}

/// One row from `scheduled` (pending or historical).
#[derive(Debug, Clone)]
pub struct ScheduledEntry {
    /// Row id.
    pub id: i64,
    /// Content hash of the function to invoke.
    pub function_hash: String,
    /// Serialized call arguments.
    pub args: Option<Vec<u8>>,
    /// ISO-like run time or cron anchor.
    pub run_at: String,
    /// Optional cron expression.
    pub cron_expr: Option<String>,
}

/// Registered UI or service component metadata.
#[derive(Debug, Clone)]
pub struct ComponentEntry {
    /// Component name.
    pub name: String,
    /// Namespace path.
    pub namespace: String,
    /// Version string.
    pub version: String,
    /// Optional description.
    pub description: Option<String>,
}

/// One row from `memories`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryEntry {
    /// Row id.
    pub id: i64,
    /// Agent that owns this memory.
    pub agent_id: String,
    /// Session scope.
    pub session_id: String,
    /// Memory type label.
    pub memory_type: String,
    /// Text content.
    pub content: String,
    /// Optional JSON metadata.
    pub metadata: Option<String>,
    /// Importance score.
    pub importance: f64,
    /// Creation timestamp string.
    pub created_at: String,
}

/// Vector embedding row from `embeddings`.
#[derive(Debug, Clone)]
pub struct EmbeddingEntry {
    /// Row id.
    pub id: i64,
    /// Source object type, if set.
    pub source_type: Option<String>,
    /// Source object id.
    pub source_id: String,
    /// Vector dimensionality.
    pub dim: i64,
    /// Optional JSON metadata.
    pub metadata: Option<String>,
}

/// Learned preference or habit pattern.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearnedPatternEntry {
    /// Row id.
    pub id: i64,
    /// User or agent id the pattern belongs to.
    pub user_id: String,
    /// Pattern classifier.
    pub pattern_type: String,
    /// Category bucket.
    pub category: String,
    /// Human-readable description.
    pub description: String,
    /// Model confidence in `[0, 1]`.
    pub confidence: f64,
    /// Optional VCS snapshot correlation.
    pub vcs_snapshot_id: Option<String>,
}

/// Analytics event from `behavior_events`.
#[derive(Debug, Clone)]
pub struct BehaviorEventEntry {
    /// Row id.
    pub id: i64,
    /// Subject user id.
    pub user_id: String,
    /// Event type label.
    pub event_type: String,
    /// Free-form context (e.g. command string).
    pub context: Option<String>,
    /// JSON metadata blob.
    pub metadata: Option<String>,
    /// Event timestamp string.
    pub created_at: String,
}

/// Aggregated CLI command statistics for a user.
#[derive(Debug, Clone)]
pub struct CommandFrequencyEntry {
    /// Command or context key.
    pub command: String,
    /// Total invocations.
    pub count: i64,
    /// Count inferred as successful from metadata heuristics.
    pub success_count: i64,
    /// Average duration in ms when present in metadata.
    pub avg_duration_ms: Option<f64>,
}

/// Prompt/response pair plus optional human feedback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingPair {
    /// User prompt.
    pub prompt: String,
    /// Model response.
    pub response: String,
    /// Optional numeric rating.
    pub rating: Option<i64>,
    /// Preferred or corrected response text.
    pub correction: Option<String>,
    /// Feedback channel label.
    pub feedback_type: String,
}

/// Row from `users`.
#[derive(Debug, Clone)]
pub struct UserEntry {
    /// User id.
    pub id: String,
    /// Display name.
    pub display_name: String,
    /// Email when stored.
    pub email: Option<String>,
    /// Avatar URL.
    pub avatar_url: Option<String>,
    /// Role label.
    pub role: String,
}

/// Agent definition as stored in `agents` (by name ordering in list APIs).
#[derive(Debug, Clone)]
pub struct AgentDefEntry {
    /// Agent name.
    pub name: String,
    /// Definition version.
    pub version: String,
    /// Description text.
    pub description: Option<String>,
    /// System prompt.
    pub system_prompt: Option<String>,
    /// Tools JSON.
    pub tools: Option<String>,
    /// Model config JSON.
    pub model_config: Option<String>,
    /// Public visibility flag.
    pub is_public: bool,
}

/// Code snippet catalog row.
#[derive(Debug, Clone)]
pub struct SnippetEntry {
    /// Row id.
    pub id: i64,
    /// Language tag.
    pub language: String,
    /// Title.
    pub title: String,
    /// Source code.
    pub code: String,
    /// Description.
    pub description: Option<String>,
    /// Tags string.
    pub tags: Option<String>,
}

/// One matching package from `search_packages`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageSearchResult {
    /// Package name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// Description.
    pub description: Option<String>,
    /// Author string.
    pub author: Option<String>,
    /// SPDX or license label.
    pub license: Option<String>,
}

/// Artifact registry row (search/list).
#[derive(Debug, Clone)]
pub struct ArtifactEntry {
    /// Artifact name.
    pub name: String,
    /// Type discriminator.
    pub artifact_type: String,
    /// Version string.
    pub version: String,
    /// Author user id.
    pub author_id: String,
    /// Download counter.
    pub downloads: i64,
    /// Average star or quality rating.
    pub avg_rating: f64,
    /// Lifecycle status.
    pub status: String,
    /// Description text.
    pub description: Option<String>,
}

/// Published skill: manifest JSON plus markdown body.
#[derive(Debug, Clone)]
pub struct SkillManifestEntry {
    /// Skill id.
    pub id: String,
    /// Skill version.
    pub version: String,
    /// Serialized manifest JSON.
    pub manifest_json: String,
    /// Skill markdown documentation.
    pub skill_md: String,
}

/// Minimal knowledge graph node (id + label).
#[derive(Debug, Clone)]
pub struct KnowledgeNodeSummary {
    /// Node id.
    pub id: String,
    /// Short label.
    pub label: String,
}

/// Orchestrator builder session payload.
#[derive(Debug, Clone)]
pub struct BuilderSessionEntry {
    /// Session id.
    pub id: String,
    /// Opaque JSON payload.
    pub payload_json: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// One turn in an agent session transcript.
#[derive(Debug, Clone)]
pub struct SessionTurnEntry {
    /// Row id.
    pub id: i64,
    /// Session id.
    pub session_id: String,
    /// Turn payload JSON.
    pub payload_json: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Typed stream event for SSE-style feeds.
#[derive(Debug, Clone)]
pub struct TypedStreamEventEntry {
    /// Row id.
    pub id: i64,
    /// Logical stream id.
    pub stream_id: String,
    /// Event JSON body.
    pub payload_json: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// Mens / review row for a target entity.
#[derive(Debug, Clone)]
pub struct ReviewEntry {
    /// Row id.
    pub id: i64,
    /// Entity under review.
    pub target_id: String,
    /// Review kind label.
    pub review_kind: String,
    /// Review payload JSON.
    pub payload_json: String,
    /// Creation timestamp.
    pub created_at: String,
}

/// One row from `codex_change_log` (Codex reactivity / SSE).
#[derive(Debug, Clone)]
pub struct CodexChangeLogEntry {
    /// Monotonic change id.
    pub id: i64,
    /// Subscription topic.
    pub topic: String,
    /// Affected entity type, if any.
    pub entity_kind: Option<String>,
    /// Affected entity id, if any.
    pub entity_id: Option<String>,
    /// Change operation label.
    pub change_kind: String,
    /// Optional JSON details.
    pub payload_json: Option<String>,
    /// Insertion timestamp.
    pub created_at: String,
}

/// Parameters for `record_skill_execution`.
#[derive(Debug, Clone)]
pub struct SkillExecutionParams<'a> {
    pub skill_id: &'a str,
    pub version: &'a str,
    pub session_id: Option<&'a str>,
    pub workflow_id: Option<&'a str>,
    pub agent_id: Option<&'a str>,
    pub status: &'a str,
    pub duration_ms: i64,
    pub input_hash: Option<&'a str>,
    pub output_size: i64,
    pub error_kind: Option<&'a str>,
    pub reflection_score: Option<f64>,
}

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

/// A single row from `a2a_messages`.
#[derive(Debug, Clone)]
pub struct A2AMessageRow {
    pub id: i64,
    pub sender_id: String,
    pub receiver_id: String,
    pub msg_type: String,
    pub payload_json: String,
    pub status: String,
    pub created_at_ms: i64,
}

/// A single row from `agent_events`.
#[derive(Debug, Clone)]
pub struct AgentEventRow {
    pub id: i64,
    pub agent_id: String,
    pub event_type: String,
    pub repository_id: String,
    pub metadata_json: String,
    pub created_at_ms: i64,
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

/// A single row from `agent_sessions`.
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub session_id: String,
    pub repository_id: String,
    pub agent_id: String,
    pub model_id: String,
    pub task_snapshot_json: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
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
