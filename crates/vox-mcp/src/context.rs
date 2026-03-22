use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use vox_orchestrator::AgentId;

use crate::{ServerState, ToolResult};

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

/// MCP arguments: upsert a namespaced context string for one agent (`ttl_seconds` optional retention hint).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetContextParams {
    /// Agent namespace for the key (orchestrator context is per-agent).
    pub agent_id: u64,
    /// Arbitrary string key.
    pub key: String,
    /// UTF-8 value to store.
    pub value: String,
    /// Optional seconds-to-live (`0` means default / indefinite in orchestrator).
    pub ttl_seconds: Option<u64>,
}

/// MCP arguments: fetch a single context value by global key.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetContextParams {
    /// Key to look up in the global context map.
    pub key: String,
}

/// MCP arguments: enumerate keys sharing a string prefix (orchestrator context store).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListContextParams {
    /// Only keys starting with this prefix are listed.
    pub prefix: String,
}

/// MCP arguments: inspect token budget / summarization hint for one agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContextBudgetParams {
    /// Agent whose token budget is summarized.
    pub agent_id: u64,
}

/// MCP arguments: copy summarized context from `from_agent` to `to_agent` via orchestrator handoff.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct HandoffContextParams {
    /// Source agent id for summarized context.
    pub from_agent: u64,
    /// Destination agent id receiving the handoff.
    pub to_agent: u64,
}

// ---------------------------------------------------------------------------
// Tool Handlers
// ---------------------------------------------------------------------------

/// Set a key-value pair in the shared orchestrator context (async).
pub async fn set_context(state: &ServerState, params: SetContextParams) -> String {
    let orch = state.orchestrator.lock().await;
    let ttl = params.ttl_seconds.unwrap_or(0);
    orch.context()
        .set(AgentId(params.agent_id), &params.key, &params.value, ttl);
    ToolResult::ok(format!("Key '{}' set successfully", params.key)).to_json()
}

/// Retrieve a value from the shared context (async).
pub async fn get_context(state: &ServerState, params: GetContextParams) -> String {
    let orch = state.orchestrator.lock().await;
    if let Some(val) = orch.context().get(&params.key) {
        ToolResult::ok(val).to_json()
    } else {
        ToolResult::<String>::err("Key not found or expired").to_json()
    }
}

/// List available context keys by prefix (async).
pub async fn list_context(state: &ServerState, params: ListContextParams) -> String {
    let orch = state.orchestrator.lock().await;
    let keys = orch.context().list_keys(&params.prefix);
    ToolResult::ok(keys).to_json()
}

/// Get the token budget status for an agent (async).
pub async fn context_budget(state: &ServerState, params: ContextBudgetParams) -> String {
    let orch = state.orchestrator.lock().await;
    let id = AgentId(params.agent_id);
    if let Some(budget) = orch.budget().check_budget(id) {
        let should_summarize = budget.should_summarize();
        ToolResult::ok(format!(
            "Budget: {}/{} tokens used. Summarize recommended: {}",
            budget.tokens_used, budget.model_max_tokens, should_summarize
        ))
        .to_json()
    } else {
        ToolResult::ok("No budget tracked for this agent.").to_json()
    }
}

/// Handoff summarized context from one agent to another (async).
pub async fn handoff_context(state: &ServerState, params: HandoffContextParams) -> String {
    let orch = state.orchestrator.lock().await;
    orch.summary()
        .handoff(AgentId(params.from_agent), AgentId(params.to_agent));
    ToolResult::ok(format!(
        "Context handed off from agent {} to {}",
        params.from_agent, params.to_agent
    ))
    .to_json()
}
