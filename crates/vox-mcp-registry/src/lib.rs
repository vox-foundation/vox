//! MCP tool registry built from `contracts/mcp/tool-registry.canonical.yaml`.
//!
//! Consume [`TOOL_REGISTRY`] instead of duplicating name/description tables.

include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));
