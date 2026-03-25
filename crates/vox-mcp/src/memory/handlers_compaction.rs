use crate::{ServerState, ToolResult};

/// Get current context window usage and compaction recommendation (async).
pub async fn compaction_status(
    state: &ServerState,
    params: crate::context::ContextBudgetParams,
) -> String {
    let orch = &state.orchestrator;
    let id = vox_orchestrator::AgentId(params.agent_id);
    let handle = orch.budget_handle();
    let budget_lock = match crate::sync_poison::poison_rw_read(handle.read(), "agent budget") {
        Ok(g) => g,
        Err(e) => return ToolResult::<String>::err(e.to_string()).to_json(),
    };
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
