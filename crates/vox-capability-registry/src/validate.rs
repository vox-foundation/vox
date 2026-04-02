//! Cross-registry consistency checks (MCP tools, CLI operations, curated rows).

use std::collections::HashSet;

use crate::document::CapabilityRegistryDoc;
use crate::ids::{implicit_cli_capability_id, implicit_mcp_capability_id};

/// Validate curated rows against MCP and CLI registries. Returns human-readable errors.
pub fn validate_cross_registry(
    doc: &CapabilityRegistryDoc,
    mcp_tools: &[String],
    cli_paths_active: &[Vec<String>],
) -> Vec<String> {
    let mut errs = Vec::new();
    let mcp_set: HashSet<&str> = mcp_tools.iter().map(String::as_str).collect();
    let cli_set: HashSet<Vec<String>> = cli_paths_active.iter().cloned().collect();

    let mut seen_ids: HashSet<String> = HashSet::new();
    for row in &doc.curated {
        if !seen_ids.insert(row.id.clone()) {
            errs.push(format!("duplicate curated capability id: {}", row.id));
        }
        if let Some(ref tool) = row.mcp_tool {
            if !mcp_set.contains(tool.as_str()) {
                errs.push(format!(
                    "curated capability '{}' references unknown MCP tool '{}'",
                    row.id, tool
                ));
            }
            let expected = implicit_mcp_capability_id(tool);
            if row.id != expected {
                errs.push(format!(
                    "curated id '{}' must equal implicit MCP id '{}' (mcp_tool={})",
                    row.id, expected, tool
                ));
            }
        }
        if let Some(ref path) = row.cli_path {
            if !cli_set.contains(path) {
                errs.push(format!(
                    "curated capability '{}' references unknown CLI path {:?}",
                    row.id, path
                ));
            }
            let expected = implicit_cli_capability_id(path);
            if row.id != expected {
                errs.push(format!(
                    "curated id '{}' must equal implicit CLI id '{}' (cli_path={:?})",
                    row.id, expected, path
                ));
            }
        }
    }

    let mut seen_rt: HashSet<(String, String)> = HashSet::new();
    for m in &doc.runtime_builtin_maps {
        let key = (m.namespace.clone(), m.method.clone());
        if !seen_rt.insert(key) {
            errs.push(format!(
                "duplicate runtime_builtin_maps entry: {}.{}",
                m.namespace, m.method
            ));
        }
    }

    if doc.auto_mcp_capabilities {
        // Implicit ids cover all MCP tools; nothing else required.
    } else {
        let mut covered: HashSet<&str> = HashSet::new();
        for row in &doc.curated {
            if let Some(ref t) = row.mcp_tool {
                covered.insert(t.as_str());
            }
        }
        for t in mcp_tools {
            if !covered.contains(t.as_str()) {
                errs.push(format!(
                    "auto_mcp_capabilities=false but MCP tool '{t}' has no curated row with mcp_tool"
                ));
            }
        }
    }

    errs
}
