//! Deterministic capability id helpers (implicit MCP / CLI surfaces).

/// Implicit capability id for an MCP tool name from `tool-registry.canonical.yaml`.
#[must_use]
pub fn implicit_mcp_capability_id(mcp_tool: &str) -> String {
    format!("mcp.{mcp_tool}")
}

/// Implicit capability id for a `vox-cli` command path from `command-registry.yaml`.
#[must_use]
pub fn implicit_cli_capability_id(cli_path: &[String]) -> String {
    format!("cli.{}", cli_path.join("."))
}
