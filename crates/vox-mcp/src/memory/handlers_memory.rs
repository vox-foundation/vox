use super::config::memory_config_for_state;
use super::params::{
    KnowledgeQueryParams, MemoryLogParams, MemoryRecallParams, MemorySearchParams,
    MemoryStoreParams,
};
use super::retrieval::{RetrievalTriggerMode, run_retrieval_bundle};
use crate::{ServerState, ToolResult};

const REM_MEMORY_VOXDB: &str =
    "Attach VoxDb (`VOX_DB_PATH` / `VOX_DB_URL`) to the MCP server for knowledge-graph queries.";
const REM_MEMORY_INIT: &str = "Verify orchestrator memory paths and permissions; restart the MCP server if config is inconsistent.";
const REM_MEMORY_PERSIST: &str =
    "Check disk quotas and MEMORY.md / daily log paths; ensure the agent id is valid.";
const REM_MEMORY_KEY: &str =
    "Store the fact first or run `memory_list_keys`; keys are case-sensitive.";
const REM_MEMORY_RETRIEVAL: &str =
    "Verify corpus/index paths and RAG settings; see orchestrator memory configuration.";
const REM_MEMORY_KG_QUERY: &str =
    "Check Turso connectivity, vox-db migrations, and that the knowledge graph tables exist.";

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
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_MEMORY_PERSIST)
                        .to_json()
                }
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("memory init failed: {e}"),
            REM_MEMORY_INIT,
        )
        .to_json(),
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
                Ok(None) => ToolResult::<String>::err_with_remediation(
                    format!("Key '{}' not found", params.key),
                    REM_MEMORY_KEY,
                )
                .to_json(),
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_MEMORY_PERSIST)
                        .to_json()
                }
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("memory init failed: {e}"),
            REM_MEMORY_INIT,
        )
        .to_json(),
    }
}

/// Search memory (daily logs + MEMORY.md) by keyword.
pub async fn memory_search(state: &ServerState, params: MemorySearchParams) -> String {
    match run_retrieval_bundle(
        state,
        &params.query,
        RetrievalTriggerMode::ExplicitToolQuery,
        10,
    )
    .await
    {
        Ok(bundle) => {
            if bundle.memory_lines.is_empty()
                && bundle.knowledge_lines.is_empty()
                && bundle.chunk_lines.is_empty()
            {
                ToolResult::ok("No results found.".to_string()).to_json()
            } else {
                let mut out = Vec::new();
                out.push(format!(
                    "retrieval_tier={} trigger={:?} used_vector={} used_bm25={} lexical_fallback={} contradictions={} knowledge_hits={} chunk_hits={}",
                    bundle.evidence.retrieval_tier,
                    bundle.evidence.trigger,
                    bundle.evidence.used_vector,
                    bundle.evidence.used_bm25,
                    bundle.evidence.used_lexical_fallback,
                    bundle.evidence.contradiction_count,
                    bundle.evidence.knowledge_hit_count,
                    bundle.evidence.chunk_hit_count,
                ));
                if !bundle.memory_lines.is_empty() {
                    out.push("[MEMORY]".to_string());
                    out.extend(bundle.memory_lines);
                }
                if !bundle.knowledge_lines.is_empty() {
                    out.push("[KNOWLEDGE_GRAPH]".to_string());
                    out.extend(bundle.knowledge_lines);
                }
                if !bundle.chunk_lines.is_empty() {
                    out.push("[DOCUMENT_CHUNKS]".to_string());
                    out.extend(bundle.chunk_lines);
                }
                ToolResult::ok(out.join("\n")).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(e, REM_MEMORY_RETRIEVAL).to_json(),
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
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_MEMORY_PERSIST)
                        .to_json()
                }
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("memory init failed: {e}"),
            REM_MEMORY_INIT,
        )
        .to_json(),
    }
}

/// List all memory keys from MEMORY.md.
pub async fn memory_list_keys(state: &ServerState) -> String {
    let config = memory_config_for_state(state);
    match vox_orchestrator::MemoryManager::new(config) {
        Ok(mgr) => match mgr.list_keys() {
            Ok(keys) => ToolResult::ok(keys).to_json(),
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_MEMORY_PERSIST)
                    .to_json()
            }
        },
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("memory init failed: {e}"),
            REM_MEMORY_INIT,
        )
        .to_json(),
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
                        .map(|(id, label, snippet)| {
                            format!("[{}] {} — {}", id, label, snippet.replace('\n', " "))
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    ToolResult::ok(formatted).to_json()
                }
            }
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_MEMORY_KG_QUERY)
                    .to_json()
            }
        }
    } else {
        ToolResult::<String>::err_with_remediation(
            "VoxDb not attached to MCP server.".to_string(),
            REM_MEMORY_VOXDB,
        )
        .to_json()
    }
}
