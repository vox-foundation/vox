//! Vox MCP Server — Model Context Protocol (stdio) for Vox: orchestrator, Codex/Turso tools, LLM
//! bridge, and mesh/Mens registry surfaces.

mod sync_poison;

// Re-export from vox-orchestrator native MCP tools
pub use vox_orchestrator::mcp_tools::{
    ServerState, ToolResult, VoxMcpServer, client, context, dei_ipc, http_gateway,
    journey_envelope, lifecycle, llm_bridge, load_config, mcp_agent_fleet_env_enabled,
    populi_startup, run_stdio_server_blocking, speech_constraints,
};

pub mod sync_lock;

pub use vox_orchestrator::mcp_tools::params::{
    AgentInfo, CancelTaskParams, DrainAgentParams, MapAgentSessionParams, ReorderTaskParams,
    StatusResponse,
};

// Internal-to-CLI helper used by handlers (now moved but we can keep a local alias if needed)
pub use vox_orchestrator::mcp_tools::server::tool_json_envelope_is_error;
