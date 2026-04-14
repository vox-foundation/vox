use schemars::JsonSchema;
use serde::Deserialize;

use crate::AgentId;

use crate::mcp_tools::server_state::ServerState;
use crate::mcp_tools::params::ToolResult;

const REM_CTX_LOCK: &str = "Retry; poisoned orchestrator locks usually clear after MCP restart.";
const REM_CTX_KEY: &str = "Use `list_context` with a prefix or verify the key was set under the expected agent namespace.";

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

/// MCP arguments: Set custom VoxDB-powered budget limits for an agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetAgentBudgetParams {
    /// Agent to apply budget limits to.
    pub agent_id: u64,
    /// Maximum total tokens before hard-stop.
    pub max_tokens: usize,
    /// Maximum total dollar cost before hard-stop.
    pub max_cost_usd: f64,
    /// Ratio at which to trigger token warnings (default 0.8).
    pub token_alert_threshold: Option<f64>,
    /// Ratio at which to trigger cost warnings (default 0.9).
    pub cost_alert_threshold: Option<f64>,
    /// Token rollover fraction (0.0 to 1.0).
    pub rollover_fraction: Option<f64>,
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
    let orch = &state.orchestrator;
    let ttl = params.ttl_seconds.unwrap_or(0);
    let ctx_handle = orch.context_handle();
    let guard: std::sync::RwLockWriteGuard<crate::context::ContextStore> =
        match crate::mcp_tools::sync_poison::poison_rw_write(ctx_handle.write(), "orchestrator context") {
            Ok(g) => g,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(e.to_string(), REM_CTX_LOCK)
                    .to_json();
            }
        };
    guard.set(
        AgentId(params.agent_id),
        &params.key,
        &params.value,
        ttl,
    );
    ToolResult::ok(format!("Key '{}' set successfully", params.key)).to_json()
}

/// Retrieve a value from the shared context (async).
pub async fn get_context(state: &ServerState, params: GetContextParams) -> String {
    let orch = &state.orchestrator;
    let ctx_handle = orch.context_handle();
    let read_guard: std::sync::RwLockReadGuard<crate::context::ContextStore> =
        match crate::mcp_tools::sync_poison::poison_rw_read(ctx_handle.read(), "orchestrator context") {
            Ok(g) => g,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(e.to_string(), REM_CTX_LOCK)
                    .to_json();
            }
        };
    if let Some(val) = read_guard.get(&params.key) {
        ToolResult::ok(val).to_json()
    } else {
        ToolResult::<String>::err_with_remediation("Key not found or expired", REM_CTX_KEY)
            .to_json()
    }
}

/// List available context keys by prefix (async).
pub async fn list_context(state: &ServerState, params: ListContextParams) -> String {
    let orch = &state.orchestrator;
    let ctx_handle = orch.context_handle();
    let read_guard: std::sync::RwLockReadGuard<crate::context::ContextStore> =
        match crate::mcp_tools::sync_poison::poison_rw_read(ctx_handle.read(), "orchestrator context") {
            Ok(g) => g,
            Err(e) => {
                return ToolResult::<Vec<String>>::err_with_remediation(
                    e.to_string(),
                    REM_CTX_LOCK,
                )
                .to_json();
            }
        };
    let keys = read_guard.list_keys(&params.prefix);
    ToolResult::ok(keys).to_json()
}

/// Get the token budget status for an agent (async).
pub async fn context_budget(state: &ServerState, params: ContextBudgetParams) -> String {
    let orch = &state.orchestrator;
    let id = AgentId(params.agent_id);
    let budget_handle = orch.budget_handle();
    let budget_guard: std::sync::RwLockReadGuard<crate::budget::BudgetManager> =
        match crate::mcp_tools::sync_poison::poison_rw_read(budget_handle.read(), "token budget") {
            Ok(g) => g,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(e.to_string(), REM_CTX_LOCK)
                    .to_json();
            }
        };
    if let Some(budget) = budget_guard.check_budget(id) {
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

/// Set a custom budget capped limit and persist to VoxDB.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmergencyStopParams {
    /// Optional reason for stopping the orchestrator.
    #[serde(default)]
    pub reason: Option<String>,
}

pub async fn emergency_stop(state: &ServerState, params: EmergencyStopParams) -> String {
    state.orchestrator.emergency_stop(params.reason.clone());
    format!(
        "Emergency stop triggered. Reason: {}",
        params.reason.as_deref().unwrap_or("none")
    )
}

pub async fn set_agent_budget(state: &ServerState, params: SetAgentBudgetParams) -> String {
    let orch = &state.orchestrator;
    let agent_id = AgentId(params.agent_id);

    let mut alloc = crate::budget::AgentBudgetAllocation::new(
        params.max_tokens,
        params.max_cost_usd,
    );
    if let (Some(token_al), Some(cost_al)) =
        (params.token_alert_threshold, params.cost_alert_threshold)
    {
        alloc = alloc.with_alert_thresholds(token_al, cost_al);
    }
    if let Some(rollover) = params.rollover_fraction {
        alloc = alloc.with_rollover(rollover);
    }

    let bm = match crate::mcp_tools::sync_poison::poison_rw_read(
        orch.budget_handle().read(),
        "budget manager lock",
    ) {
        Ok(guard) => (*guard).clone(),
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e.to_string(), REM_CTX_LOCK)
                .to_json();
        }
    };

    bm.set_and_persist_allocation(agent_id, alloc).await;

    ToolResult::ok(format!(
        "Budget cap set and persisted for agent {}",
        params.agent_id
    ))
    .to_json()
}

/// Handoff summarized context from one agent to another (async).
pub async fn handoff_context(state: &ServerState, params: HandoffContextParams) -> String {
    let orch = &state.orchestrator;
    let summary_handle = orch.summary_handle();
    let sum_guard: std::sync::RwLockWriteGuard<crate::summary::SummaryManager> =
        match crate::mcp_tools::sync_poison::poison_rw_write(summary_handle.write(), "context summary") {
            Ok(g) => g,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(e.to_string(), REM_CTX_LOCK)
                    .to_json();
            }
        };
    sum_guard.handoff(
        AgentId(params.from_agent),
        AgentId(params.to_agent),
    );
    ToolResult::ok(format!(
        "Context handed off from agent {} to {}",
        params.from_agent, params.to_agent
    ))
    .to_json()
}

