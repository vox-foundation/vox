//! Committed inputs of the capability index.
//!
//! Reads only checked-in contracts, never `command_catalog::build_catalog()`: the clap tree varies
//! with cargo features, the committed catalog does not.

use std::path::Path;

use anyhow::Result;

pub(crate) use super::super::operations_catalog::OperationRow;
use super::super::operations_catalog::read_catalog;

/// Everything `join::build_index` needs; pure data so the join is testable without IO.
pub(crate) struct Inputs {
    pub ops: Vec<OperationRow>,
}

pub(crate) fn load(repo_root: &Path) -> Result<Inputs> {
    Ok(Inputs {
        ops: read_catalog(repo_root)?.operations,
    })
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
