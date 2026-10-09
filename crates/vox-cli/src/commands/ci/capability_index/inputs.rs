//! Committed inputs of the capability index.
//!
//! Reads only checked-in contracts, never `command_catalog::build_catalog()`: the clap tree varies
//! with cargo features, the committed catalog does not.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use vox_bounded_fs::read_utf8_path_capped;

pub(crate) use super::super::operations_catalog::OperationRow;
use super::super::operations_catalog::read_catalog;
use super::owner::{CLI_COMMANDS_DIR, Layers};

const LAYERS_REL: &str = "docs/src/architecture/layers.toml";

/// Everything `join::build_index` needs; pure data so the join is testable without IO.
#[derive(Default)]
pub(crate) struct Inputs {
    pub ops: Vec<OperationRow>,
    /// `layers.toml [crates]`: crate name -> layer.
    pub layers: Layers,
    /// `/`-normalised repo-relative `.rs` files under `crates/vox-cli/src/commands/`.
    pub cli_command_files: BTreeSet<String>,
}

pub(crate) fn load(repo_root: &Path) -> Result<Inputs> {
    Ok(Inputs {
        ops: read_catalog(repo_root)?.operations,
        layers: load_layers(repo_root)?,
        cli_command_files: load_cli_command_files(repo_root)?,
    })
}

fn load_layers(repo_root: &Path) -> Result<Layers> {
    let path = repo_root.join(LAYERS_REL);
    let doc: toml::Value = toml::from_str(&read_utf8_path_capped(&path)?)
        .with_context(|| format!("parse {} as TOML", path.display()))?;
    let crates = doc
        .get("crates")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow!("{LAYERS_REL} has no [crates] table"))?;
    Ok(crates
        .iter()
        .map(|(name, v)| {
            let layer = v
                .get("layer")
                .and_then(toml::Value::as_integer)
                .and_then(|l| u8::try_from(l).ok());
            (name.clone(), layer)
        })
        .collect())
}

fn load_cli_command_files(repo_root: &Path) -> Result<BTreeSet<String>> {
    let dir = repo_root.join(CLI_COMMANDS_DIR);
    let mut files = BTreeSet::new();
    for entry in walkdir::WalkDir::new(&dir) {
        let entry = entry.with_context(|| format!("walk {}", dir.display()))?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "rs") {
            let rel = entry.path().strip_prefix(repo_root).unwrap_or(entry.path());
            files.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(files)
}

/// Synthetic catalog row for unit tests. `cli` is `(path, status)`.
#[cfg(test)]
pub(crate) fn test_op(
    id: &str,
    cli: Option<(&[&str], &str)>,
    mcp: Option<&str>,
    description: &str,
) -> OperationRow {
    let cli = cli.map(|(path, status)| serde_json::json!({ "path": path, "status": status }));
    let mcp = mcp.map(|name| serde_json::json!({ "name": name }));
    serde_json::from_value(serde_json::json!({
        "id": id,
        "title": format!("Title {id}"),
        "description": description,
        "product_lane": "platform",
        "cli": cli,
        "mcp": mcp,
    }))
    .expect("synthetic OperationRow")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_builds_both_projections() {
        let op = test_op("a.b", Some((&["a", "b"], "active")), Some("vox_a_b"), "d");
        assert_eq!(op.cli.expect("cli").path, vec!["a", "b"]);
        assert_eq!(op.mcp.expect("mcp").name, "vox_a_b");
    }
}
