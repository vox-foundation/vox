//! MCP tool argument storage modes for Ludus routing (privacy / payload size).
//!
//! Generalized field-level redaction for MCP transcripts (beyond args) is tracked in telemetry SSOT
//! (`docs/src/architecture/telemetry-trust-ssot.md`); this module implements the **storage-shape** policy for tool `args` only.

use serde_json::Value;

use crate::config_gate::McpToolArgsStorage;

/// Shape `args` for persistence in `mcp_tool_called` / `tool_call` agent_events.
#[must_use]
pub fn prepare_mcp_tool_args_for_storage(args: &Value) -> Value {
    match crate::config_gate::mcp_tool_args_storage() {
        McpToolArgsStorage::Full => args.clone(),
        McpToolArgsStorage::Omit => Value::Null,
        McpToolArgsStorage::Hash => {
            let bytes = serde_json::to_vec(args).unwrap_or_default();
            let h = xxhash_rust::xxh3::xxh3_64(&bytes);
            Value::String(format!("xxh3:{h:016x}"))
        }
    }
}
