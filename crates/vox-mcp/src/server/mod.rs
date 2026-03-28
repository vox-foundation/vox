//! MCP Server state and protocol handler implementation.

mod clarification_inbox;
mod handlers;
mod lifecycle;

pub use handlers::{VoxMcpServer, tool_json_envelope_is_error};
pub use lifecycle::{ServerState, mcp_agent_fleet_env_enabled};
