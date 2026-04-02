//! Minimal parse of `contracts/cli/command-registry.yaml` for active `vox-cli` paths.

use anyhow::{Context, Result};
use serde::Deserialize;

/// Repo-relative path to the shipped CLI command registry (SSOT).
pub const COMMAND_REGISTRY_REL: &str = "contracts/cli/command-registry.yaml";

#[derive(Debug, Deserialize)]
struct CommandRegistryOp {
    surface: String,
    path: Vec<String>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct CommandRegistryRoot {
    operations: Vec<CommandRegistryOp>,
}

/// Active `surface: vox-cli` command paths (used for implicit CLI capability ids).
pub fn active_vox_cli_paths_from_command_registry_yaml(yaml: &str) -> Result<Vec<Vec<String>>> {
    let root: CommandRegistryRoot =
        serde_yaml::from_str(yaml).context("parse command-registry.yaml")?;
    Ok(root
        .operations
        .into_iter()
        .filter(|o| o.surface == "vox-cli" && o.status == "active")
        .map(|o| o.path)
        .collect())
}
