//! Pure join: committed inputs -> [`CapabilityIndex`]. No IO, no clock.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow, bail};
use vox_capability_registry::{
    CapabilityIndex, CliRef, CoverageRef, DocLink, IndexRow, McpRef, RowKind,
    implicit_cli_capability_id, implicit_mcp_capability_id,
};

use super::coverage;
use super::inputs::{Inputs, OperationRow};
use super::nodes;
use super::owner::{
    group_owners, resolve_domain, resolve_owner, source_path_for, validate_domains,
};
use crate::command_catalog::{CatalogTier, tier_for_path};

/// Default page-level docs link per row kind (D-06: the repo has no per-row anchors yet).
pub(super) const CLI_DOC: &str = "docs/src/reference/cli-command-surface.generated.md";
pub(super) const MCP_DOC: &str = "docs/src/reference/mcp-tool-registry-contract.md";

/// Invisible Unicode format characters (category Cf) that `char::is_control` (Cc only) misses:
/// zero-width, bidi marks/overrides/isolates, word joiner, BOM and the U+E0000 tag block, the
/// standard carriers of hidden text in LLM-facing files.
fn is_invisible_format(c: char) -> bool {
    matches!(
        c as u32,
        0x00AD
            | 0x061C
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x2066..=0x2069
            | 0xFEFF
            | 0xE0000..=0xE007F
    )
}

/// One-line, control-free form of a catalog description (also blocks markup/newline/invisible-text
/// injection into downstream one-line emitters such as llms.txt). `None` when nothing printable
/// remains.
pub(super) fn single_line(text: &str) -> Option<String> {
    let kept: String = text
        .chars()
        .filter(|c| c.is_whitespace() || !(c.is_control() || is_invisible_format(*c)))
        .collect();
    let line = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    (!line.is_empty()).then_some(line)
}

/// Fields common to every row built from a catalog op; kind-specific fields are set by the caller.
fn base_row(op: &OperationRow, id: String, kind: RowKind, doc: &str) -> IndexRow {
    IndexRow {
        id,
        kind,
        title: op.title.clone(),
        summary: single_line(op.description_human.as_deref().unwrap_or(&op.description)),
        op_id: Some(op.id.clone()),
        status: None,
        domain: None,
        product_lane: Some(op.product_lane.clone()),
        side_effect_class: op.side_effect_class.clone(),
        tier: None,
        cli: None,
        mcp: None,
        gui: None,
        owner: None,
        source_path: None,
        docs: vec![DocLink {
            path: doc.to_string(),
            origin: "default".to_string(),
        }],
        skills: Vec::new(),
        equivalents: Vec::new(),
        refs: Vec::new(),
        coverage: Some(no_snapshot_coverage()),
    }
}

/// Coverage of a row before (or without) a committed snapshot: unknown, with the reason.
pub(super) fn no_snapshot_coverage() -> CoverageRef {
    CoverageRef {
        kind: None,
        basis: "none".to_string(),
        ambiguous: false,
        reason: Some("no graph-coverage snapshot committed".to_string()),
    }
}

/// Fill the Location and grouping fields (owner crate + rule + layer, source path, domain) of a
/// cli/mcp row.
fn set_placement(
    row: &mut IndexRow,
    kind: RowKind,
    op: &OperationRow,
    inputs: &Inputs,
    group_owners: &BTreeMap<String, String>,
) -> Result<()> {
    let owner = resolve_owner(
        kind,
        op,
        &inputs.layers,
        group_owners,
        &inputs.cli_command_files,
    )?;
    row.source_path = source_path_for(kind, op, owner.as_ref(), &inputs.cli_command_files);
    row.domain = Some(resolve_domain(kind, op, owner.as_ref(), &inputs.domains)?);
    row.owner = owner;
    Ok(())
}

