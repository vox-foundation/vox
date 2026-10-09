//! Pure join: committed inputs -> [`CapabilityIndex`]. No IO, no clock.

use std::collections::BTreeSet;

use anyhow::{Result, anyhow, bail};
use vox_capability_registry::{
    CapabilityIndex, CliRef, CoverageRef, DocLink, IndexRow, McpRef, RowKind,
    implicit_cli_capability_id, implicit_mcp_capability_id,
};

use super::inputs::{Inputs, OperationRow};

/// Default page-level docs link per row kind (D-06: the repo has no per-row anchors yet).
const CLI_DOC: &str = "docs/src/reference/cli-command-surface.generated.md";
const MCP_DOC: &str = "docs/src/reference/mcp-tool-registry-contract.md";

/// One-line, control-free form of a catalog description (also blocks markup/newline injection into
/// downstream one-line emitters such as llms.txt). `None` when nothing printable remains.
fn single_line(text: &str) -> Option<String> {
    let kept: String = text
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
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
        coverage: Some(CoverageRef {
            kind: None,
            basis: "none".to_string(),
            ambiguous: false,
            reason: Some("no graph-coverage snapshot committed".to_string()),
        }),
    }
}

pub(crate) fn build_index(inputs: &Inputs) -> Result<CapabilityIndex> {
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
            row.cli = Some(CliRef {
                path: c.path.clone(),
                invocation: format!("vox {}", c.path.join(" ")),
                feature_gate: c.feature_gate.clone(),
            });
            row.equivalents = mcp_id.clone().into_iter().collect();
            rows.push(row);
        }
        if let (Some(m), Some(id)) = (&op.mcp, &mcp_id) {
            let mut row = base_row(op, id.clone(), RowKind::McpTool, MCP_DOC);
            row.mcp = Some(McpRef {
                name: m.name.clone(),
            });
            row.equivalents = cli_id.clone().into_iter().collect();
            rows.push(row);
        }
    }
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
        coverage_snapshot: None,
        domains: Vec::new(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::super::inputs::{OperationRow, test_op};
    use super::*;
    use vox_capability_registry::RowKind;

    fn inputs(ops: Vec<OperationRow>) -> Inputs {
        Inputs { ops }
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
