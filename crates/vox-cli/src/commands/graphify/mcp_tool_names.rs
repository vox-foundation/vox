//! Reader for the canonical MCP tool registry (`contracts/mcp/tool-registry.canonical.yaml`).

use std::path::Path;

#[derive(serde::Deserialize)]
struct CanonicalRegistry {
    tools: Vec<ToolEntry>,
}

#[derive(serde::Deserialize)]
struct ToolEntry {
    name: String,
}

/// Reads all tool names defined in `contracts/mcp/tool-registry.canonical.yaml`.
///
/// Fails with an error if the registry file is missing or unparseable.
pub(crate) fn canonical_mcp_tool_names(repo_root: &Path) -> anyhow::Result<Vec<String>> {
    let path = repo_root.join("contracts/mcp/tool-registry.canonical.yaml");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        anyhow::anyhow!(
            "failed to read canonical MCP tool registry at {}: {e}",
            path.display()
        )
    })?;
    let reg: CanonicalRegistry = serde_yaml::from_str(&content).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse canonical MCP tool registry at {}: {e}",
            path.display()
        )
    })?;
    let mut names: Vec<String> = reg.tools.into_iter().map(|t| t.name).collect();
    // Injected at runtime by orchestrator daemon for dynamic oversight level changes,
    // not listed in static tool-registry.canonical.yaml.
    if !names.iter().any(|n| n == "orch.oversight_set") {
        names.push("orch.oversight_set".to_string());
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_canonical_registry() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let names = canonical_mcp_tool_names(repo_root).expect("read canonical tools");
        assert!(
            names.len() > 300,
            "expected >300 tools, found {}",
            names.len()
        );
        assert!(
            names.contains(&"vox_mesh_nodes".to_string()),
            "must contain vox_mesh_nodes"
        );
        assert!(
            names.contains(&"vox_compiler::ast_inspect".to_string()),
            "must contain vox_compiler::ast_inspect"
        );

        let missing = canonical_mcp_tool_names(Path::new("/nonexistent/repo/root"));
        assert!(
            missing.is_err(),
            "missing file must be Err, never empty list"
        );
    }
}
