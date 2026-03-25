//! MCP tools: episodic memory, knowledge graph queries, and session-scoped recall.
//!
//! **Agents calling these tools:** Use stable, namespaced `key` strings for writes; pair
//! `MemoryStoreParams::agent_id` with the orchestrator’s agent id when available. Cap graph
//! fan-out with [`KnowledgeQueryParams::limit`](crate::memory::KnowledgeQueryParams::limit) so tool responses stay bounded. For authoritative
//! Codex semantics, see repo-root `AGENTS.md` §2.2.1 and ADR 004—not every tool doc repeats the
//! data-plane glossary.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ServerState, ToolResult};

fn memory_config_for_state(state: &ServerState) -> vox_orchestrator::MemoryConfig {
    state.orchestrator_config.memory.clone()
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Memory tool handlers
// ---------------------------------------------------------------------------

/// Persist a key-value fact to long-term memory (MEMORY.md + VoxDb).
pub async fn memory_store(state: &ServerState, params: MemoryStoreParams) -> String {
    let config = memory_config_for_state(state);
    match vox_orchestrator::MemoryManager::new(config) {
        Ok(mut mgr) => {
            if let Some(ref db) = state.db {
                mgr.set_db(db.clone());
            }
            let rels = params.relations.unwrap_or_default();
            let rel_strs: Vec<&str> = rels.iter().map(|s| s.as_str()).collect();
            match mgr.persist_fact(
                vox_orchestrator::AgentId(params.agent_id),
                &params.key,
                &params.value,
                &rel_strs,
                params.media_url.as_deref(),
                params.media_type.as_deref(),
            ) {
                Ok(()) => ToolResult::ok(format!("Stored '{}' = '{}'", params.key, params.value))
                    .to_json(),
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
        Err(e) => ToolResult::<String>::err(format!("memory init failed: {e}")).to_json(),
    }
}

/// Retrieve a fact from long-term memory by key.
///
/// Uses the same [`MemoryConfig`] as [`memory_store`] (`state.orchestrator_config.memory`).
/// When `state.db` is set, it is attached for parity with store; [`MemoryManager::recall`] is still
/// file/cache-first and does **not** query Codex on miss yet.
pub async fn memory_recall(state: &ServerState, params: MemoryRecallParams) -> String {
    let config = memory_config_for_state(state);
    match vox_orchestrator::MemoryManager::new(config) {
        Ok(mut mgr) => {
            if let Some(ref db) = state.db {
                mgr.set_db(db.clone());
            }
            match mgr.recall(&params.key) {
                Ok(Some(val)) => ToolResult::ok(val).to_json(),
                Ok(None) => {
                    ToolResult::<String>::err(format!("Key '{}' not found", params.key)).to_json()
                }
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
        Err(e) => ToolResult::<String>::err(format!("memory init failed: {e}")).to_json(),
    }
}

/// Search memory (daily logs + MEMORY.md) by keyword.
pub async fn memory_search(state: &ServerState, params: MemorySearchParams) -> String {
    let config = memory_config_for_state(state);
    match vox_orchestrator::MemoryManager::new(config) {
        Ok(mgr) => match mgr.search(&params.query) {
            Ok(hits) => {
                if hits.is_empty() {
                    ToolResult::ok("No results found.".to_string()).to_json()
                } else {
                    let formatted = hits
                        .iter()
                        .map(|h| format!("[{}:{}] {}", h.source, h.line, h.content))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ToolResult::ok(formatted).to_json()
                }
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
        Err(e) => ToolResult::<String>::err(format!("memory init failed: {e}")).to_json(),
    }
}

/// Append an entry to today's daily memory log.
///
/// Uses the same orchestrator-scoped memory paths as other memory tools.
pub async fn memory_daily_log(state: &ServerState, params: MemoryLogParams) -> String {
    let config = memory_config_for_state(state);
    match vox_orchestrator::MemoryManager::new(config) {
        Ok(mut mgr) => {
            if let Some(ref db) = state.db {
                mgr.set_db(db.clone());
            }
            match mgr.log(&params.entry) {
                Ok(()) => ToolResult::ok("Entry logged to daily memory.".to_string()).to_json(),
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
        Err(e) => ToolResult::<String>::err(format!("memory init failed: {e}")).to_json(),
    }
}

/// List all memory keys from MEMORY.md.
pub async fn memory_list_keys(state: &ServerState) -> String {
    let config = memory_config_for_state(state);
    match vox_orchestrator::MemoryManager::new(config) {
        Ok(mgr) => match mgr.list_keys() {
            Ok(keys) => ToolResult::ok(keys).to_json(),
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
        Err(e) => ToolResult::<String>::err(format!("memory init failed: {e}")).to_json(),
    }
}

/// Query the knowledge graph by keyword.
pub async fn knowledge_query(state: &ServerState, params: KnowledgeQueryParams) -> String {
    if let Some(ref db) = state.db {
        let limit = params.limit.unwrap_or(10);
        match db.query_knowledge_nodes(&params.query, limit).await {
            Ok(nodes) => {
                if nodes.is_empty() {
                    ToolResult::ok("No related knowledge nodes found.".to_string()).to_json()
                } else {
                    let formatted = nodes
                        .into_iter()
                        .map(|(id, ntype, label)| format!("[{}] {} ({})", id, label, ntype))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ToolResult::ok(formatted).to_json()
                }
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        }
    } else {
        ToolResult::<String>::err("VoxDb not attached to MCP server.".to_string()).to_json()
    }
}

// ---------------------------------------------------------------------------
// Compaction tool handlers
// ---------------------------------------------------------------------------

/// Get current context window usage and compaction recommendation (async).
pub async fn compaction_status(
    state: &ServerState,
    params: crate::context::ContextBudgetParams,
) -> String {
    let orch = &state.orchestrator;
    let id = vox_orchestrator::AgentId(params.agent_id);
    let handle = orch.budget_handle();
    let budget_lock = handle.read().unwrap();
    if let Some(budget) = budget_lock.check_budget(id) {
        let engine = vox_orchestrator::CompactionEngine::default();
        let should = engine.should_compact(budget.tokens_used);
        ToolResult::ok(format!(
            "Agent {}: {}/{} tokens used. Compaction recommended: {}. Strategy: {}",
            params.agent_id,
            budget.tokens_used,
            budget.model_max_tokens,
            should,
            vox_orchestrator::CompactionStrategy::default()
        ))
        .to_json()
    } else {
        ToolResult::ok(format!(
            "Agent {}: no budget tracked. Compaction engine ready with {}k token limit.",
            params.agent_id,
            vox_orchestrator::CompactionConfig::default().max_context_tokens / 1000
        ))
        .to_json()
    }
}

// ---------------------------------------------------------------------------
// Session tool handlers
// ---------------------------------------------------------------------------

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

/// Create a new session for an agent (async).
pub async fn session_create(state: &ServerState, params: SessionCreateParams) -> String {
    let mut mgr = state.session_manager.lock().await;
    match mgr.create(vox_orchestrator::AgentId(params.agent_id)) {
        Ok(id) => ToolResult::ok(id).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// List all sessions (async).
pub async fn session_list(state: &ServerState) -> String {
    let mgr = state.session_manager.lock().await;
    let sessions: Vec<SessionInfo> = mgr
        .list_sessions()
        .iter()
        .map(|s| SessionInfo {
            id: s.id.clone(),
            agent_id: s.agent_id.0,
            state: s.state.to_string(),
            turn_count: s.turn_count,
            total_tokens: s.total_tokens,
            last_active: s.last_active,
        })
        .collect();
    ToolResult::ok(sessions).to_json()
}

/// Reset a session (clear history, keep metadata) (async).
pub async fn session_reset(state: &ServerState, params: SessionIdParams) -> String {
    let mut mgr = state.session_manager.lock().await;
    match mgr.reset(&params.session_id) {
        Ok(cleared) => ToolResult::ok(format!(
            "Session '{}' reset: {} turns cleared.",
            params.session_id, cleared
        ))
        .to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Compact a session with a summary (async).
pub async fn session_compact(state: &ServerState, params: SessionCompactParams) -> String {
    let mut mgr = state.session_manager.lock().await;
    match mgr.compact(&params.session_id, &params.summary) {
        Ok(removed) => ToolResult::ok(format!(
            "Session '{}' compacted: {} turns replaced with summary.",
            params.session_id, removed
        ))
        .to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Get info about a specific session (async).
pub async fn session_info(state: &ServerState, params: SessionIdParams) -> String {
    let mgr = state.session_manager.lock().await;
    match mgr.get(&params.session_id) {
        Some(s) => ToolResult::ok(SessionInfo {
            id: s.id.clone(),
            agent_id: s.agent_id.0,
            state: s.state.to_string(),
            turn_count: s.turn_count,
            total_tokens: s.total_tokens,
            last_active: s.last_active,
        })
        .to_json(),
        None => ToolResult::<String>::err(format!("Session '{}' not found.", params.session_id))
            .to_json(),
    }
}

/// Cleanup archived sessions (async).
pub async fn session_cleanup(state: &ServerState) -> String {
    let mut mgr = state.session_manager.lock().await;
    mgr.tick_lifecycle();
    match mgr.cleanup() {
        Ok(n) => ToolResult::ok(format!("{n} sessions cleaned up.")).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

// ---------------------------------------------------------------------------
// Preference & Behavioral Learning tool handlers
// ---------------------------------------------------------------------------

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

/// Get a user preference from VoxDb.
pub async fn preference_get(state: &ServerState, params: PreferenceGetParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .get_user_preference(&params.user_id, &params.key)
            .await
        {
            Ok(Some(val)) => ToolResult::ok(val).to_json(),
            Ok(None) => ToolResult::<String>::err(format!("Preference '{}' not found", params.key))
                .to_json(),
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Set a user preference in VoxDb.
pub async fn preference_set(state: &ServerState, params: PreferenceSetParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .set_user_preference(&params.user_id, &params.key, &params.value)
            .await
        {
            Ok(()) => {
                ToolResult::ok(format!("Set '{}' = '{}'", params.key, params.value)).to_json()
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// List user preferences from VoxDb, optionally filtered by key prefix.
pub async fn preference_list(state: &ServerState, params: PreferenceListParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .list_user_preferences(&params.user_id, params.prefix.as_deref())
            .await
        {
            Ok(prefs) => {
                let lines: Vec<String> =
                    prefs.iter().map(|(k, v)| format!("{k} = {v}")).collect();
                ToolResult::ok(lines.join("\n")).to_json()
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Store a learned behavior pattern in VoxDb.
pub async fn learn_pattern(state: &ServerState, params: LearnPatternParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .store_learned_pattern(
                &params.user_id,
                &params.pattern_type,
                &params.category,
                &params.description,
                params.confidence.unwrap_or(0.5),
                None,
            )
            .await
        {
            Ok(id) => ToolResult::ok(format!("Pattern stored with id={id}")).to_json(),
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Record a user behavior event and get triggered suggestions.
pub async fn behavior_record(state: &ServerState, params: BehaviorRecordParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => {
            let learner = db.learner();
            match learner
                .observe(
                    &params.user_id,
                    &params.event_type,
                    params.context.as_deref(),
                    params.metadata.as_deref(),
                    None,
                )
                .await
            {
                Ok(suggestions) => {
                    if suggestions.is_empty() {
                        ToolResult::ok("Event recorded. No new patterns detected.".to_string())
                            .to_json()
                    } else {
                        let lines: Vec<String> = suggestions
                            .iter()
                            .map(|s| {
                                format!(
                                    "[{:.0}%] {}: {}",
                                    s.confidence * 100.0,
                                    s.title,
                                    s.description
                                )
                            })
                            .collect();
                        ToolResult::ok(format!(
                            "Event recorded. New patterns:\n{}",
                            lines.join("\n")
                        ))
                        .to_json()
                    }
                }
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
    }
}

/// Analyze all behavior events for a user and return learned patterns summary.
pub async fn behavior_summary(state: &ServerState, params: BehaviorSummaryParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => {
            let learner = db.learner();
            match learner.analyze(&params.user_id, None).await {
                Ok(patterns) => {
                    if patterns.is_empty() {
                        ToolResult::ok("No patterns detected yet.".to_string()).to_json()
                    } else {
                        let lines: Vec<String> = patterns
                            .iter()
                            .map(|p| {
                                format!(
                                    "[{:.0}%] {} / {} — {}",
                                    p.confidence * 100.0,
                                    p.pattern_type,
                                    p.category,
                                    p.description
                                )
                            })
                            .collect();
                        ToolResult::ok(lines.join("\n")).to_json()
                    }
                }
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
    }
}

/// Persist a fact directly into VoxDb agent_memory table.
pub async fn memory_save_db(state: &ServerState, params: MemorySaveDbParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .save_memory(vox_db::MemoryParams {
                agent_id: &params.agent_id,
                session_id: &params.session_id,
                memory_type: &params.memory_type,
                content: &params.content,
                metadata: None,
                importance: params.importance.unwrap_or(1.0),
                vcs_snapshot_id: None,
            })
            .await
        {
            Ok(id) => ToolResult::ok(format!("Memory saved with id={id}")).to_json(),
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Recall facts from VoxDb agent_memory table.
pub async fn memory_recall_db(state: &ServerState, params: MemoryRecallDbParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .recall_memory(
                &params.agent_id,
                params.memory_type.as_deref(),
                params.limit.unwrap_or(20),
                None,
            )
            .await
        {
            Ok(entries) => {
                if entries.is_empty() {
                    ToolResult::ok("No memories found.".to_string()).to_json()
                } else {
                    let lines: Vec<String> = entries
                        .iter()
                        .map(|e| format!("[{}] [{:.2}] {}", e.memory_type, e.importance, e.content))
                        .collect();
                    ToolResult::ok(lines.join("\n")).to_json()
                }
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

#[cfg(test)]
mod memory_config_tests {
    use super::memory_config_for_state;
    use crate::server::ServerState;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use vox_orchestrator::{
        AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
    };
    use vox_repository::{RepoCapabilities, RepositoryContext};
    use vox_skills::new_registry_arc;

    #[test]
    fn memory_config_for_state_matches_orchestrator_memory() {
        let custom = std::env::temp_dir().join("vox_mcp_memory_config_test");
        let mut cfg = OrchestratorConfig::default();
        cfg.memory.log_dir = custom.clone();
        cfg.memory.memory_md_path = custom.join("MEMORY.md");
        let orch_cfg = cfg.clone();
        let groups = AffinityGroupRegistry::new(vec![]);
        let session_cfg = SessionConfig {
            persist: false,
            sessions_dir: std::env::temp_dir().join("vox-mcp-test-sessions"),
            ..SessionConfig::default()
        };
        let session_manager = SessionManager::new(session_cfg).expect("session manager");
        let repository = RepositoryContext {
            root: PathBuf::from("."),
            git_root: None,
            repository_id: "test".into(),
            origin_url: None,
            capabilities: RepoCapabilities {
                vox_project: false,
                cargo_workspace: false,
                cargo_package: false,
                node_workspace: false,
                python_project: false,
                go_module: false,
                git: false,
            },
            has_vox_agents_dir: false,
            vox_toml: None,
        };
        let state = ServerState::test_stub(
            cfg.clone(),
            repository,
            Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
            Arc::new(Mutex::new(session_manager)),
            new_registry_arc(),
        );
        let mc = memory_config_for_state(&state);
        assert_eq!(mc.log_dir, custom);
        assert_eq!(mc.memory_md_path, custom.join("MEMORY.md"));
    }
}
