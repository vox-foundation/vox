//! MCP tool registry built from `contracts/mcp/tool-registry.canonical.yaml`.
//!
//! Consume [`TOOL_REGISTRY`] instead of duplicating name/description tables.

/// One MCP tool row (name, description, bell-curve [`product_lane`](McpToolRegistryEntry::product_lane)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpToolRegistryEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub product_lane: &'static str,
    pub http_read_role_eligible: bool,
}

include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));