pub(crate) fn build_index(inputs: &Inputs) -> Result<CapabilityIndex> {
    validate_domains(&inputs.domains, &inputs.layers)?;
    let group_owners = group_owners(&inputs.domains);
    let mut rows: Vec<IndexRow> = Vec::new();
    for op in &inputs.ops {
        if op.cli.is_none() && op.mcp.is_none() {
            bail!(
                "catalog op `{}` has neither a cli nor an mcp projection",
                op.id
            );
        }
        let cli_id = op.cli.as_ref().map(|c| implicit_cli_capability_id(&c.path));
        let mcp_id = op.mcp.as_ref().map(|m| implicit_mcp_capability_id(&m.name));
        if let (Some(c), Some(id)) = (&op.cli, &cli_id) {
            let mut row = base_row(op, id.clone(), RowKind::Cli, CLI_DOC);
            row.status = Some(c.status.clone());
            row.tier = Some(
                match tier_for_path(&c.path, c.feature_gate.is_some()) {
                    CatalogTier::Recommended => "recommended",
                    CatalogTier::Advanced => "advanced",
                    CatalogTier::FeatureGated => "feature_gated",
                }
                .to_string(),
            );
            row.cli = Some(CliRef {
                path: c.path.clone(),
                invocation: format!("vox {}", c.path.join(" ")),
                feature_gate: c.feature_gate.clone(),
            });
            row.equivalents = mcp_id.clone().into_iter().collect();
            set_placement(&mut row, RowKind::Cli, op, inputs, &group_owners)?;
            rows.push(row);
        }
        if let (Some(m), Some(id)) = (&op.mcp, &mcp_id) {
            let mut row = base_row(op, id.clone(), RowKind::McpTool, MCP_DOC);
            row.mcp = Some(McpRef {
                name: m.name.clone(),
            });
            row.equivalents = cli_id.clone().into_iter().collect();
            set_placement(&mut row, RowKind::McpTool, op, inputs, &group_owners)?;
            rows.push(row);
        }
    }
    nodes::apply_surface_join(&mut rows, &inputs.surfaces);
    let extra = nodes::non_operation_rows(&mut rows, inputs)?;
    rows.extend(extra);
    let docs = nodes::apply_doc_claims(&mut rows, inputs)?;
    rows.extend(docs);
    coverage::apply_coverage(&mut rows, inputs.coverage.as_ref());
    let mut seen = BTreeSet::new();
    for row in &rows {
        if !seen.insert(row.id.as_str()) {
            return Err(anyhow!("duplicate capability index id `{}`", row.id));
        }
    }
    rows.sort_by(|a, b| (a.kind, &a.id).cmp(&(b.kind, &b.id)));
    Ok(CapabilityIndex {
        schema_version: 1,
        generator: "vox ci capability-index".to_string(),
        coverage_snapshot: inputs.coverage.as_ref().map(|s| s.source.clone()),
        domains: inputs.domains.domains.clone(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::super::inputs::{OperationRow, test_op};
    use super::*;
    use vox_capability_registry::RowKind;

    fn inputs(ops: Vec<OperationRow>) -> Inputs {
        Inputs {
            ops,
            ..Default::default()
        }
    }

    fn ids(idx: &CapabilityIndex) -> Vec<&str> {
        idx.rows.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn builds_one_row_per_projection() {
        let idx = build_index(&inputs(vec![
            test_op("a.b", Some((&["a", "b"], "active")), None, "cli only"),
            test_op("x", None, Some("vox_x"), "mcp only"),
            test_op("p", Some((&["p"], "active")), Some("vox_p"), "paired"),
        ]))
        .expect("build");
        assert_eq!(
            ids(&idx),
            vec!["cli.a.b", "cli.p", "mcp.vox_p", "mcp.vox_x"]
        );
        let cli_p = idx.row("cli.p").expect("cli.p");
        assert_eq!(cli_p.kind, RowKind::Cli);
        assert_eq!(cli_p.equivalents, vec!["mcp.vox_p"]);
        assert_eq!(
            idx.row("mcp.vox_p").expect("mcp.vox_p").equivalents,
            vec!["cli.p"]
        );
        assert!(idx.row("cli.a.b").expect("cli.a.b").equivalents.is_empty());
        assert!(
            idx.row("mcp.vox_x")
                .expect("mcp.vox_x")
                .equivalents
                .is_empty()
        );
        let a = idx.row("cli.a.b").expect("cli.a.b");
        assert_eq!(a.cli.as_ref().expect("cli ref").invocation, "vox a b");
        let cov = a.coverage.as_ref().expect("coverage object");
        assert!(cov.kind.is_none());
        assert!(cov.reason.as_deref().is_some_and(|r| !r.is_empty()));
    }

    #[test]
    fn rows_carry_owner_and_source_path() {
        let mut inp = inputs(vec![test_op(
            "secrets.get",
            Some((&["secrets", "get"], "active")),
            Some("vox_secrets_get"),
            "d",
        )]);
        inp.layers = [("vox-secrets", 1u8), ("vox-orchestrator-mcp", 4)]
            .into_iter()
            .map(|(n, l)| (n.to_string(), Some(l)))
            .collect();
        let idx = build_index(&inp).expect("build");
        let cli = idx.row("cli.secrets.get").expect("cli row");
        let owner = cli.owner.as_ref().expect("cli owner");
        assert_eq!(
            (owner.crate_name.as_str(), owner.rule.as_str(), owner.layer),
            ("vox-secrets", "crate_name_match", Some(1))
        );
        assert_eq!(cli.source_path.as_deref(), Some("crates/vox-secrets/"));
        let mcp = idx.row("mcp.vox_secrets_get").expect("mcp row");
        assert_eq!(mcp.owner.as_ref().expect("mcp owner").rule, "mcp_dispatch");
        assert_eq!(
            mcp.source_path.as_deref(),
            Some("crates/vox-orchestrator-mcp/src/dispatch.rs")
        );
    }

    #[test]
    fn tier_matches_catalog_rule() {
        let mut gated = test_op("g", Some((&["zz", "x"], "active")), None, "d");
        gated.cli.as_mut().expect("cli").feature_gate = Some("extras".into());
        let idx = build_index(&inputs(vec![
            test_op(
                "check",
                Some((&["check"], "active")),
                Some("vox_check"),
                "d",
            ),
            test_op("ci.x", Some((&["ci", "x"], "active")), None, "d"),
            gated,
        ]))
        .expect("build");
        let tier = |id: &str| idx.row(id).expect("row").tier.clone();
        assert_eq!(tier("cli.check").as_deref(), Some("recommended"));
        assert_eq!(tier("cli.ci.x").as_deref(), Some("advanced"));
        assert_eq!(tier("cli.zz.x").as_deref(), Some("feature_gated"));
        assert_eq!(tier("mcp.vox_check"), None, "mcp rows carry no cli tier");
    }

    #[test]
    fn rows_carry_domain_and_index_lists_vocabulary() {
        use super::super::inputs::{Domains, GroupEntry};
        use vox_capability_registry::DomainDef;
        let def = |id: &str| DomainDef {
            id: id.to_string(),
            label: format!("L {id}"),
        };
        let mut inp = inputs(vec![
            test_op("s.get", Some((&["secrets", "get"], "active")), None, "d"),
            test_op("m", None, Some("vox_zz_m"), "d"),
        ]);
        inp.domains = Domains {
            schema_version: 1,
            domains: vec![def("secrets"), def("platform")],
            cli_groups: [(
                "secrets".to_string(),
                GroupEntry {
                    domain: "secrets".to_string(),
                    owner: None,
                },
            )]
            .into(),
            ..Default::default()
        };
        let idx = build_index(&inp).expect("build");
        assert_eq!(
            idx.row("cli.secrets.get").expect("cli").domain.as_deref(),
            Some("secrets")
        );
        assert_eq!(
            idx.row("mcp.vox_zz_m").expect("mcp").domain.as_deref(),
            Some("platform")
        );
        let vocab: Vec<&str> = idx.domains.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(vocab, vec!["secrets", "platform"], "file order");
        assert!(
            idx.row("cli.secrets.get")
                .expect("cli")
                .product_lane
                .is_some(),
            "product_lane stays as a secondary facet"
        );
    }

    #[test]
    fn invalid_domain_table_fails_generation() {
        use super::super::inputs::{Domains, GroupEntry};
        let mut inp = inputs(vec![test_op("a", Some((&["a"], "active")), None, "d")]);
        inp.domains = Domains {
            cli_groups: [(
                "a".to_string(),
                GroupEntry {
                    domain: "bogus".to_string(),
                    owner: None,
                },
            )]
            .into(),
            ..Default::default()
        };
        let err = build_index(&inp).expect_err("bogus domain must fail");
        assert!(err.to_string().contains("cli_groups.a"), "got: {err}");
    }

    #[test]
    fn non_active_cli_row_keeps_status() {
        let idx = build_index(&inputs(vec![test_op(
            "old",
            Some((&["old"], "deprecated")),
            None,
            "d",
        )]))
        .expect("build");
        assert_eq!(idx.rows.len(), 1);
        assert_eq!(idx.rows[0].status.as_deref(), Some("deprecated"));
    }

    #[test]
    fn duplicate_id_is_an_error() {
        let err = build_index(&inputs(vec![
            test_op("one", Some((&["dup", "x"], "active")), None, "d"),
            test_op("two", Some((&["dup", "x"], "active")), None, "d"),
        ]))
        .expect_err("duplicate ids must fail");
        assert!(err.to_string().contains("cli.dup.x"), "got: {err}");
    }

    #[test]
    fn deterministic_and_sorted() {
        let make = || {
            inputs(vec![
                test_op("z", Some((&["z"], "active")), Some("vox_z"), "d"),
                test_op("a", Some((&["a"], "active")), Some("vox_a"), "d"),
            ])
        };
        let one =
            serde_json::to_string_pretty(&build_index(&make()).expect("build")).expect("json");
        let two =
            serde_json::to_string_pretty(&build_index(&make()).expect("build")).expect("json");
        assert_eq!(one, two);
        let idx = build_index(&make()).expect("build");
        assert_eq!(ids(&idx), vec!["cli.a", "cli.z", "mcp.vox_a", "mcp.vox_z"]);
    }

    #[test]
    fn summary_is_one_line() {
        let idx = build_index(&inputs(vec![test_op(
            "s",
            Some((&["s"], "active")),
            None,
            "line one\nline\ttwo\r\nthree \u{7}bell",
        )]))
        .expect("build");
        assert_eq!(idx.rows.len(), 1);
        let summary = idx.rows[0].summary.as_deref().expect("summary");
        assert!(!summary.chars().any(char::is_control), "got: {summary:?}");
        assert_eq!(summary, "line one line two three bell");
    }
}
