//! MCP tools: query endpoint and agent reliability metrics (V19/20).
//!
//! Provides agents visibility into system performance, hallucination rates,
//! and infra stability across OpenRouter and Populi backends.

use serde::Deserialize;
use crate::params::ToolResult;
use crate::server::ServerState;

/// Arguments for `vox_reliability_list`.
#[derive(Debug, Deserialize)]
pub struct ReliabilityListParams {
    /// Max rows to return (default 50).
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 { 50 }

/// List ranked endpoint reliability metrics (hallucination rates, failures, timeouts).
pub async fn reliability_list(state: &ServerState, params: ReliabilityListParams) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<String>::err("Codex not attached; reliability tracking requires Turso.").to_json();
    };
    match db.store().list_endpoint_reliability(params.limit).await {
        Ok(rows) => ToolResult::ok(rows).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// List agent success/failure reliability EMAs (Socrates routing signals).
pub async fn reliability_agents(state: &ServerState) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<String>::err("Codex not attached.").to_json();
    };
    match db.store().list_agent_reliability().await {
        Ok(rows) => ToolResult::ok(rows).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}
