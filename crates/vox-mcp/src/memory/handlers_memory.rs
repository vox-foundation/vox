use crate::{ServerState, ToolResult};

use super::config::memory_config_for_state;
use super::params::{
    KnowledgeQueryParams, MemoryLogParams, MemoryRecallParams, MemorySearchParams,
    MemoryStoreParams,
};
use super::retrieval::{RetrievalTriggerMode, run_retrieval_bundle};

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
    match run_retrieval_bundle(
        state,
        &params.query,
        RetrievalTriggerMode::ExplicitToolQuery,
        10,
    )
    .await
    {
        Ok(bundle) => {
            if bundle.memory_lines.is_empty() && bundle.knowledge_lines.is_empty() {
                ToolResult::ok("No results found.".to_string()).to_json()
            } else {
                let mut out = Vec::new();
                out.push(format!(
                    "retrieval_tier={} trigger={:?} used_vector={} used_bm25={} lexical_fallback={} contradictions={}",
                    bundle.evidence.retrieval_tier,
                    bundle.evidence.trigger,
                    bundle.evidence.used_vector,
                    bundle.evidence.used_bm25,
                    bundle.evidence.used_lexical_fallback,
                    bundle.evidence.contradiction_count
                ));
                if !bundle.memory_lines.is_empty() {
                    out.push("[MEMORY]".to_string());
                    out.extend(bundle.memory_lines);
                }
                if !bundle.knowledge_lines.is_empty() {
                    out.push("[KNOWLEDGE_GRAPH]".to_string());
                    out.extend(bundle.knowledge_lines);
                }
                ToolResult::ok(out.join("\n")).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err(e).to_json(),
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
