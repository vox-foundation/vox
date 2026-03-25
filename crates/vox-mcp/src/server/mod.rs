//! MCP Server state and protocol handler implementation.

mod handlers;
mod lifecycle;

pub use handlers::{tool_json_envelope_is_error, VoxMcpServer};
pub use lifecycle::ServerState;
