use crate::server::ServerState;

pub(crate) fn memory_config_for_state(state: &ServerState) -> vox_orchestrator::MemoryConfig {
    state.orchestrator_config.memory.clone()
}
