//! Capability registry YAML vs MCP tool registry and CLI command-registry.

use anyhow::{Context, Result, anyhow};

use crate::command_registry_model::RegistryFile;
use vox_bounded_fs::read_utf8_path_capped;

use super::registry::{
    CAPABILITY_REGISTRY_REL, parse_mcp_registry_yaml,
    validate_capability_registry_against_json_schema,
};

/// Validate `contracts/capability/capability-registry.yaml` and cross-registry mappings.
pub(crate) fn check_capability_registry(
    repo_root: &std::path::Path,
    reg: &RegistryFile,
    mcp_yaml: &str,
) -> Result<()> {
    let cap_path = repo_root.join(CAPABILITY_REGISTRY_REL);
    let raw =
        read_utf8_path_capped(&cap_path).with_context(|| format!("read {}", cap_path.display()))?;
    validate_capability_registry_against_json_schema(repo_root, &raw)?;
    let doc: vox_capability_registry::CapabilityRegistryDoc =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", cap_path.display()))?;
    let mcp_tools = parse_mcp_registry_yaml(mcp_yaml)?;
    let cli_paths_active: Vec<Vec<String>> = reg
        .operations
        .iter()
        .filter(|o| o.surface == "vox-cli" && o.status == "active")
        .map(|o| o.path.clone())
        .collect();

    let mut errs =
        vox_capability_registry::validate_cross_registry(&doc, &mcp_tools, &cli_paths_active);
    if let Some(ex) = &doc.exemptions {
        for path in &ex.cli_paths {
            if !cli_paths_active.iter().any(|p| p == path) {
                errs.push(format!(
                    "capability-registry exemptions.cli_paths: unknown active CLI path {:?}",
                    path
                ));
            }
        }
    }
    if !errs.is_empty() {
        let msg = errs.join("\n");
        return Err(anyhow!("capability-registry cross-check failed:\n{msg}"));
    }
    println!("capability-registry OK (curated rows + MCP implicit coverage)");
    Ok(())
}
