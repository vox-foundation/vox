//! Owner crate and source-path resolution (D-04). Pure: no IO, no clock.
//!
//! Rule order for a cli row: catalog `owner_crate` override, `vox_<x>::` handler path,
//! group table, `vox-cli-<g>` / `vox-<g>` crate-name match, `commands/<g>` module (owner
//! `vox-cli`), else unresolved. An mcp row is the override or the MCP dispatch crate. The `rule`
//! field of the result names the rule that fired.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use vox_capability_registry::{OwnerRef, RowKind};

use super::inputs::OperationRow;

/// `docs/src/architecture/layers.toml [crates]`: crate name -> layer.
pub(crate) type Layers = BTreeMap<String, Option<u8>>;

pub(crate) const CLI_COMMANDS_DIR: &str = "crates/vox-cli/src/commands";
pub(crate) const MCP_DISPATCH_PATH: &str = "crates/vox-orchestrator-mcp/src/dispatch.rs";
const MCP_CRATE: &str = "vox-orchestrator-mcp";
const CLI_CRATE: &str = "vox-cli";

fn owner_ref(layers: &Layers, krate: &str, rule: &str) -> OwnerRef {
    OwnerRef {
        crate_name: krate.to_string(),
        layer: layers.get(krate).copied().flatten(),
        rule: rule.to_string(),
    }
}

/// The owning crate of one row, or `None` when no rule resolves it (counted by the ratchet).
/// An `owner_crate` override that is not a `layers.toml` key is an error naming the op id.
pub(crate) fn resolve_owner(
    kind: RowKind,
    op: &OperationRow,
    layers: &Layers,
    group_owners: &BTreeMap<String, String>,
    cli_command_files: &BTreeSet<String>,
) -> Result<Option<OwnerRef>> {
    if let Some(name) = &op.owner_crate {
        if !layers.contains_key(name) {
            bail!(
                "catalog op `{}`: owner_crate `{name}` is not a crate in docs/src/architecture/layers.toml [crates]",
                op.id
            );
        }
        return Ok(Some(owner_ref(layers, name, "override")));
    }
    if kind == RowKind::McpTool {
        return Ok(Some(owner_ref(layers, MCP_CRATE, "mcp_dispatch")));
    }
    let Some(cli) = &op.cli else {
        return Ok(None);
    };
    // `vox_<x>::path` names its crate directly.
    if let Some(first) = cli
        .handler_rust
        .as_deref()
        .and_then(|h| h.split("::").next())
        .filter(|f| f.starts_with("vox_"))
    {
        let krate = first.replace('_', "-");
        if layers.contains_key(&krate) {
            return Ok(Some(owner_ref(layers, &krate, "handler_path")));
        }
    }
    let Some(group) = cli.path.first() else {
        return Ok(None);
    };
    if let Some(krate) = group_owners.get(group) {
        return Ok(Some(owner_ref(layers, krate, "group_table")));
    }
    for krate in [format!("vox-cli-{group}"), format!("vox-{group}")] {
        if layers.contains_key(&krate) {
            return Ok(Some(owner_ref(layers, &krate, "crate_name_match")));
        }
    }
    let module = group.replace('-', "_");
    let has_module = [
        format!("{CLI_COMMANDS_DIR}/{module}.rs"),
        format!("{CLI_COMMANDS_DIR}/{module}/mod.rs"),
    ]
    .iter()
    .any(|p| cli_command_files.contains(p));
    Ok(has_module.then(|| owner_ref(layers, CLI_CRATE, "cli_module")))
}

/// Where to read the implementation: the handler module when the catalog names one, the MCP
/// dispatch table for tools, else the owning crate directory.
pub(crate) fn source_path_for(
    kind: RowKind,
    op: &OperationRow,
    owner: Option<&OwnerRef>,
    cli_command_files: &BTreeSet<String>,
) -> Option<String> {
    if kind == RowKind::McpTool {
        return Some(MCP_DISPATCH_PATH.to_string());
    }
    let handler = op.cli.as_ref().and_then(|c| c.handler_rust.as_deref());
    if let Some(rest) = handler.and_then(|h| h.strip_prefix("commands::")) {
        let segs: Vec<&str> = rest.split("::").collect();
        let a = segs[0];
        let mut candidates = Vec::new();
        if let Some(b) = segs.get(1) {
            candidates.push(format!("{CLI_COMMANDS_DIR}/{a}/{b}.rs"));
            candidates.push(format!("{CLI_COMMANDS_DIR}/{a}/{b}/mod.rs"));
        }
        candidates.push(format!("{CLI_COMMANDS_DIR}/{a}.rs"));
        candidates.push(format!("{CLI_COMMANDS_DIR}/{a}/mod.rs"));
        if let Some(found) = candidates
            .into_iter()
            .find(|c| cli_command_files.contains(c))
        {
            return Some(found);
        }
    }
    owner.map(|o| format!("crates/{}/", o.crate_name))
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
