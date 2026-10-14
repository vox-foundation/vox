//! Owner crate and source-path resolution (D-04). Pure: no IO, no clock.
//!
//! Rule order for a cli row: catalog `owner_crate` override, `vox_<x>::` handler path,
//! group table, `vox-cli-<g>` / `vox-<g>` crate-name match, `commands/<g>` module (owner
//! `vox-cli`), else unresolved. An mcp row is the override or the MCP dispatch crate. The `rule`
//! field of the result names the rule that fired.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use vox_capability_registry::{
    FALLBACK_DOMAIN, OwnerRef, REQUIRED_DOMAINS, RowKind, SURFACE_CRATES,
};

use super::inputs::{Domains, OperationRow};

/// `docs/src/architecture/layers.toml [crates]`: crate name -> layer.
pub(crate) type Layers = BTreeMap<String, Option<u8>>;

pub(crate) const CLI_COMMANDS_DIR: &str = "crates/vox-cli/src/commands";
pub(crate) const MCP_DISPATCH_PATH: &str = "crates/vox-orchestrator-mcp/src/dispatch.rs";
const MCP_CRATE: &str = "vox-orchestrator-mcp";
const CLI_CRATE: &str = "vox-cli";

pub(super) fn owner_ref(layers: &Layers, krate: &str, rule: &str) -> OwnerRef {
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

/// `group -> owner crate` for the cli groups whose dispatch provably calls into one crate.
pub(crate) fn group_owners(domains: &Domains) -> BTreeMap<String, String> {
    domains
        .cli_groups
        .iter()
        .filter_map(|(g, e)| e.owner.clone().map(|o| (g.clone(), o)))
        .collect()
}

/// Reject a malformed `domains.v1.yaml`: every table domain must be in the vocabulary and every
/// owner / crate key must be a `layers.toml` crate. Errors name the yaml key.
pub(crate) fn validate_domains(domains: &Domains, layers: &Layers) -> Result<()> {
    let known = |id: &str| domains.domains.iter().any(|d| d.id == id);
    let check_domain = |key: String, id: &str| -> Result<()> {
        if known(id) {
            return Ok(());
        }
        bail!("domains.v1.yaml {key}: domain `{id}` is not in `domains`")
    };
    let check_crate = |key: String, name: &str| -> Result<()> {
        if layers.contains_key(name) {
            return Ok(());
        }
        bail!(
            "domains.v1.yaml {key}: `{name}` is not a crate in docs/src/architecture/layers.toml [crates]"
        )
    };
    let mut seen = BTreeSet::new();
    for d in &domains.domains {
        if !seen.insert(d.id.as_str()) {
            bail!("domains.v1.yaml `domains`: duplicate domain id `{}`", d.id);
        }
    }
    // The generator assigns these ids without a lookup table (fallback, GUI surfaces, skill and
    // doc rows); a vocabulary without them would emit rows `vox features` can never list.
    if !domains.domains.is_empty() {
        for id in REQUIRED_DOMAINS {
            if !known(id) {
                bail!("domains.v1.yaml `domains` must contain the domain `{id}`");
            }
        }
    }
    for (group, entry) in &domains.cli_groups {
        check_domain(format!("cli_groups.{group}"), &entry.domain)?;
        if let Some(owner) = &entry.owner {
            check_crate(format!("cli_groups.{group}.owner"), owner)?;
        }
    }
    for (prefix, id) in &domains.mcp_prefixes {
        check_domain(format!("mcp_prefixes.{prefix}"), id)?;
    }
    for (name, id) in &domains.crates {
        check_crate(format!("crates.{name}"), name)?;
        check_domain(format!("crates.{name}"), id)?;
    }
    Ok(())
}

/// First token of an MCP tool name after `vox_` (`vox_mesh_nodes` -> `mesh`).
fn mcp_prefix(name: &str) -> Option<&str> {
    name.strip_prefix("vox_")
        .unwrap_or(name)
        .split('_')
        .next()
        .filter(|p| !p.is_empty())
}

/// The curated domain of one cli/mcp row (D-09): catalog override, group / prefix table, owner
/// crate table (never for a surface crate), `platform`.
pub(crate) fn resolve_domain(
    kind: RowKind,
    op: &OperationRow,
    owner: Option<&OwnerRef>,
    domains: &Domains,
) -> Result<String> {
    if let Some(id) = &op.domain {
        if !domains.domains.iter().any(|d| &d.id == id) {
            bail!(
                "catalog op `{}`: domain `{id}` is not in contracts/capabilities/domains.v1.yaml `domains`",
                op.id
            );
        }
        return Ok(id.clone());
    }
    let cli_group = op
        .cli
        .as_ref()
        .and_then(|c| c.path.first())
        .and_then(|g| domains.cli_groups.get(g))
        .map(|e| e.domain.clone());
    let from_table = if kind == RowKind::McpTool {
        op.mcp
            .as_ref()
            .and_then(|m| mcp_prefix(&m.name))
            .and_then(|p| domains.mcp_prefixes.get(p))
            .cloned()
            .or(cli_group)
    } else {
        cli_group
    };
    if let Some(id) = from_table {
        return Ok(id);
    }
    let from_crate = owner
        .filter(|o| !SURFACE_CRATES.contains(&o.crate_name.as_str()))
        .and_then(|o| domains.crates.get(&o.crate_name));
    Ok(from_crate
        .cloned()
        .unwrap_or_else(|| FALLBACK_DOMAIN.to_string()))
}

#[cfg(test)]
mod tests {
    use super::super::inputs::{GroupEntry, test_op};
    use super::*;
    use vox_capability_registry::DomainDef;

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

    fn domains() -> Domains {
        let def = |id: &str| DomainDef {
            id: id.to_string(),
            label: id.to_string(),
        };
        let entry = |domain: &str, owner: Option<&str>| GroupEntry {
            domain: domain.to_string(),
            owner: owner.map(str::to_string),
        };
        Domains {
            schema_version: 1,
            domains: [
                "secrets", "mesh", "compiler", "platform", "gui", "skills", "docs",
            ]
            .map(def)
            .to_vec(),
            cli_groups: BTreeMap::from([
                ("secrets".to_string(), entry("secrets", None)),
                ("ars".to_string(), entry("compiler", Some("vox-cli"))),
            ]),
            mcp_prefixes: BTreeMap::from([("mesh".to_string(), "mesh".to_string())]),
            crates: BTreeMap::from([
                ("vox-cli".to_string(), "compiler".to_string()),
                ("vox-db".to_string(), "compiler".to_string()),
            ]),
        }
    }

    fn owner(name: &str) -> OwnerRef {
        OwnerRef {
            crate_name: name.to_string(),
            layer: Some(5),
            rule: "x".to_string(),
        }
    }

    #[test]
    fn domain_override_wins() {
        let mut op = cli_op(&["zz", "x"]);
        op.domain = Some("secrets".into());
        let d = resolve_domain(RowKind::Cli, &op, None, &domains()).expect("ok");
        assert_eq!(d, "secrets");
    }

    #[test]
    fn unknown_domain_override_is_error() {
        let mut op = cli_op(&["secrets", "get"]);
        op.domain = Some("nope".into());
        let err = resolve_domain(RowKind::Cli, &op, None, &domains()).expect_err("must fail");
        assert!(err.to_string().contains("secrets.get"), "got: {err}");
    }

    #[test]
    fn cli_group_table_domain() {
        let op = cli_op(&["secrets", "get"]);
        let d = resolve_domain(RowKind::Cli, &op, None, &domains()).expect("ok");
        assert_eq!(d, "secrets");
    }

    #[test]
    fn mcp_prefix_domain() {
        let op = test_op("mesh.nodes", None, Some("vox_mesh_nodes"), "d");
        let d = resolve_domain(RowKind::McpTool, &op, None, &domains()).expect("ok");
        assert_eq!(d, "mesh");
    }

    #[test]
    fn paired_mcp_row_falls_back_to_cli_group() {
        let op = test_op(
            "secrets.get",
            Some((&["secrets", "get"], "active")),
            Some("vox_zzz_get"),
            "d",
        );
        let d = resolve_domain(RowKind::McpTool, &op, None, &domains()).expect("ok");
        assert_eq!(d, "secrets");
    }

    #[test]
    fn surface_crate_never_gives_domain() {
        let op = cli_op(&["zz"]);
        let d = resolve_domain(RowKind::Cli, &op, Some(&owner("vox-cli")), &domains()).expect("ok");
        assert_eq!(d, "platform", "vox-cli is a surface crate");
        let d = resolve_domain(RowKind::Cli, &op, Some(&owner("vox-db")), &domains()).expect("ok");
        assert_eq!(d, "compiler", "a domain crate does give its domain");
    }

    #[test]
    fn table_value_outside_vocabulary_is_error() {
        let mut d = domains();
        d.cli_groups.insert(
            "zz".to_string(),
            GroupEntry {
                domain: "bogus".to_string(),
                owner: None,
            },
        );
        let l = layers(&[("vox-cli", 5), ("vox-db", 2)]);
        let err = validate_domains(&d, &l).expect_err("must fail");
        assert!(err.to_string().contains("cli_groups.zz"), "got: {err}");
        assert!(validate_domains(&domains(), &l).is_ok());
    }

    #[test]
    fn generator_assigned_domains_must_be_in_the_vocabulary() {
        let l = layers(&[("vox-cli", 5), ("vox-db", 2)]);
        for missing in REQUIRED_DOMAINS {
            let mut d = domains();
            d.domains.retain(|x| x.id != missing);
            let err = validate_domains(&d, &l).expect_err("missing required domain must fail");
            assert!(err.to_string().contains(missing), "{missing}: {err}");
        }
    }

    #[test]
    fn duplicate_domain_ids_are_rejected() {
        let mut d = domains();
        d.domains.push(DomainDef {
            id: "mesh".to_string(),
            label: "Mesh again".to_string(),
        });
        let l = layers(&[("vox-cli", 5), ("vox-db", 2)]);
        let err = validate_domains(&d, &l).expect_err("duplicate id must fail");
        assert!(
            err.to_string().contains("duplicate") && err.to_string().contains("mesh"),
            "{err}"
        );
    }

    #[test]
    fn owner_and_crate_keys_must_be_layer_crates() {
        let mut d = domains();
        d.cli_groups.insert(
            "zz".to_string(),
            GroupEntry {
                domain: "mesh".to_string(),
                owner: Some("vox-nope".to_string()),
            },
        );
        let l = layers(&[("vox-cli", 5), ("vox-db", 2)]);
        let err = validate_domains(&d, &l).expect_err("unknown owner must fail");
        assert!(err.to_string().contains("vox-nope"), "got: {err}");
        let l = layers(&[("vox-cli", 5)]);
        let err = validate_domains(&domains(), &l).expect_err("unknown crates key must fail");
        assert!(err.to_string().contains("vox-db"), "got: {err}");
    }

    #[test]
    fn group_owners_extracts_owner_entries() {
        let owners = group_owners(&domains());
        assert_eq!(owners.len(), 1);
        assert_eq!(owners.get("ars").map(String::as_str), Some("vox-cli"));
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
