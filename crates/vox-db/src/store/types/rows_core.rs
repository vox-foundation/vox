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

