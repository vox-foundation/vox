//! MCP Server state and protocol handler implementation.

mod handlers;
mod lifecycle;

pub use handlers::{VoxMcpServer, tool_json_envelope_is_error};
pub use lifecycle::{CachedCatalog, ServerState, mcp_agent_fleet_env_enabled, run_stdio_server_blocking};
