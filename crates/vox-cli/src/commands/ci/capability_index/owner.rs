//! Owner crate and source-path resolution (D-04). Pure: no IO, no clock.
//!
//! Rule order for a cli row: catalog `owner_crate` override, `vox_<x>::` handler path,
//! group table, `vox-cli-<g>` / `vox-<g>` crate-name match, `commands/<g>` module (owner
//! `vox-cli`), else unresolved. An mcp row is the override or the MCP dispatch crate. The `rule`
//! field of the result names the rule that fired.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use vox_capability_registry::{OwnerRef, RowKind};

use super::inputs::OperationRow;

/// `docs/src/architecture/layers.toml [crates]`: crate name -> layer.
pub(crate) type Layers = BTreeMap<String, Option<u8>>;

pub(crate) const CLI_COMMANDS_DIR: &str = "crates/vox-cli/src/commands";
pub(crate) const MCP_DISPATCH_PATH: &str = "crates/vox-orchestrator-mcp/src/dispatch.rs";

pub(crate) fn resolve_owner(
    _kind: RowKind,
    _op: &OperationRow,
    _layers: &Layers,
    _group_owners: &BTreeMap<String, String>,
    _cli_command_files: &BTreeSet<String>,
) -> Result<Option<OwnerRef>> {
    Ok(None)
}

pub(crate) fn source_path_for(
    _kind: RowKind,
    _op: &OperationRow,
    _owner: Option<&OwnerRef>,
    _cli_command_files: &BTreeSet<String>,
) -> Option<String> {
    Some(String::new()) // RED stub: replaced by the real resolver in the GREEN commit
}

#[cfg(test)]
mod tests {
    use super::super::inputs::test_op;
    use super::*;

    fn layers(names: &[(&str, u8)]) -> Layers {
        names
            .iter()
            .map(|(n, l)| ((*n).to_string(), Some(*l)))
            .collect()
    }

    fn cli_op(path: &[&str]) -> OperationRow {
        test_op(&path.join("."), Some((path, "active")), None, "d")
    }

    fn resolve(kind: RowKind, op: &OperationRow, l: &Layers) -> Result<Option<OwnerRef>> {
        resolve_owner(kind, op, l, &BTreeMap::new(), &BTreeSet::new())
    }

    #[test]
    fn override_wins() {
        let mut op = cli_op(&["db", "x"]);
        op.owner_crate = Some("vox-secrets".into());
        op.cli.as_mut().expect("cli").handler_rust = Some("vox_db::x".into());
        let l = layers(&[("vox-secrets", 1), ("vox-db", 2)]);
        let o = resolve(RowKind::Cli, &op, &l).expect("ok").expect("owner");
        assert_eq!(
            (o.crate_name.as_str(), o.rule.as_str()),
            ("vox-secrets", "override")
        );
    }

    #[test]
    fn unknown_override_is_error() {
        let mut op = cli_op(&["db", "x"]);
        op.owner_crate = Some("vox-nope".into());
        let err = resolve(RowKind::Cli, &op, &layers(&[("vox-db", 2)])).expect_err("must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("db.x") && msg.contains("vox-nope"),
            "got: {msg}"
        );
    }

    #[test]
    fn handler_path_rule() {
        let mut op = cli_op(&["research", "run"]);
        op.cli.as_mut().expect("cli").handler_rust =
            Some("vox_cli_research::run_research_query".into());
        let l = layers(&[("vox-cli-research", 5)]);
        let o = resolve(RowKind::Cli, &op, &l).expect("ok").expect("owner");
        assert_eq!(
            (o.crate_name.as_str(), o.rule.as_str()),
            ("vox-cli-research", "handler_path")
        );
    }

    #[test]
    fn group_table_rule() {
        let op = cli_op(&["ars", "x"]);
        let groups = BTreeMap::from([("ars".to_string(), "vox-openclaw-runtime".to_string())]);
        let l = layers(&[("vox-openclaw-runtime", 3)]);
        let o = resolve_owner(RowKind::Cli, &op, &l, &groups, &BTreeSet::new())
            .expect("ok")
            .expect("owner");
        assert_eq!(
            (o.crate_name.as_str(), o.rule.as_str()),
            ("vox-openclaw-runtime", "group_table")
        );
    }

    #[test]
    fn crate_name_match_rule() {
        let op = cli_op(&["secrets", "get"]);
        let l = layers(&[("vox-secrets", 1)]);
        let o = resolve(RowKind::Cli, &op, &l).expect("ok").expect("owner");
        assert_eq!(
            (o.crate_name.as_str(), o.rule.as_str()),
            ("vox-secrets", "crate_name_match")
        );
        let l = layers(&[("vox-secrets", 1), ("vox-cli-secrets", 5)]);
        let o = resolve(RowKind::Cli, &op, &l).expect("ok").expect("owner");
        assert_eq!(
            o.crate_name, "vox-cli-secrets",
            "vox-cli-<g> is tried first"
        );
    }

    #[test]
    fn cli_module_rule() {
        let op = cli_op(&["add"]);
        let files = BTreeSet::from([format!("{CLI_COMMANDS_DIR}/add.rs")]);
        let l = layers(&[("vox-cli", 5)]);
        let o = resolve_owner(RowKind::Cli, &op, &l, &BTreeMap::new(), &files)
            .expect("ok")
            .expect("owner");
        assert_eq!(
            (o.crate_name.as_str(), o.rule.as_str()),
            ("vox-cli", "cli_module")
        );
    }

    #[test]
    fn unresolved_is_none() {
        let op = cli_op(&["zz"]);
        let l = layers(&[("vox-cli", 5)]);
        assert!(resolve(RowKind::Cli, &op, &l).expect("ok").is_none());
    }

    #[test]
    fn mcp_rows_use_dispatch_crate() {
        let op = test_op("m", None, Some("vox_m"), "d");
        let l = layers(&[("vox-orchestrator-mcp", 4)]);
        let o = resolve(RowKind::McpTool, &op, &l)
            .expect("ok")
            .expect("owner");
        assert_eq!(
            (o.crate_name.as_str(), o.rule.as_str()),
            ("vox-orchestrator-mcp", "mcp_dispatch")
        );
    }

    #[test]
    fn layer_from_layers_toml() {
        let op = cli_op(&["secrets", "get"]);
        let l = layers(&[("vox-secrets", 1)]);
        let o = resolve(RowKind::Cli, &op, &l).expect("ok").expect("owner");
        assert_eq!(o.layer, Some(1));
    }

    #[test]
    fn source_path_prefers_handler_module() {
        let mut op = cli_op(&["graph", "x"]);
        op.cli.as_mut().expect("cli").handler_rust = Some("commands::graphify::run".into());
        let files = BTreeSet::from([format!("{CLI_COMMANDS_DIR}/graphify/mod.rs")]);
        assert_eq!(
            source_path_for(RowKind::Cli, &op, None, &files).as_deref(),
            Some("crates/vox-cli/src/commands/graphify/mod.rs")
        );
        let owner = OwnerRef {
            crate_name: "vox-db".into(),
            layer: Some(2),
            rule: "group_table".into(),
        };
        assert_eq!(
            source_path_for(RowKind::Cli, &cli_op(&["db", "x"]), Some(&owner), &files).as_deref(),
            Some("crates/vox-db/")
        );
        assert_eq!(
            source_path_for(RowKind::McpTool, &op, None, &files).as_deref(),
            Some(MCP_DISPATCH_PATH)
        );
        assert_eq!(
            source_path_for(RowKind::Cli, &cli_op(&["zz"]), None, &files),
            None
        );
    }
}
