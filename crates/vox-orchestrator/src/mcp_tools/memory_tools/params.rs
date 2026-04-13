use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// MCP arguments: persist a fact into MEMORY.md / optional Codex graph.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemoryStoreParams {
    /// Owning agent id for namespacing.
    pub agent_id: u64,
    /// Fact key.
    pub key: String,
    /// Fact value body.
    pub value: String,
    /// Optional related keys for graph edges.
    pub relations: Option<Vec<String>>,
    /// Optional multimodal URL (e.g. data URI or remote asset).
    pub media_url: Option<String>,
    /// Optional MIME type for the media.
    pub media_type: Option<String>,
}

/// MCP arguments: keyword search against the knowledge graph in VoxDb.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct KnowledgeQueryParams {
    /// Free-text query string.
    pub query: String,
    /// Max nodes to return.
    pub limit: Option<i64>,
}

/// MCP arguments: fetch one MEMORY.md entry by key.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemoryRecallParams {
    /// Key previously stored via [`memory_store`].
    pub key: String,
}

/// MCP arguments: grep-style search across memory logs and MEMORY.md.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemorySearchParams {
    /// Substring or keyword to search for.
    pub query: String,
    /// Optional trace id for retrieval correlation (`X-Vox-Trace-Id` on vector sidecars).
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
}

/// MCP arguments: append one line to today's rolling log file.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemoryLogParams {
    /// Log line content.
    pub entry: String,
}

/// MCP arguments: compact long-term memory for an agent with a summary string.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompactParams {
    /// Agent whose memory shard is compacted.
    pub agent_id: u64,
    /// Replacement summary text.
    pub summary: String,
}

/// MCP arguments: create a new persisted session for an agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionCreateParams {
    /// Agent that owns the session.
    pub agent_id: u64,
}

/// MCP arguments: refer to an existing session by opaque id string.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionIdParams {
    /// Session id returned by create/list tools.
    pub session_id: String,
}

/// MCP arguments: replace old turns with a single summary turn.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionCompactParams {
    /// Target session id.
    pub session_id: String,
    /// Summary text inserted as a synthetic turn.
    pub summary: String,
}

/// MCP arguments: append one chat turn to a session transcript.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionAddTurnParams {
    /// Session receiving the turn.
    pub session_id: String,
    /// Role label (`user`, `assistant`, ...).
    pub role: String,
    /// Turn body.
    pub content: String,
}

/// Serializable session row for MCP list/info tools.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session id string.
    pub id: String,
    /// Owning agent id.
    pub agent_id: u64,
    /// Lifecycle state label.
    pub state: String,
    /// Number of turns stored.
    pub turn_count: usize,
    /// Accumulated token estimate.
    pub total_tokens: usize,
    /// Last activity timestamp (epoch ms).
    pub last_active: u64,
}

/// MCP arguments: read one `user_preferences` row.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreferenceGetParams {
    /// Codex user id namespace.
    pub user_id: String,
    /// Preference key.
    pub key: String,
}

/// MCP arguments: upsert one `user_preferences` row.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreferenceSetParams {
    /// Codex user id namespace.
    pub user_id: String,
    /// Preference key.
    pub key: String,
    /// New value string.
    pub value: String,
}

/// MCP arguments: enumerate preferences, optionally filtered by key prefix.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreferenceListParams {
    /// Codex user id namespace.
    pub user_id: String,
    /// When set, only keys starting with this prefix are returned.
    pub prefix: Option<String>,
}

/// MCP arguments: store a learned pattern row for adaptive tooling.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LearnPatternParams {
    /// Subject user id.
    pub user_id: String,
    /// Pattern classifier string.
    pub pattern_type: String,
    /// High-level grouping label.
    pub category: String,
    /// Free-text description stored in Codex.
    pub description: String,
    /// Optional confidence in `0..1` (defaults in handler).
    pub confidence: Option<f64>,
}

/// MCP arguments: append a behavior observation for the learner pipeline.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BehaviorRecordParams {
    /// Subject user id.
    pub user_id: String,
    /// Short event label (`task.completed`, ...).
    pub event_type: String,
    /// Optional human context string.
    pub context: Option<String>,
    /// Optional JSON metadata blob as string.
    pub metadata: Option<String>,
}

/// MCP arguments: aggregate patterns for one user.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BehaviorSummaryParams {
    /// Subject user id.
    pub user_id: String,
}

/// MCP arguments: insert into `agent_memory` via Codex SQL API.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemorySaveDbParams {
    /// Agent id as string (matches DB column type).
    pub agent_id: String,
    /// Related session id string.
    pub session_id: String,
    /// Memory category / type label.
    pub memory_type: String,
    /// Body text persisted.
    pub content: String,
    /// Optional importance weight.
    pub importance: Option<f64>,
}

/// MCP arguments: query `agent_memory` rows for one agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemoryRecallDbParams {
    /// Agent id filter.
    pub agent_id: String,
    /// Optional type filter (`None` = all types).
    pub memory_type: Option<String>,
    /// Max rows.
    pub limit: Option<i64>,
}
