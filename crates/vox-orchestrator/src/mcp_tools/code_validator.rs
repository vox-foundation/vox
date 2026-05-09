//! Code validation MCP tools — PHASE_0a_STUB.

use super::params::{ValidateFileParams, VoxCheckParams};
use super::server_state::ServerState;

pub async fn vox_check(_state: &ServerState, _params: VoxCheckParams) -> String {
    // PHASE_0a_STUB: real implementation pending MCP tool wiring.
    String::new()
}

pub async fn validate_file(_state: &ServerState, _params: ValidateFileParams) -> String {
    // PHASE_0a_STUB: real implementation pending MCP tool wiring.
    String::new()
}
