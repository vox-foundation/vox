//! MCP server state — PHASE_0a_STUB.

use crate::config::OrchestratorConfig;

pub struct ServerState {
    #[allow(dead_code)]
    config: OrchestratorConfig,
}

impl ServerState {
    pub fn new_full(config: OrchestratorConfig) -> Self {
        // PHASE_0a_STUB: real implementation pending MCP server redesign.
        Self { config }
    }
}
