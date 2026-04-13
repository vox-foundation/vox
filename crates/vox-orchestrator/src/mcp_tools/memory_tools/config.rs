use crate::mcp_tools::server_state::ServerState;

pub(crate) fn memory_config_for_state(state: &ServerState) -> crate::MemoryConfig {
    state.orchestrator_config.memory.clone()
}
